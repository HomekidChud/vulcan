@name Cinematic
@desc Warm teal-and-orange film grade, soft bloom, chromatic fringing, grain and letterbox bars on top of the vanilla lighting.

@common
vec3 toLinear(vec3 c) { return pow(c, vec3(2.2)); }
float hash13(vec3 p) { p = fract(p * 0.1031); p += dot(p, p.zyx + 31.32); return fract((p.x + p.y) * p.z); }
vec3 skyColor(vec3 d) {
    float h = clamp(d.y, 0.0, 1.0);
    vec3 c = mix(uSkyHorizon, uSkyTop, pow(h, 0.55));
    float s = max(dot(d, uSunDir), 0.0);
    c += uSunColor * (pow(s, 900.0) * 6.0 + pow(s, 12.0) * 0.18) * uDay;
    return c;
}

@scene.vs
out vec2 vUV; out vec3 vN; out vec3 vL; out float vMat; out vec3 vW;
void main() {
    float c = cos(uYaw), s = sin(uYaw);
    vec3 p = vec3(c * aPos.x + s * aPos.z, aPos.y, -s * aPos.x + c * aPos.z);
    vec3 n = vec3(c * aNormal.x + s * aNormal.z, aNormal.y, -s * aNormal.x + c * aNormal.z);
    vec3 w = p + uOrigin;
    float sway = 0.0;
    if (aMat > 1.5 && aMat < 2.5) sway = 0.06;
    if (aMat > 2.5 && aMat < 3.5) sway = 0.025;
    w.x += sin(uTime * 1.7 + w.x * 0.9 + w.z * 0.6) * sway;
    w.z += cos(uTime * 1.3 + w.z * 0.8 + w.x * 0.5) * sway;
    vUV = aUV; vN = n; vL = aLight; vMat = aMat; vW = w;
    gl_Position = uViewProj * vec4(w, 1.0);
}

@scene.fs
in vec2 vUV; in vec3 vN; in vec3 vL; in float vMat; in vec3 vW;
void main() {
    vec4 t = textureLod(uAtlas, vUV, 1.0);
    if (uLayer == 0 && t.a < 0.5) discard;
    vec3 albedo = toLinear(t.rgb);
    vec3 N = normalize(vN);
    float sun = max(dot(N, uSunDir), 0.0);
    float sky = vL.x;
    vec3 amb = uAmbient * (0.65 + 0.35 * N.y);
    vec3 light = sky * (amb + uSunColor * sun * 0.55 * uDay) + vL.y * vec3(1.15, 0.85, 0.55);
    light = max(light, vec3(0.02));
    vec3 col = albedo * light * vL.z;
    if (vMat > 3.5 && vMat < 4.5) col = albedo * 1.6;
    float a = t.a;
    if (vMat > 0.5 && vMat < 1.5) a = 0.72;
    if (vMat > 6.5) a = t.a;
    float d = length(vW - uCamPos);
    float f = 1.0 - exp(-d * uFog);
    col = mix(col, uSkyHorizon, clamp(f, 0.0, 1.0));
    fragColor = vec4(col, a);
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
    float n = hash13(floor(d * 220.0));
    if (n > 0.9975 && d.y > 0.0) c += vec3(1.0) * (1.0 - uDay) * (n - 0.9975) * 300.0;
    fragColor = vec4(c, 1.0);
}

@post0.fs scale=0.5
void main() {
    vec3 c = texture(uColor, vUV).rgb;
    float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
    fragColor = vec4(c * smoothstep(0.9, 2.2, l), 1.0);
}

@post1.fs scale=0.5
void main() {
    vec2 px = 1.0 / vec2(textureSize(uColor, 0));
    vec3 s = vec3(0.0);
    float tw = 0.0;
    for (int i = -4; i <= 4; i++) {
        float w = exp(-float(i * i) / 8.0);
        s += texture(uColor, vUV + vec2(float(i) * px.x * 2.0, 0.0)).rgb * w;
        tw += w;
    }
    fragColor = vec4(s / tw, 1.0);
}

@post2.fs
vec3 aces(vec3 x) { return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0); }
float h21(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
void main() {
    vec2 d = vUV - 0.5;
    float r2 = dot(d, d);
    vec2 off = d * r2 * 0.02;
    vec3 c = vec3(texture(uScene, vUV + off).r, texture(uScene, vUV).g, texture(uScene, vUV - off).b);
    vec2 bp = 4.0 / vec2(textureSize(uPass1, 0));
    vec3 bl = vec3(0.0);
    for (int i = -3; i <= 3; i++) bl += texture(uPass1, vUV + vec2(0.0, float(i) * bp.y)).rgb * (1.0 - abs(float(i)) / 4.0);
    bl /= 4.0;
    c += bl * 0.55;
    c = aces(c * 1.12);
    float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
    vec3 shadowTint = vec3(0.86, 1.0, 1.1);
    vec3 highTint = vec3(1.12, 1.0, 0.86);
    c *= mix(shadowTint, highTint, smoothstep(0.1, 0.8, l));
    c = mix(vec3(dot(c, vec3(0.299, 0.587, 0.114))), c, 1.12);
    c = pow(c, vec3(1.0 / 2.2));
    c *= 1.0 - smoothstep(0.25, 0.75, r2 * 2.2) * 0.6;
    c += (h21(vUV * vec2(uResolution) + fract(uTime) * 91.0) - 0.5) * 0.035;
    float bars = 0.5 - abs(vUV.y - 0.5);
    float bar = smoothstep(0.0, 0.002, bars - 0.075);
    fragColor = vec4(c * bar, 1.0);
}
