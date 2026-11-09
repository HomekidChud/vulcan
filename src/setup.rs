// `vulcan setup <file>` — convert an app artifact (APK, IPA, DMG, EXE) into a
// Vulcan project. Pure std Rust: zip/APK/IPA, Android binary XML + DEX,
// Apple Info.plist, UDIF DMG, PE/COFF with imports, and Mach-O detection.
//
// The converter extracts everything structurally available from the artifact
// (identity, entry points, components, classes, imports, resources) and
// generates a runnable Vulcan project scaffold plus a full conversion report.
// Compiled machine code is not decompiled — the scaffold marks where the
// original logic goes, and the report maps every component it found.

use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------- zip ----

struct ZipEntry {
    name: String,
    size: u64,
    compressed: u64,
    stored: bool,
    header_off: u64,
}

fn u16le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }
fn u64le(b: &[u8], o: usize) -> u64 {
    let mut v = [0u8; 8];
    v.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(v)
}

/// Parse a zip's central directory (works for APK/IPA/jar/zip-based DMGs).
fn zip_entries(data: &[u8]) -> Vec<ZipEntry> {
    let mut out = Vec::new();
    if data.len() < 22 || &data[0..4] != b"PK\x03\x04" { return out; }
    // find the End Of Central Directory record (scan from the tail)
    let mut eocd = None;
    let start = data.len().saturating_sub(66_000);
    for i in (start..data.len() - 4).rev() {
        if &data[i..i + 4] == b"PK\x05\x06" {
            eocd = Some(i);
            break;
        }
    }
    let Some(eocd) = eocd else { return out };
    let count = u16le(data, eocd + 10) as usize;
    let cd_off = u32le(data, eocd + 16) as usize;
    let mut off = cd_off;
    for _ in 0..count {
        if off + 46 > data.len() || &data[off..off + 4] != b"PK\x01\x02" { break; }
        let method = u16le(data, off + 10);
        let csize = u32le(data, off + 20) as u64;
        let usize_ = u32le(data, off + 24) as u64;
        let nlen = u16le(data, off + 28) as usize;
        let elen = u16le(data, off + 30) as usize;
        let clen = u16le(data, off + 32) as usize;
        let header_off = u32le(data, off + 42) as u64;
        let name = String::from_utf8_lossy(&data[off + 46..off + 46 + nlen]).into_owned();
        out.push(ZipEntry {
            name,
            size: usize_,
            compressed: csize,
            stored: method == 0,
            header_off,
        });
        off += 46 + nlen + elen + clen;
    }
    out
}

/// Extract a stored (uncompressed) entry. Deflated entries are listed but not
/// extracted (no inflate in std; the report lists them for later conversion).
fn zip_extract_stored(data: &[u8], e: &ZipEntry) -> Option<Vec<u8>> {
    let off = e.header_off as usize;
    if off + 30 > data.len() || &data[off..off + 4] != b"PK\x03\x04" { return None; }
    let nlen = u16le(data, off + 26) as usize;
    let elen = u16le(data, off + 28) as usize;
    let start = off + 30 + nlen + elen;
    if e.compressed as usize + start > data.len() { return None; }
    Some(data[start..start + e.compressed as usize].to_vec())
}

// -------------------------------------------------- Android binary XML ----

/// A parsed AXML document: enough to read package/activity/permission names.
struct Axml {
    strings: Vec<String>,
    /// (element name, attributes [(name, value)]) in document order
    elements: Vec<(String, Vec<(String, String)>)>,
}

fn read_axml_string_pool(b: &[u8], base: usize) -> Vec<String> {
    let mut strings = Vec::new();
    if base + 28 > b.len() { return strings; }
    let count = u32le(b, base + 8) as usize;
    let flags = u32le(b, base + 16);
    let utf8 = flags & 0x100 != 0;
    let strings_start = u32le(b, base + 20) as usize;
    for i in 0..count {
        let off_pos = base + 28 + i * 4;
        if off_pos + 4 > b.len() { break; }
        let so = base + strings_start + u32le(b, off_pos) as usize;
        if so >= b.len() { strings.push(String::new()); continue; }
        let s = if utf8 {
            // u16 char count, u16/u8 byte count, then bytes
            let mut p = so;
            let mut skip = |p: &mut usize| {
                let mut n = b[*p] as usize; *p += 1;
                if n & 0x80 != 0 { n = ((n & 0x7f) << 8) | b[*p] as usize; *p += 1; }
                n
            };
            skip(&mut p); // chars
            let blen = skip(&mut p);
            let end = (p + blen).min(b.len());
            String::from_utf8_lossy(&b[p..end]).into_owned()
        } else {
            let mut p = so;
            let mut n = u16le(b, p) as usize; p += 2;
            if n & 0x8000 != 0 {
                n = ((n & 0x7fff) << 16) | u16le(b, p) as usize;
                p += 2;
            }
            let end = (p + n * 2).min(b.len());
            let mut s = String::new();
            let mut i = p;
            while i + 1 < end {
                s.push(char::from_u32(u16::from_le_bytes([b[i], b[i + 1]]) as u32).unwrap_or('\u{fffd}'));
                i += 2;
            }
            s
        };
        strings.push(s);
    }
    strings
}

fn axml_str(strings: &[String], idx: i64) -> String {
    if idx < 0 || idx as usize >= strings.len() { String::new() }
    else { strings[idx as usize].clone() }
}

fn parse_axml(data: &[u8]) -> Option<Axml> {
    if data.len() < 8 || u16le(data, 4) as usize != data.len() { /* still try */ }
    let mut strings = Vec::new();
    let mut elements = Vec::new();
    let mut off = 0usize;
    while off + 8 <= data.len() {
        let ctype = u16le(data, off);
        let size = u32le(data, off + 4) as usize;
        if size == 0 || off + size > data.len() { break; }
        match ctype {
            0x0001 => strings = read_axml_string_pool(data, off),
            0x0102 => {
                // START_ELEMENT
                if off + 36 > data.len() { break; }
                let name_idx = u32le(data, off + 20) as i64;
                let attr_start = u16le(data, off + 24) as usize;
                let attr_count = u16le(data, off + 28) as usize;
                let mut attrs = Vec::new();
                for i in 0..attr_count {
                    let a = off + attr_start + i * 20;
                    if a + 20 > data.len() { break; }
                    let aname = u32le(data, a + 4) as i64;
                    let raw = u32le(data, a + 8) as i64;
                    let dtype = data[a + 15];
                    let ddata = u32le(data, a + 16);
                    let value = if raw >= 0 && (raw as usize) < strings.len() && dtype != 0x10 {
                        strings.get(raw as usize).cloned().unwrap_or_default()
                    } else if dtype == 0x03 && (ddata as usize) < strings.len() {
                        strings[ddata as usize].clone()
                    } else if dtype == 0x10 {
                        ddata.to_string() // INT
                    } else if dtype == 0x12 {
                        if ddata != 0 { "true".into() } else { "false".into() }
                    } else if dtype == 0x01 {
                        format!("0x{ddata:08x}") // REF
                    } else {
                        String::new()
                    };
                    attrs.push((axml_str(&strings, aname), value));
                }
                elements.push((axml_str(&strings, name_idx), attrs));
            }
            _ => {}
        }
        off += size;
    }
    if strings.is_empty() && elements.is_empty() { None } else { Some(Axml { strings, elements }) }
}

// ------------------------------------------------------------- DEX ----

fn uleb128(b: &[u8], p: &mut usize) -> u64 {
    let mut result = 0u64;
    let mut shift = 0;
    loop {
        if *p >= b.len() { return result; }
        let byte = b[*p]; *p += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 { break; }
        shift += 7;
    }
    result
}

struct DexInfo {
    strings: usize,
    methods: usize,
    fields: usize,
    classes: Vec<String>,
}

fn parse_dex(data: &[u8]) -> Option<DexInfo> {
    if data.len() < 112 || &data[0..4] != b"dex\n" { return None; }
    let string_ids_size = u32le(data, 52) as usize;
    let string_ids_off = u32le(data, 56) as usize;
    let method_ids_size = u32le(data, 84) as usize;
    let field_ids_size = u32le(data, 76) as usize;
    let type_ids_size = u32le(data, 60) as usize;
    let type_ids_off = u32le(data, 64) as usize;
    let class_defs_size = u32le(data, 92) as usize;
    let class_defs_off = u32le(data, 96) as usize;
    let type_string = |tidx: usize| -> Option<String> {
        if tidx >= type_ids_size { return None; }
        // type_ids[tidx] is a string INDEX; string_ids[idx] is a file offset
        let sidx = u32le(data, type_ids_off + tidx * 4) as usize;
        if sidx >= string_ids_size { return None; }
        let so = u32le(data, string_ids_off + sidx * 4) as usize;
        if so >= data.len() { return None; }
        let mut p = so;
        let _len = uleb128(data, &mut p);
        let mut end = p;
        while end < data.len() && data[end] != 0 { end += 1; }
        Some(String::from_utf8_lossy(&data[p..end]).into_owned())
    };
    let mut classes = Vec::new();
    for i in 0..class_defs_size.min(500) {
        let cd = class_defs_off + i * 32;
        if cd + 4 > data.len() { break; }
        let tidx = u32le(data, cd) as usize;
        if let Some(s) = type_string(tidx) { classes.push(s); }
    }
    Some(DexInfo {
        strings: string_ids_size,
        methods: method_ids_size,
        fields: field_ids_size,
        classes,
    })
}

fn dex_class_name(desc: &str) -> String {
    desc.trim_start_matches('L').trim_end_matches(';').replace('/', ".")
}

// -------------------------------------------------------- Apple plist ----

/// Best-effort plist reader: handles XML plists (real key/value parse) and
/// binary plists (marker walk for the common keys we need).
fn plist_field(data: &[u8], key: &str) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    if text.contains("<plist") || text.contains("<?xml") {
        // XML plist: <key>K</key><string>V</string>
        let k = format!("<key>{}</key>", key);
        if let Some(p) = text.find(&k) {
            let rest = &text[p + k.len()..];
            for tag in ["string", "integer", "true", "false"] {
                let open = format!("<{}>", tag);
                if let Some(s) = rest.find(&open) {
                    let seg = &rest[s + open.len()..];
                    if let Some(e) = seg.find("</") {
                        return Some(seg[..e].trim().to_string());
                    }
                }
            }
        }
        return None;
    }
    if data.len() > 8 && &data[0..8] == b"bplist00" {
        // binary plist: scan object table for the key string, take the next
        // ASCII string marker after it
        let needle = key.as_bytes();
        for i in 0..data.len().saturating_sub(needle.len() + 16) {
            if &data[i..i + needle.len()] == needle {
                // walk forward for a string marker 0x5n (ASCII) or 0x6n (UTF16)
                for j in i + needle.len()..(i + needle.len() + 64).min(data.len()) {
                    let m = data[j];
                    if m & 0xf0 == 0x50 {
                        let len = (m & 0x0f) as usize;
                        let end = (j + 1 + len).min(data.len());
                        return Some(String::from_utf8_lossy(&data[j + 1..end]).into_owned());
                    }
                    if m & 0xf0 == 0x60 {
                        let len = (m & 0x0f) as usize * 2;
                        let end = (j + 1 + len).min(data.len());
                        let mut s = String::new();
                        let mut k2 = j + 1;
                        while k2 + 1 < end {
                            s.push(char::from_u32(u16::from_be_bytes([data[k2], data[k2 + 1]]) as u32).unwrap_or('\u{fffd}'));
                            k2 += 2;
                        }
                        return Some(s);
                    }
                }
            }
        }
        return None;
    }
    None
}

// -------------------------------------------------------------- PE ----

struct PeInfo {
    machine: String,
    subsystem: String,
    is_dotnet: bool,
    entry_rva: u32,
    sections: Vec<(String, u64, u64)>,
    imports: Vec<(String, Vec<String>)>,
}

fn parse_pe(data: &[u8]) -> Option<PeInfo> {
    if data.len() < 0x40 || &data[0..2] != b"MZ" { return None; }
    let pe_off = u32le(data, 0x3c) as usize;
    if pe_off + 24 > data.len() || &data[pe_off..pe_off + 4] != b"PE\0\0" { return None; }
    let coff = pe_off + 4;
    let machine = match u16le(data, coff) {
        0x14c => "i386", 0x8664 => "x86-64", 0xaa64 => "ARM64",
        0x1c0 => "ARM Thumb", 0x1c4 => "ARM Thumb-2", _ => "unknown",
    };
    let nsec = u16le(data, coff + 2) as usize;
    let opt_size = u16le(data, coff + 16) as usize;
    let opt = coff + 20;
    let (magic, dd_base) = match u16le(data, opt) {
        0x20b => (0x20b, opt + 112),
        _ => (0x10b, opt + 96),
    };
    let entry_rva = u32le(data, opt + 16);
    let subsystem = match u16le(data, opt + 68) {
        2 => "Windows GUI", 3 => "Windows console", 1 => "native",
        9 => "Windows CE GUI", 14 => "EFI application", _ => "other",
    };
    // data directories: 16 entries, import = 1, CLR = 14
    let imp_rva = if dd_base + 8 * 2 <= data.len() { u32le(data, dd_base + 8) } else { 0 };
    let clr_rva = if dd_base + 8 * 15 <= data.len() { u32le(data, dd_base + 8 * 14) } else { 0 };
    let is_dotnet = clr_rva != 0;
    // sections
    let sec_base = opt + opt_size;
    let mut sections = Vec::new();
    for i in 0..nsec.min(64) {
        let s = sec_base + i * 40;
        if s + 40 > data.len() { break; }
        let name = String::from_utf8_lossy(&data[s..s + 8]).trim_end_matches('\0').to_string();
        let vsize = u32le(data, s + 8) as u64;
        let rsize = u32le(data, s + 16) as u64;
        sections.push((name, vsize, rsize));
    }
    let rva2off = |rva: u32| -> Option<usize> {
        for (i, (_, vsize, rsize)) in sections.iter().enumerate() {
            let s = sec_base + i * 40;
            let va = u32le(data, s + 12);
            if rva >= va && (rva as u64) < va as u64 + (*vsize).max(*rsize) {
                return Some((rva - va) as usize + u32le(data, s + 20) as usize);
            }
        }
        None
    };
    // imports
    let mut imports = Vec::new();
    if imp_rva != 0 {
        if let Some(mut d) = rva2off(imp_rva) {
            loop {
                if d + 20 > data.len() { break; }
                let name_rva = u32le(data, d + 12);
                let thunk_rva = u32le(data, d + 16);
                if name_rva == 0 && thunk_rva == 0 { break; }
                let dll = rva2off(name_rva).and_then(|o| {
                    let mut e = o;
                    while e < data.len() && data[e] != 0 { e += 1; }
                    if e > o { Some(String::from_utf8_lossy(&data[o..e]).into_owned()) } else { None }
                }).unwrap_or_default();
                let mut funcs = Vec::new();
                if let Some(t) = rva2off(thunk_rva) {
                    let mut p = t;
                    loop {
                        if p + 8 > data.len() { break; }
                        let (hint_rva, is_ord) = if magic == 0x20b {
                            let v = u64le(data, p); (v as u32, v & (1 << 63) != 0)
                        } else {
                            let v = u32le(data, p) as u64; (v as u32, v & (1 << 31) != 0)
                        };
                        if hint_rva == 0 && !is_ord { break; }
                        if !is_ord {
                            if let Some(h) = rva2off(hint_rva) {
                                if h + 2 < data.len() {
                                    let mut e = h + 2;
                                    while e < data.len() && data[e] != 0 { e += 1; }
                                    funcs.push(String::from_utf8_lossy(&data[h + 2..e]).into_owned());
                                }
                            }
                        } else {
                            funcs.push(format!("#{}", hint_rva));
                        }
                        p += if magic == 0x20b { 8 } else { 4 };
                        if funcs.len() > 4096 { break; }
                    }
                }
                imports.push((dll, funcs));
                d += 20;
            }
        }
    }
    Some(PeInfo { machine: machine.into(), subsystem: subsystem.into(), is_dotnet, entry_rva, sections, imports })
}

// ----------------------------------------------------------- Mach-O ----

fn macho_arch(data: &[u8]) -> Option<String> {
    if data.len() < 12 { return None; }
    let m = u32le(data, 0);
    let (cputype, be) = match m {
        0xFEEDFACF | 0xFEEDFACE => (u32le(data, 4), false),
        0xCFFAEDFE | 0xCEFAEDFE => (u32le(data, 4), true),
        _ => return None,
    };
    let ct = if be { cputype.swap_bytes() } else { cputype };
    let name = match ct {
        7 => "x86-64", 0x01000007 => "x86-64",
        12 => "ARM64", 0x0100000C => "ARM64",
        6 => "x86", 5 => "ARM", _ => "other",
    };
    Some(name.to_string())
}

// ---------------------------------------------------- the setup command ----

enum Artifact {
    Apk { entries: Vec<ZipEntry>, manifest: Option<Axml>, dex: Option<DexInfo> },
    Ipa { entries: Vec<ZipEntry>, bundle_id: Option<String>, bundle_name: Option<String>, version: Option<String> },
    Dmg { zip_entries: Option<Vec<ZipEntry>>, xml_len: u64, data_len: u64, version: u32 },
    Exe { pe: Option<PeInfo> },
    Macho { arch: String },
    Zip { entries: Vec<ZipEntry> },
}

fn detect(data: &[u8]) -> Artifact {
    if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" {
        let entries = zip_entries(data);
        let is_apk = entries.iter().any(|e| e.name == "AndroidManifest.xml");
        let is_ipa = entries.iter().any(|e| e.name.starts_with("Payload/") && e.name.contains(".app"));
        if is_apk {
            let manifest = find_entry(&entries, "AndroidManifest.xml")
                .and_then(|e| zip_extract_stored(data, e))
                .or_else(|| {
                    // some APKs deflate the manifest; still try the parser on raw bytes
                    find_entry(&entries, "AndroidManifest.xml").map(|_| Vec::new())
                })
                .and_then(|b| if b.is_empty() { None } else { parse_axml(&b) });
            let dex = find_entry(&entries, "classes.dex")
                .and_then(|e| zip_extract_stored(data, e))
                .and_then(|b| parse_dex(&b));
            return Artifact::Apk { entries, manifest, dex };
        }
        if is_ipa {
            let mut bundle_id = None;
            let mut bundle_name = None;
            let mut version = None;
            for e in &entries {
                if e.name.ends_with(".app/Info.plist") {
                    if let Some(b) = zip_extract_stored(data, e) {
                        bundle_id = plist_field(&b, "CFBundleIdentifier").or(bundle_id);
                        bundle_name = plist_field(&b, "CFBundleName").or(bundle_name);
                        version = plist_field(&b, "CFBundleShortVersionString").or(version);
                    }
                }
            }
            return Artifact::Ipa { entries, bundle_id, bundle_name, version };
        }
        return Artifact::Zip { entries };
    }
    if data.len() >= 2 && &data[0..2] == b"MZ" {
        return Artifact::Exe { pe: parse_pe(data) };
    }
    if let Some(arch) = macho_arch(data) {
        return Artifact::Macho { arch };
    }
    // DMG: UDIF trailer 'koly' in the last 512 bytes
    if data.len() >= 512 {
        let t = &data[data.len() - 512..];
        if &t[0..4] == b"koly" {
            let version = u32le(t, 4);
            let data_len = u64le(t, 0x28);
            let xml_len = u64le(t, 0xd8);
            // some DMGs are zip-wrapped (e.g. transferred via zip) — check the head too
            let z = if data.len() >= 4 && &data[0..4] == b"PK\x03\x04" { Some(zip_entries(data)) } else { None };
            return Artifact::Dmg { zip_entries: z, xml_len, data_len, version };
        }
    }
    Artifact::Zip { entries: Vec::new() }
}

fn find_entry<'a>(entries: &'a [ZipEntry], name: &str) -> Option<&'a ZipEntry> {
    entries.iter().find(|e| e.name == name)
}

fn safe_name(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}

struct Report {
    lines: Vec<String>,
}

impl Report {
    fn add(&mut self, s: String) { self.lines.push(s); }
}

pub fn setup(path_str: &str) {
    let path = Path::new(path_str);
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => { eprintln!("vulcan: cannot read {path_str}: {e}"); std::process::exit(1); }
    };
    if data.is_empty() {
        eprintln!("vulcan: {path_str} is empty");
        std::process::exit(1);
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
    let artifact = detect(&data);
    let mut report = Report { lines: Vec::new() };
    let project_name = format!("{}-vulcan", safe_name(stem).to_lowercase());
    let root = PathBuf::from(&project_name);

    let kind;
    match artifact {
        Artifact::Apk { entries, manifest, dex } => {
            kind = "Android APK".to_string();
            convert_apk(&data, &entries, manifest.as_ref(), dex.as_ref(), &root, stem, &mut report);
        }
        Artifact::Ipa { entries, bundle_id, bundle_name, version } => {
            kind = "iOS IPA".to_string();
            convert_ipa(&data, &entries, bundle_id, bundle_name, version, &root, stem, &mut report);
        }
        Artifact::Dmg { zip_entries, xml_len, data_len, version } => {
            kind = "macOS DMG".to_string();
            convert_dmg(zip_entries.as_deref(), xml_len, data_len, version, &root, stem, &mut report);
        }
        Artifact::Exe { pe } => {
            kind = "Windows EXE".to_string();
            convert_exe(pe.as_ref(), &root, stem, &mut report);
        }
        Artifact::Macho { arch } => {
            kind = "Mach-O binary".to_string();
            convert_macho(&arch, &root, stem, &mut report);
        }
        Artifact::Zip { entries } => {
            if entries.is_empty() {
                eprintln!("vulcan: could not identify {} — supported: .apk, .ipa, .dmg, .exe (and zip-based variants)", path_str);
                std::process::exit(1);
            }
            kind = "zip archive".to_string();
            convert_zip(&data, &entries, &root, stem, &mut report);
        }
    }
    // write the report
    let mut md = String::new();
    md.push_str(&format!("# Converting {stem} ({kind}) to Vulcan\n\n"));
    md.push_str("Generated by `vulcan setup`. This file maps everything the converter\n");
    md.push_str("extracted from the original artifact onto the generated project.\n\n");
    md.push_str("## What was found\n\n```\n");
    for l in &report.lines { md.push_str(l); md.push('\n'); }
    md.push_str("\n```\n\n## What the scaffold contains\n\n");
    md.push_str(&format!("- `{}/vulcan.toml` — project manifest\n", project_name));
    md.push_str(&format!("- `{}/src/main.vulcan` — runnable entry point modeling the app\n", project_name));
    md.push_str(&format!("- `{}/assets/` — resources extracted from the artifact (stored/uncompressed entries)\n", project_name));
    md.push_str("\n## Notes\n\n");
    md.push_str("- Compiled machine code is not decompiled. The scaffold marks where original\n");
    md.push_str("  logic goes (`TODO` comments); every component the converter found is\n");
    md.push_str("  listed above with its original name so you can rebuild it piece by piece.\n");
    md.push_str("- Deflated (compressed) zip entries are listed in the report but not extracted\n");
    md.push_str("  in this version.\n");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("CONVERSION.md"), md).unwrap();
    println!("converted {stem} ({kind}) -> {project_name}/");
    println!("  {project_name}/vulcan.toml");
    println!("  {project_name}/src/main.vulcan");
    println!("  {project_name}/CONVERSION.md   (full extraction report)");
    println!("\nnext steps:");
    println!("  cd {project_name}");
    println!("  vulcan run            # run the generated scaffold");
    println!("  vulcan package --platform android|ios|macos|linux|windows   # re-ship it");
}

// ------------------------------------------------------------- writers ----

fn write_project(root: &Path, name: &str, main_src: &str, assets: Vec<(String, Vec<u8>)>) {
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(root.join("assets")).unwrap();
    fs::write(root.join("vulcan.toml"), format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\ndescription = \"converted by vulcan setup\"\n\n[deps]\n")).unwrap();
    fs::write(root.join(".gitignore"), "/build\n*.vbin\n*.v.c\n").unwrap();
    fs::write(src.join("main.vulcan"), main_src).unwrap();
    for (rel, bytes) in assets {
        let p = root.join("assets").join(&rel);
        if let Some(parent) = p.parent() { let _ = fs::create_dir_all(parent); }
        let _ = fs::write(p, bytes);
    }
}

fn extract_assets(data: &[u8], entries: &[ZipEntry], filter: &dyn Fn(&str) -> bool) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for e in entries {
        if e.name.ends_with('/') || !filter(&e.name) || !e.stored { continue; }
        if let Some(b) = zip_extract_stored(data, e) {
            let rel = e.name.replace("assets/", "").replace("res/", "res-");
            let rel = rel.split('/').map(safe_name).collect::<Vec<_>>().join("-");
            out.push((rel, b));
        }
    }
    out
}

fn vstr(s: &str) -> String { format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")) }

fn convert_apk(data: &[u8], entries: &[ZipEntry], manifest: Option<&Axml>, dex: Option<&DexInfo>, root: &Path, stem: &str, r: &mut Report) {
    r.add(format!("artifact: {} ({} zip entries)", stem, entries.len()));
    let mut package = String::new();
    let mut activities: Vec<String> = Vec::new();
    let mut permissions: Vec<String> = Vec::new();
    let mut label = String::new();
    if let Some(m) = manifest {
        for (el, attrs) in &m.elements {
            if el == "manifest" {
                for (k, v) in attrs { if k == "package" { package = v.clone(); } }
            }
            if el == "application" {
                for (k, v) in attrs { if k == "label" { label = v.clone(); } }
            }
            if el == "activity" || el == "activity-alias" || el == "service" || el == "receiver" {
                for (k, v) in attrs {
                    if k == "name" {
                        let comp = if v.starts_with('.') { format!("{}{}", package, v) } else { v.clone() };
                        activities.push(format!("{} {}", el, comp));
                    }
                }
            }
            if el == "uses-permission" {
                for (k, v) in attrs { if k == "name" { permissions.push(v.clone()); } }
            }
        }
    }
    r.add(format!("package: {}", if package.is_empty() { "(not found — manifest may be compressed)" } else { &package }));
    r.add(format!("label: {}", if label.is_empty() { "(none)" } else { &label }));
    r.add(format!("components ({}):", activities.len()));
    for a in activities.iter().take(40) { r.add(format!("  {}", a)); }
    if activities.len() > 40 { r.add(format!("  … and {} more", activities.len() - 40)); }
    r.add(format!("permissions ({}):", permissions.len()));
    for p in permissions.iter().take(30) { r.add(format!("  {}", p)); }
    if let Some(d) = dex {
        r.add(format!("dex: {} strings, {} methods, {} fields, {} classes", d.strings, d.methods, d.fields, d.classes.len()));
        for c in d.classes.iter().take(25) { r.add(format!("  class {}", dex_class_name(c))); }
        if d.classes.len() > 25 { r.add(format!("  … and {} more", d.classes.len() - 25)); }
    } else {
        r.add("dex: classes.dex not extractable (compressed) — see report notes".into());
    }
    // native libs + resources
    for e in entries {
        if e.name.starts_with("lib/") && e.name.ends_with(".so") {
            r.add(format!("native lib: {} ({})", e.name, e.size));
        }
    }
    let assets = extract_assets(data, entries, &|n| n.starts_with("assets/") || n.starts_with("res/") && !n.contains(".arsc"));
    r.add(format!("extracted assets: {} (stored/uncompressed entries only)", assets.len()));

    let main_name = activities.iter().find(|a| a.contains("activity "))
        .map(|a| a.rsplit(' ').next().unwrap_or("").to_string())
        .unwrap_or_default();
    let pkg_id = if package.is_empty() { stem.to_string() } else { package.clone() };
    let main_src = format!(r#"// {pkg_id} — converted from Android APK by `vulcan setup`
// Original label: {label}
// See CONVERSION.md for every component, class and permission found.

const PACKAGE: string = {p}
const MAIN_ACTIVITY: string = {m}
const N_COMPONENTS: i64 = {nc}
const N_PERMISSIONS: i64 = {np}

struct Screen {{
    name: string,
    role: string,
}}

// The app's component map — extend as you port each screen's logic.
// (TODO: port the logic from the original {mc} class.)
fn screens() -> [Screen] {{
    return [
        Screen {{ name: "main", role: {m2} }},
    ]
}}

fn main() -> void {{
    print("app:", PACKAGE)
    print("main activity:", MAIN_ACTIVITY)
    print("components:", N_COMPONENTS, "permissions:", N_PERMISSIONS)
    let s = screens()
    print("screens:", s.len())
    for scr in s {{
        print(" -", scr.name, "(", scr.role, ")")
    }}
    // mobile scaffold: this program ships to Android again with
    //   vulcan package --platform android
    ui_title({l})
    print("scaffold ready — fill in the TODOs and re-ship with vulcan package")
}}
"#,
        p = vstr(&pkg_id), m = vstr(&main_name), m2 = vstr(if main_name.is_empty() { "main" } else { &main_name }),
        nc = activities.len(), np = permissions.len(), l = vstr(if label.is_empty() { &pkg_id } else { &label }),
        mc = if main_name.is_empty() { "main activity".to_string() } else { main_name.clone() });
    write_project(root, &safe_name(&project_pkg_name(&pkg_id)), &main_src, assets);
}

fn project_pkg_name(pkg: &str) -> String {
    pkg.rsplit('.').next().filter(|s| !s.is_empty()).unwrap_or("app").to_string()
}

fn convert_ipa(data: &[u8], entries: &[ZipEntry], bundle_id: Option<String>, bundle_name: Option<String>, version: Option<String>, root: &Path, stem: &str, r: &mut Report) {
    r.add(format!("artifact: {} ({} zip entries)", stem, entries.len()));
    let bid = bundle_id.unwrap_or_else(|| "(not found — Info.plist may be compressed)".into());
    let bname = bundle_name.unwrap_or_else(|| stem.into());
    let ver = version.unwrap_or_else(|| "0".into());
    r.add(format!("bundle id: {}", bid));
    r.add(format!("bundle name: {}", bname));
    r.add(format!("version: {}", ver));
    let mut n_exec = 0;
    for e in entries {
        if e.name.ends_with(".app/") { r.add(format!("app bundle: {} ({} files)", e.name.trim_end_matches('/'), entries.iter().filter(|x| x.name.starts_with(&e.name)).count())); }
        if e.name.contains(".app/") && !e.name.ends_with('/') && !e.name.contains("Info.plist") {
            if e.name.ends_with(".png") || e.name.ends_with(".jpg") || e.name.ends_with(".car") || e.name.ends_with(".nib") || e.name.ends_with(".strings") {
                r.add(format!("resource: {}", e.name));
            }
        }
        if e.name.ends_with(".framework/") { r.add(format!("framework: {}", e.name)); }
        if e.name.contains(".app/") && e.name.split('/').count() == 2 && !e.name.ends_with('/') { n_exec += 1; }
    }
    r.add(format!("bundle executables: {}", n_exec));
    let assets = extract_assets(data, entries, &|n| n.contains(".app/") && (n.ends_with(".png") || n.ends_with(".jpg")));
    r.add(format!("extracted icons: {}", assets.len()));
    let main_src = format!(r#"// {bname} — converted from iOS IPA by `vulcan setup`
// Bundle {bid} v{ver}. See CONVERSION.md for every resource found.

const BUNDLE_ID: string = {bid2}
const BUNDLE_NAME: string = {bn2}
const VERSION: string = {v2}

fn main() -> void {{
    print("app:", BUNDLE_NAME, "v" + VERSION)
    print("bundle id:", BUNDLE_ID)
    // mobile scaffold: this program ships to iOS again with
    //   vulcan package --platform ios
    ui_title({bn2})
    print("scaffold ready — fill in the TODOs and re-ship with vulcan package")
}}
"#,
        bid2 = vstr(&bid), bn2 = vstr(&bname), v2 = vstr(&ver));
    write_project(root, &safe_name(&project_pkg_name(&bid)), &main_src, assets);
}

fn convert_dmg(zip_entries: Option<&[ZipEntry]>, xml_len: u64, data_len: u64, version: u32, root: &Path, stem: &str, r: &mut Report) {
    r.add(format!("artifact: {} (UDIF dmg, format version {})", stem, version));
    r.add(format!("data fork: {} bytes", data_len));
    r.add(format!("partition plist: {} bytes", xml_len));
    if let Some(z) = zip_entries {
        r.add(format!("zip-wrapped: {} entries", z.len()));
        for e in z.iter().take(20) { r.add(format!("  {}", e.name)); }
    } else {
        r.add("contents: HFS+/APFS filesystem image — the converter lists structure only;".into());
        r.add("  mount the dmg (hdiutil attach / macOS) to extract files, then run:".into());
        r.add("  vulcan setup <App.app> or vulcan setup <binary> on the inner payload".into());
    }
    let main_src = format!(r#"// {stem} — converted from macOS DMG by `vulcan setup`
// UDIF image, format version {version}. See CONVERSION.md for details.

const IMAGE: string = {s2}

fn main() -> void {{
    print("image:", IMAGE)
    // desktop scaffold: this program ships to macOS again with
    //   vulcan package --platform macos
    ui_title({s2})
    print("scaffold ready — fill in the TODOs and re-ship with vulcan package")
}}
"#, s2 = vstr(stem));
    write_project(root, &safe_name(stem), &main_src, Vec::new());
}

fn convert_exe(pe: Option<&PeInfo>, root: &Path, stem: &str, r: &mut Report) {
    r.add(format!("artifact: {} (Windows PE executable)", stem));
    match pe {
        Some(p) => {
            r.add(format!("machine: {}", p.machine));
            r.add(format!("subsystem: {}", p.subsystem));
            r.add(format!("entry point RVA: 0x{:x}", p.entry_rva));
            if p.is_dotnet { r.add("managed: yes (.NET/CLR image — IL is present)".into()); }
            r.add(format!("sections ({}):", p.sections.len()));
            for (n, vs, rs) in p.sections.iter() { r.add(format!("  {} vsize={} raw={}", n, vs, rs)); }
            r.add(format!("imports ({} DLLs):", p.imports.len()));
            for (dll, funcs) in p.imports.iter() {
                r.add(format!("  {} ({} functions)", dll, funcs.len()));
                for f in funcs.iter().take(8) { r.add(format!("    {}", f)); }
                if funcs.len() > 8 { r.add(format!("    … +{} more", funcs.len() - 8)); }
            }
            let total_imports: usize = p.imports.iter().map(|(_, f)| f.len()).sum();
            let dlls: Vec<String> = p.imports.iter().map(|(d, _)| d.clone()).collect();
            let machine_line = format!("{} / {}.", p.machine, p.subsystem);
            let main_src = format!(r#"// {stem} — converted from Windows executable by `vulcan setup`
// {machine_line} See CONVERSION.md for the full import map.

const MACHINE: string = {m}
const SUBSYSTEM: string = {ss}
const N_DLLS: i64 = {nd}
const N_IMPORTS: i64 = {ni}

// External C libraries the original linked against — call them from
// vulcan with `extern "c" {{ fn name(args) -> ret }}` + `link "dll"`.
fn linked_dlls() -> [string] {{
    let out: [string] = [{dll_list}]
    return out
}}

fn main() -> void {{
    print("binary:", MACHINE, "/", SUBSYSTEM)
    print("imports:", N_IMPORTS, "functions across", N_DLLS, "DLLs")
    let dlls = linked_dlls()
    for d in dlls {{
        print(" -", d)
    }}
    // desktop scaffold: this program ships to Windows again with
    //   vulcan package --platform windows
    ui_title({s2})
    print("scaffold ready — fill in the TODOs and re-ship with vulcan package")
}}
"#,
                machine_line = machine_line,
                m = vstr(&p.machine), ss = vstr(&p.subsystem), nd = p.imports.len(), ni = total_imports,
                dll_list = dlls.iter().map(|d| vstr(d)).collect::<Vec<_>>().join(", "),
                s2 = vstr(stem));
            write_project(root, &safe_name(stem), &main_src, Vec::new());
        }
        None => {
            r.add("parse: not a valid PE image (MZ magic without PE header)".into());
            let main_src = format!("#[comment] stub\nfn main() -> void {{ print(\"{}\") }}\n", vstr(stem));
            write_project(root, &safe_name(stem), &main_src, Vec::new());
        }
    }
}

fn convert_macho(arch: &str, root: &Path, stem: &str, r: &mut Report) {
    r.add(format!("artifact: {} (Mach-O binary, {})", stem, arch));
    r.add("note: a raw Mach-O carries no bundle metadata; for a .app bundle run".into());
    r.add("  vulcan setup on the .ipa or the bundle directory zip instead".into());
    let main_src = format!(r#"// {stem} — converted from Mach-O ({arch}) by `vulcan setup`

const ARCH: string = {a}

fn main() -> void {{
    print("binary:", ARCH)
    ui_title({s2})
    print("scaffold ready — fill in the TODOs and re-ship with vulcan package")
}}
"#, a = vstr(arch), s2 = vstr(stem));
    write_project(root, &safe_name(stem), &main_src, Vec::new());
}

fn convert_zip(data: &[u8], entries: &[ZipEntry], root: &Path, stem: &str, r: &mut Report) {
    r.add(format!("artifact: {} (zip archive, {} entries)", stem, entries.len()));
    for e in entries.iter().take(50) { r.add(format!("  {} ({} bytes)", e.name, e.size)); }
    if entries.len() > 50 { r.add(format!("  … and {} more", entries.len() - 50)); }
    // a zipped .app bundle (macOS app shipped as zip) converts like an IPA
    let has_app = entries.iter().any(|e| e.name.ends_with(".app/Info.plist"));
    if has_app {
        r.add("detected macOS .app bundle inside — converting as app".into());
        let mut bid = None; let mut bname = None; let mut ver = None;
        for e in entries {
            if e.name.ends_with(".app/Info.plist") {
                if let Some(b) = zip_extract_stored(data, e) {
                    bid = plist_field(&b, "CFBundleIdentifier").or(bid);
                    bname = plist_field(&b, "CFBundleName").or(bname);
                    ver = plist_field(&b, "CFBundleShortVersionString").or(ver);
                }
            }
        }
        let bid = bid.unwrap_or_else(|| stem.into());
        let bname = bname.unwrap_or_else(|| stem.into());
        let main_src = format!(r#"// {bname} — converted from macOS .app (zipped) by `vulcan setup`

const BUNDLE_ID: string = {bid2}
const BUNDLE_NAME: string = {bn2}

fn main() -> void {{
    print("app:", BUNDLE_NAME)
    print("bundle id:", BUNDLE_ID)
    ui_title({bn2})
    print("scaffold ready — fill in the TODOs and re-ship with vulcan package")
}}
"#, bid2 = vstr(&bid), bn2 = vstr(&bname));
        write_project(root, &safe_name(&project_pkg_name(&bid)), &main_src, Vec::new());
        return;
    }
    let assets = extract_assets(data, entries, &|_| true);
    let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
    let main_src = format!(r#"// {stem} — converted from zip archive by `vulcan setup`
// {n} entries; see CONVERSION.md for the full listing.

const N_ENTRIES: i64 = {n2}

fn main() -> void {{
    print("archive entries:", N_ENTRIES)
    // TODO: port the archive's contents into vulcan source files
    print("scaffold ready — see CONVERSION.md")
}}
"#, n = names.len(), n2 = names.len());
    write_project(root, &safe_name(stem), &main_src, assets);
}
