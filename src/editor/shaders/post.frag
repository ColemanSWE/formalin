// The front of the tank: what you see the scene through.
//   wow     -> the image sways and a tracking band rolls up the screen
//   flutter -> per-line jitter
//   drive   -> heat haze rising off the orb
//   smear   -> ink in the water (from the ink pass)
//   age     -> grime and algae on the glass; fingerprints stay longer
//   stop    -> fixation: the formalin clouds milky from the edges inward
in vec2 vUv;
out vec4 frag;
uniform sampler2D uTex;
uniform sampler2D uInk;
uniform vec2 uRes;
uniform vec2 uOrb;
uniform float uOrbScreenR;
uniform float uTime, uWorld, uWow, uFlutter, uDrive, uAge, uDead;

float h(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }

void main() {
    float aspect = uRes.x / uRes.y;
    vec2 uv = vUv;
    vec2 p = (vUv - 0.5) * vec2(aspect, 1.0);
    float line = floor(uv.y * uRes.y / 2.0);

    // Tape wobble.
    uv.x += sin(uv.y * 5.0 + uTime * 1.1) * uWow * 0.004;
    uv.x += (h(vec2(line, floor(uTime * 30.0))) - 0.5) * uFlutter * 0.004;
    float roll = fract(uTime * 0.06);
    float band = smoothstep(0.035, 0.0, abs(uv.y - roll)) * (0.25 + uWow);
    uv.x += band * 0.02 * (h(vec2(line, floor(uTime * 60.0))) - 0.3);

    // Heat haze: a shimmering column above the orb, on tape time.
    vec2 dm = p - uOrb;
    float above = smoothstep(uOrbScreenR * 0.3, uOrbScreenR * 1.0, dm.y) * exp(-max(dm.y - uOrbScreenR, 0.0) * 2.5);
    float column = exp(-pow(dm.x / (uOrbScreenR * 1.1), 2.0));
    float haze = above * column * uDrive;
    uv += (vec2(fbm2(vec2(p.x * 9.0, p.y * 5.0 - uWorld * 1.2)),
                fbm2(vec2(p.x * 7.0 + 3.0, p.y * 6.0 - uWorld))) - 0.5) * 0.014 * haze;

    vec3 c = texture(uTex, uv).rgb;
    vec4 ink = texture(uInk, uv);

    // Ink, in the water around the orb (not over it): it picks up the caustics.
    float outside = smoothstep(uOrbScreenR * 0.92, uOrbScreenR * 1.04, length(dm));
    float lum = dot(c, vec3(0.333));
    vec3 inkCol = vec3(0.46, 0.05, 0.17) * (0.55 + 1.6 * lum);
    c = mix(c, inkCol, clamp(ink.r, 0.0, 1.0) * 0.8 * outside);

    // Grime: algae and film creeping up from the bottom and in from the corners.
    float rr = length(p);
    float g = fbm2(vUv * vec2(aspect * 6.0, 6.0));
    float creep = smoothstep(0.4, 0.0, vUv.y) + smoothstep(0.6, 1.0, rr) * 0.7;
    float grime = smoothstep(0.6, 0.9, g + creep * 0.45) * uAge;
    float drips = smoothstep(0.7, 1.0, vnoise(vec3(vUv.x * 70.0, 0.0, 0.0))) * smoothstep(0.55, 0.05, vUv.y) * uAge;
    c = mix(c, vec3(0.1, 0.14, 0.05) + c * 0.35, clamp(grime * 0.8 + drips * 0.35, 0.0, 0.85));

    // Grease from your fingers: catches the light.
    c = mix(c, c * 1.15 + vec3(0.09, 0.1, 0.09), clamp(ink.g, 0.0, 1.0) * 0.55);

    c += band * 0.05;
    c *= 0.9 + 0.1 * sin(gl_FragCoord.y * 3.14159);
    c += (h(gl_FragCoord.xy + fract(uTime * 7.1) * 91.0) - 0.5) * (0.03 + 0.03 * uAge);

    // Fixation: when the tape stops, the formalin turns milky. The cloud is
    // fixed in shape (no time term) -- it advances from the edges with uDead.
    float billow = fbm2(p * vec2(4.0, 4.0) + 11.0);
    float edge = rr * 1.1 + (billow - 0.5) * 0.45;
    float front = 1.25 - uDead * 1.1;
    float fog = smoothstep(front - 0.14, front + 0.14, edge);
    c = mix(c, vec3(dot(c, vec3(0.333))), uDead * 0.6);
    vec3 milk = vec3(0.78, 0.8, 0.74) * (0.7 + 0.35 * billow) + c * 0.15;
    c = mix(c, milk, fog * 0.9);

    vec2 v = vUv - 0.5;
    c *= 1.0 - dot(v, v) * 0.9;
    frag = vec4(c, 1.0);
}
