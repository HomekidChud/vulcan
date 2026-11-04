@name Lumina
@desc Rich realistic look: soft sun shadows, SSAO, bloom, god rays, animated water, volumetric-style clouds, ACES tonemapping, FXAA.

@common
vec3 toLinear(vec3 c) { return pow(c, vec3(2.2)); }
float hash12(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * 0.1031); p3 += dot(p3, p3.yzx + 33.33); return fract((p3.x + p3.y) * p3.z); }
float hash13(vec3 p) { p = fract(p * 0.1031); p += dot(p, p.zyx + 31.32); return fract((p.x + p.y) * p.z); }
float vnoise(vec2 p) {
    vec2 i = floor(p), f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2(1, 0)), f.x), mix(hash12(i + vec2(0, 1)), hash12(i + vec2(1, 1)), f.x), f.y);
}
float fbm(vec2 p) {
    float a = 0.5, s = 0.0;
    for (int i = 0; i < 5; i++) { s += a * vnoise(p); p = p * 2.03 + vec2(17.1, 9.2); a *= 0.5; }
    return s;
}
vec3 skyColor(vec3 d) {
    float h = clamp(d.y, 0.0, 1.0);
    vec3 c = mix(uSkyHorizon, uSkyTop, pow(h, 0.5));
    float s = max(dot(d, uSunDir), 0.0);
    c += uSunColor * pow(s, 8.0) * 0.12;
    return c;
}
vec3 aces(vec3 x) {
    x *= 0.55;
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
float linDepth(float d) {
    float z = d * 2.0 - 1.0;
    return 2.0 * uNear * uFar / (uFar + uNear - z * (uFar - uNear));
}
vec3 waveNormal(vec3 w, float t) {
    float dx = cos(w.x * 1.3 + t * 1.6) * 0.65 + cos((w.x + w.z) * 2.1 + t * 2.3) * 0.35 + cos(w.x * 4.1 - t * 2.9 + w.z * 1.7) * 0.18;
    float dz = cos(w.z * 1.1 + t * 1.3) * 0.6 + cos((w.x - w.z) * 1.9 - t * 2.0) * 0.4 + cos(w.z * 3.7 + t * 3.1 - w.x * 1.3) * 0.18;
    return normalize(vec3(-dx * 0.05, 1.0, -dz * 0.05));
}

@scene.vs
out vec2 vUV; out vec3 vN; out vec3 vL; out float vMat; out vec3 vW;
void main() {
    float c = cos(uYaw), s = sin(uYaw);
    vec3 p = vec3(c * aPos.x + s * aPos.z, aPos.y, -s * aPos.x + c * aPos.z);
    vec3 n = vec3(c * aNormal.x + s * aNormal.z, aNormal.y, -s * aNormal.x + c * aNormal.z);
    vec3 w = p + uOrigin;
    float sway = 0.0;
    if (aMat > 1.5 && aMat < 2.5) sway = 0.09;
    if (aMat > 2.5 && aMat < 3.5) sway = 0.03;
    float ph = w.x * 0.8 + w.z * 0.7;
    w.x += (sin(uTime * 1.9 + ph) + 0.5 * sin(uTime * 3.7 + ph * 2.0)) * sway;
    w.z += cos(uTime * 1.5 + ph * 1.2) * sway;
    if (aMat > 0.5 && aMat < 1.5) {
        w.y += (sin(w.x * 1.3 + uTime * 1.6) * 0.5 + sin(w.z * 1.1 + uTime * 1.3) * 0.5) * 0.04 - 0.03;
    }
    vUV = aUV; vN = n; vL = aLight; vMat = aMat; vW = w;
    gl_Position = uViewProj * vec4(w, 1.0);
}

@scene.fs
in vec2 vUV; in vec3 vN; in vec3 vL; in float vMat; in vec3 vW;
const vec2 PD[12] = vec2[12](vec2(-0.326,-0.406), vec2(-0.840,-0.074), vec2(-0.696,0.457), vec2(-0.203,0.621), vec2(0.962,-0.195), vec2(0.473,-0.480), vec2(0.519,0.767), vec2(0.185,-0.893), vec2(0.507,0.064), vec2(0.896,0.412), vec2(-0.322,-0.933), vec2(-0.792,-0.598));
float shadowAt(vec3 wp, vec3 n, float ndl) {
    if (uShadowOn == 0) return 1.0;
    vec4 sp = uShadowMat * vec4(wp + n * (0.05 + 0.05 * (1.0 - ndl)), 1.0);
    vec3 pc = sp.xyz / sp.w * 0.5 + 0.5;
    if (pc.z > 1.0) return 1.0;
    float edge = max(abs(pc.x - 0.5), abs(pc.y - 0.5)) * 2.0;
    if (edge > 1.0) return 1.0;
    float bias = 0.0004 + 0.0012 * (1.0 - ndl);
    float ang = hash12(gl_FragCoord.xy) * 6.2831;
    float cs = cos(ang), sn = sin(ang);
    float lit = 0.0;
    for (int i = 0; i < 12; i++) {
        vec2 o = vec2(PD[i].x * cs - PD[i].y * sn, PD[i].x * sn + PD[i].y * cs) * uShadowTexel * 2.2;
        lit += (pc.z - bias > texture(uShadow, pc.xy + o).r) ? 0.0 : 1.0;
    }
    lit /= 12.0;
    return mix(lit, 1.0, smoothstep(0.82, 1.0, edge));
}
void main() {
    vec4 t = textureLod(uAtlas, vUV, 0.0);
    // soften texel crawl at distance with a mip bias
    float dist = length(vW - uCamPos);
    t = mix(t, textureLod(uAtlas, vUV, 2.0), clamp((dist - 24.0) / 60.0, 0.0, 1.0));
    if (uLayer == 0 && t.a < 0.5) discard;
    vec3 albedo = toLinear(t.rgb);
    vec3 N = normalize(vN);
    vec3 V = normalize(uCamPos - vW);
    float alpha = t.a;
    bool water = vMat > 0.5 && vMat < 1.5;
    bool glass = vMat > 4.5 && vMat < 5.5;
    bool emissive = vMat > 3.5 && vMat < 4.5;
    if (water) N = waveNormal(vW, uTime);
    float ndl = max(dot(N, uSunDir), 0.0);
    // plants and leaves are lit from all sides
    if (vMat > 1.5 && vMat < 3.5) ndl = max(ndl, 0.45);
    if (vMat > 7.5) ndl = max(ndl, 0.45);
    float sh = 1.0;
    if (ndl > 0.0 && !water) sh = shadowAt(vW, normalize(vN), ndl);
    float skyVis = vL.x;
    float exposure = smoothstep(0.15, 0.9, skyVis);
    vec3 direct = uSunColor * ndl * sh * exposure;
    float hemi = 0.5 + 0.5 * N.y;
    vec3 amb = uAmbient * (0.45 + 0.55 * hemi) * skyVis * vL.z * 1.15 + vec3(0.012, 0.014, 0.02) * vL.z;
    float bl = pow(vL.y, 1.5);
    vec3 blockLight = bl * vec3(1.7, 1.05, 0.5) * mix(1.0, vL.z, 0.7);
    vec3 col = albedo * (direct + amb + blockLight);
    if (emissive) col = albedo * 2.4 + albedo * 0.4 * direct;
    if (vMat > 5.5 && vMat < 6.5) col = albedo * (direct * 0.9 + amb + blockLight * 0.5);
    if (water) {
        vec3 R = reflect(-V, N);
        float fres = 0.02 + 0.98 * pow(1.0 - max(dot(N, V), 0.0), 5.0);
        vec3 refl = skyColor(vec3(R.x, abs(R.y), R.z));
        vec3 deep = albedo * (amb * 1.2 + direct * 0.25 + 0.01);
        col = mix(deep, refl, clamp(fres + 0.08, 0.0, 1.0));
        float spec = pow(max(dot(R, uSunDir), 0.0), 220.0);
        col += uSunColor * spec * 3.0 * exposure;
        alpha = mix(0.62, 0.95, fres);
    }
    if (glass) {
        vec3 R = reflect(-V, N);
        float fres = 0.04 + 0.96 * pow(1.0 - max(dot(N, V), 0.0), 5.0);
        col = col + skyColor(vec3(R.x, abs(R.y), R.z)) * fres * 0.5;
        alpha = max(alpha, 0.2 + fres * 0.6);
    }
    // aerial perspective with sun scatter
    vec3 vd = -V;
    float sc = pow(max(dot(vd, uSunDir), 0.0), 6.0);
    vec3 fogCol = uSkyHorizon + uSunColor * sc * 0.18;
    float fd = dist * uFog;
    float fog = 1.0 - exp(-fd * fd);
    fog = max(fog, 1.0 - exp(-max(0.0, 30.0 - vW.y) * 0.003 * dist * 0.1));
    col = mix(col, fogCol, clamp(fog, 0.0, 1.0));
    fragColor = vec4(col, alpha);
}

@shadow.vs
out vec2 vUV; out float vMat;
void main() {
    float c = cos(uYaw), s = sin(uYaw);
    vec3 p = vec3(c * aPos.x + s * aPos.z, aPos.y, -s * aPos.x + c * aPos.z) + uOrigin;
    vUV = aUV; vMat = aMat;
    gl_Position = uViewProj * vec4(p, 1.0);
}

@shadow.fs
in vec2 vUV; in float vMat;
void main() {
    if (textureLod(uAtlas, vUV, 1.0).a < 0.5) discard;
    fragColor = vec4(1.0);
}

@sky.fs
void main() {
    vec4 p = uInvViewProj * vec4(vUV * 2.0 - 1.0, 1.0, 1.0);
    vec3 d = normalize(p.xyz / p.w - uCamPos);
    vec3 c = skyColor(d);
    float s = dot(d, uSunDir);
    float disc = smoothstep(0.9994, 0.9998, s);
    float halo = pow(max(s, 0.0), 48.0) * 0.5;
    vec3 sunCol = normalize(uSunColor + 0.001) * 3.2;
    c += sunCol * disc * (0.4 + 0.6 * uDay) + uSunColor * halo * 0.4;
    // stars
    vec3 sd = d * 160.0;
    float star = hash13(floor(sd));
    float tw = 0.6 + 0.4 * sin(uTime * 3.0 + star * 40.0);
    c += vec3(1.0, 0.95, 0.9) * step(0.9975, star) * smoothstep(0.0, 0.2, d.y) * (1.0 - uDay) * tw * 1.4;
    // clouds on a plane
    if (d.y > 0.02) {
        vec2 cp = d.xz / (d.y + 0.12) * 1.4 + vec2(uTime * 0.012, uTime * 0.004);
        float cl = fbm(cp * 1.6);
        float cov = smoothstep(0.52, 0.78, cl);
        float shade = fbm(cp * 1.6 + uSunDir.xz * 0.12);
        float lit = clamp(0.55 + (cl - shade) * 3.5, 0.15, 1.2);
        vec3 cc = mix(uSkyHorizon * 0.9, uSunColor * 0.45 + uAmbient * 0.8, 0.7) * lit;
        cc += uSunColor * pow(max(s, 0.0), 6.0) * 0.18 * cov;
        c = mix(c, cc, cov * smoothstep(0.02, 0.2, d.y) * 0.92);
    }
    fragColor = vec4(c, 1.0);
}

@post0.fs scale=0.5
// screen-space ambient occlusion (output: R = occlusion factor)
void main() {
    float d0 = texture(uDepth, vUV).r;
    if (d0 >= 0.99999) { fragColor = vec4(1.0); return; }
    float z0 = linDepth(d0);
    float ang = hash12(gl_FragCoord.xy) * 6.2831;
    float occ = 0.0;
    float radius = clamp(2.4 / z0, 0.004, 0.05);
    for (int i = 0; i < 10; i++) {
        float fi = float(i);
        float a = ang + fi * 2.399;
        float r = radius * (0.25 + 0.75 * fract(fi * 0.618 + 0.3));
        vec2 uv = vUV + vec2(cos(a), sin(a)) * r * vec2(uResolution.y / uResolution.x, 1.0);
        float z = linDepth(texture(uDepth, uv).r);
        float diff = z0 - z;
        occ += step(0.05, diff) * smoothstep(1.6, 0.3, diff);
    }
    float ao = 1.0 - occ / 10.0 * 0.85;
    fragColor = vec4(ao, ao, ao, 1.0);
}

@post1.fs scale=0.5
// bloom: bright pass with a small blur
void main() {
    vec2 px = 1.0 / uResolution * 2.0;
    vec3 sum = vec3(0.0);
    float wsum = 0.0;
    for (int y = -2; y <= 2; y++) for (int x = -2; x <= 2; x++) {
        vec3 c = texture(uScene, vUV + vec2(x, y) * px).rgb;
        float l = dot(c, vec3(0.299, 0.587, 0.114));
        float k = max(l - 1.1, 0.0);
        float w = exp(-float(x * x + y * y) * 0.3);
        sum += c * (k / max(l, 0.001)) * w;
        wsum += w;
    }
    fragColor = vec4(sum / wsum, 1.0);
}

@post2.fs scale=0.25
// bloom: wide gaussian blur
void main() {
    vec2 px = 1.0 / uResolution;
    vec3 sum = vec3(0.0);
    float wsum = 0.0;
    for (int y = -3; y <= 3; y++) for (int x = -3; x <= 3; x++) {
        float w = exp(-float(x * x + y * y) * 0.18);
        sum += texture(uColor, vUV + vec2(x, y) * px * 2.0).rgb * w;
        wsum += w;
    }
    fragColor = vec4(sum / wsum, 1.0);
}

@post3.fs
// composite: SSAO, bloom, god rays, ACES tonemap, colour grade, vignette
void main() {
    vec3 col = texture(uScene, vUV).rgb;
    float ao = texture(uPass0, vUV).r;
    col *= mix(1.0, ao, 0.9);
    // god rays toward the sun
    vec3 rays = vec3(0.0);
    if (uSunScreen.z > 0.5) {
        vec2 sp = uSunScreen.xy;
        vec2 dv = (sp - vUV) / 18.0;
        vec2 uv = vUV;
        float acc = 0.0;
        float decay = 1.0;
        for (int i = 0; i < 18; i++) {
            uv += dv * (0.6 + 0.4 * hash12(gl_FragCoord.xy + float(i)));
            float sky = step(0.99999, texture(uDepth, uv).r);
            acc += sky * decay;
            decay *= 0.94;
        }
        float dsun = length((vUV - sp) * vec2(uResolution.x / uResolution.y, 1.0));
        rays = uSunColor * acc / 18.0 * 0.22 * smoothstep(1.4, 0.0, dsun) * uDay;
    }
    col += rays;
    col += texture(uPass2, vUV).rgb * 0.75 + texture(uPass1, vUV).rgb * 0.25;
    col = aces(col);
    // gentle grade: warm highlights, cool shadows, a little saturation
    float l = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(vec3(l), col, 0.98);
    col *= mix(vec3(0.97, 0.99, 1.04), vec3(1.04, 1.0, 0.95), l);
    col = pow(col, vec3(1.0 / 2.2));
    float v = smoothstep(1.25, 0.35, length(vUV - 0.5) * 1.4);
    col *= mix(0.8, 1.0, v);
    fragColor = vec4(col, 1.0);
}

@post4.fs
// FXAA (luma edge-directed blur)
void main() {
    vec2 px = 1.0 / uResolution;
    vec3 c = texture(uColor, vUV).rgb;
    vec3 cn = texture(uColor, vUV + vec2(0.0, -px.y)).rgb;
    vec3 cs = texture(uColor, vUV + vec2(0.0, px.y)).rgb;
    vec3 ce = texture(uColor, vUV + vec2(px.x, 0.0)).rgb;
    vec3 cw = texture(uColor, vUV + vec2(-px.x, 0.0)).rgb;
    vec3 lw = vec3(0.299, 0.587, 0.114);
    float lc = dot(c, lw), ln = dot(cn, lw), ls = dot(cs, lw), le = dot(ce, lw), lwv = dot(cw, lw);
    float lmin = min(lc, min(min(ln, ls), min(le, lwv)));
    float lmax = max(lc, max(max(ln, ls), max(le, lwv)));
    float range = lmax - lmin;
    if (range < max(0.0312, lmax * 0.125)) { fragColor = vec4(c, 1.0); return; }
    vec2 dir = vec2(-((ln + (dot(texture(uColor, vUV + vec2(px.x, -px.y)).rgb, lw))) - (ls + dot(texture(uColor, vUV + vec2(px.x, px.y)).rgb, lw))),
                    ((le + dot(texture(uColor, vUV + vec2(px.x, -px.y)).rgb, lw)) - (lwv + dot(texture(uColor, vUV + vec2(-px.x, -px.y)).rgb, lw))));
    float dr = max((ln + ls + le + lwv) * 0.25 * 0.25, 1.0 / 128.0);
    float rcp = 1.0 / (min(abs(dir.x), abs(dir.y)) + dr);
    dir = clamp(dir * rcp, vec2(-8.0), vec2(8.0)) * px;
    vec3 a = 0.5 * (texture(uColor, vUV + dir * (1.0 / 3.0 - 0.5)).rgb + texture(uColor, vUV + dir * (2.0 / 3.0 - 0.5)).rgb);
    vec3 b = a * 0.5 + 0.25 * (texture(uColor, vUV + dir * -0.5).rgb + texture(uColor, vUV + dir * 0.5).rgb);
    float lb = dot(b, lw);
    if (lb < lmin || lb > lmax) fragColor = vec4(a, 1.0); else fragColor = vec4(b, 1.0);
}
