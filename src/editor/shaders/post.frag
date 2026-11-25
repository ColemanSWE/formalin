#version 150
// The monitor: everything the effect does to the sound, it also does to the picture.
//   wow      -> the image sways and a tracking band rolls up the screen
//   flutter  -> per-line jitter
//   drive    -> chromatic split
//   age      -> grain
//   dead     -> pixels drip into streaks and drop out to static
//              (the dissolve from the portfolio's hero)
in vec2 vUv;
out vec4 frag;
uniform sampler2D uTex;
uniform vec2 uRes;
uniform float uTime, uWow, uFlutter, uDrive, uAge, uDead;

float h(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }

void main() {
    vec2 uv = vUv;
    float line = floor(uv.y * uRes.y / 2.0);

    uv.x += sin(uv.y * 5.0 + uTime * 1.1) * uWow * 0.004;
    uv.x += (h(vec2(line, floor(uTime * 30.0))) - 0.5) * uFlutter * 0.004;

    float roll = fract(uTime * 0.06);
    float band = smoothstep(0.035, 0.0, abs(uv.y - roll)) * (0.25 + uWow);
    uv.x += band * 0.02 * (h(vec2(line, floor(uTime * 60.0))) - 0.3);

    vec2 cell = floor(uv * uRes / 2.0);
    float colN = h(vec2(floor(uv.x * uRes.x / 3.0), 7.0));
    float pxN = h(cell);
    float s = smoothstep(0.0, 1.0, uDead);
    uv.y += s * s * (0.45 * colN + 1.4 * pxN * pxN) * 0.3;

    float ca = 0.0012 + 0.007 * uDrive;
    vec3 c = vec3(texture(uTex, uv + vec2(ca, 0.0)).r,
                  texture(uTex, uv).g,
                  texture(uTex, uv - vec2(ca, 0.0)).b);
    c += band * 0.06;

    c *= 0.86 + 0.14 * sin(gl_FragCoord.y * 3.14159);
    c += (h(gl_FragCoord.xy + fract(uTime * 7.1) * 91.0) - 0.5) * (0.035 + 0.06 * uAge);

    float keep = step(s * 0.9, h(cell + 3.17));
    float st = h(cell + floor(uTime * 24.0));
    // Capped, so a burned-in ghost of the scene survives a full stop.
    c = mix(c, vec3(st) * 0.5, (1.0 - keep) * s * 0.8);

    vec2 v = vUv - 0.5;
    c *= 1.0 - dot(v, v) * 0.9;
    frag = vec4(c, 1.0);
}
