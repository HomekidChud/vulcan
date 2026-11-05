@name Arcade
@desc Flat colour lighting for the Vulcan Arcade games.

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

@sky.fs
void main() {
    vec4 p = uInvViewProj * vec4(vUV * 2.0 - 1.0, 1.0, 1.0);
    vec3 d = normalize(p.xyz / p.w - uCamPos);
    vec3 c = skyColor(d);
    float n = hash13(floor(d * 220.0));
    if (n > 0.9975 && d.y > 0.0) c += vec3(1.0) * (1.0 - uDay) * (n - 0.9975) * 300.0;
    fragColor = vec4(c, 1.0);
}

@post0.fs
void main() {
    vec3 c = texture(uColor, vUV).rgb;
    c = c / (1.0 + c * 0.25);
    c = pow(c, vec3(1.0 / 2.2));
    float v = smoothstep(1.2, 0.35, length(vUV - 0.5) * 1.35);
    fragColor = vec4(c * mix(0.88, 1.0, v), 1.0);
}
