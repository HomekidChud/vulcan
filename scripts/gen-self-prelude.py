#!/usr/bin/env python3
"""Regenerate the VPRELUDE const inside self/vulc.vulcan.

The self-hosted compiler must emit the exact same runtime prelude as the
Rust compiler (src/codegen.rs), so we extract it mechanically: compile a
trivial program with `vulcan emit` and take everything before the
"// ---- constants ----" marker. The text is embedded as a single escaped
Vulcan string literal between BEGIN/END GENERATED PRELUDE markers.

Usage:  python3 scripts/gen-self-prelude.py   (run from the repo root,
        with ./target/release/vulcan already built)
"""
import re
import subprocess
import sys
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VULC = os.path.join(ROOT, "self", "vulc.vulcan")
BIN = os.path.join(ROOT, "target", "release", "vulcan")

probe = os.path.join(ROOT, "self", "test", "_prelude_probe.vulcan")
os.makedirs(os.path.dirname(probe), exist_ok=True)
with open(probe, "w") as f:
    f.write("fn main() { }\n")

r = subprocess.run([BIN, "emit", probe], capture_output=True, text=True, cwd=ROOT)
if r.returncode != 0:
    sys.exit("vulcan emit failed:\n" + r.stderr[-2000:])

marker = "// ---- constants ----"
if marker not in r.stdout:
    sys.exit("marker not found in emitted C")
prelude = r.stdout[: r.stdout.index(marker)].rstrip() + "\n"
os.unlink(probe)

# escape into a single Vulcan string literal (Vulcan escapes: \n \t \r \" \\)
esc = (
    prelude.replace("\\", "\\\\")
    .replace('"', '\\"')
    .replace("\n", "\\n")
    .replace("\t", "\\t")
)

section = (
    "// BEGIN GENERATED PRELUDE (scripts/gen-self-prelude.py) — do not edit\n"
    'const VPRELUDE: string = "' + esc + '"\n'
    "// END GENERATED PRELUDE"
)

src = open(VULC).read()
if "// BEGIN GENERATED PRELUDE" in src:
    src = re.sub(
        r"// BEGIN GENERATED PRELUDE.*?// END GENERATED PRELUDE",
        lambda m: section,
        src,
        flags=re.S,
    )
else:
    sys.exit("prelude markers not found in self/vulc.vulcan")
open(VULC, "w").write(src)
print("VPRELUDE embedded: %d C bytes" % len(prelude))
