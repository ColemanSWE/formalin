// The tank and the orb. (gl.rs prepends #version and common.glsl.)
//
// Screen space `p`: origin at the center, y up, units of window height.
// The camera sits at z = -3 looking down +z; the orb lives around z = 0.
// The creature inside is raymarched in the orb's *local* frame, so spinning
// the orb (uRot) visibly turns it.

in vec2 vUv;
out vec4 frag;

uniform vec2 uRes;
uniform float uTime;   // wall clock
uniform float uWorld;  // tape time: slows, stops and reverses with the transport
uniform mat3 uRot;     // orb local -> world
uniform vec2 uOrb;     // orb center, screen space
uniform float uOrbR;   // orb radius, world units
uniform float uOrbScreenR;
uniform vec2 uMouse;   // screen space
uniform float uGrab, uSqueeze, uWow, uFlutter, uAge, uLevel, uDead;
// (fingerprints now live in the ink pass, on the glass in front of everything)

const float CAM_Z = -3.0;
const float FOCAL = 1.5;
const vec3 LIGHT = vec3(-0.45, 0.8, -0.4);

float smin(float a, float b, float k) {
    float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// ---------------------------------------------------------------- the tank
// Murky specimen water: caustics from above, light shafts, drifting particles.
// Everything moves on tape time, so the whole world stalls when the tape does.
vec3 tank(vec3 rd) {
    vec2 q = rd.xy / max(rd.z, 0.25) * FOCAL;
    float y = q.y;
    vec3 col = mix(vec3(0.012, 0.04, 0.05), vec3(0.03, 0.15, 0.16), smoothstep(-0.7, 0.35, y));
    col = mix(col, vec3(0.42, 0.82, 0.8), smoothstep(0.3, 0.95, y) * 0.5);

    vec2 cq = q * vec2(3.2, 5.0);
    float c1 = 1.0 - abs(vnoise(vec3(cq, uWorld * 0.35)) * 2.0 - 1.0);
    float c2 = 1.0 - abs(vnoise(vec3(cq * 1.9 + 4.0, uWorld * 0.5)) * 2.0 - 1.0);
    float caus = pow(c1, 8.0) + 0.6 * pow(c2, 10.0);
    col += vec3(0.3, 0.62, 0.58) * caus * smoothstep(-0.6, 0.6, y) * 0.5;

    float shaft = vnoise(vec3(q.x * 2.2 + q.y * 0.6, 0.0, uWorld * 0.07));
    col += vec3(0.2, 0.42, 0.42) * pow(max(shaft - 0.4, 0.0), 1.6) * smoothstep(-0.5, 0.9, y);

    float snow = 0.0;
    for (int L = 0; L < 3; L++) {
        float fl = float(L);
        float sc = 9.0 + fl * 8.0;
        vec2 g = q * sc + vec2(fl * 7.3, -uWorld * (0.35 + 0.25 * fl));
        vec2 id = floor(g), f = fract(g) - 0.5;
        vec2 o = vec2(hash21(id + 3.7), hash21(id + 9.1)) - 0.5;
        float d = length(f - o * 0.6);
        snow += smoothstep(0.08, 0.0, d) * step(0.8, hash21(id + fl * 13.1))
              * (0.35 + 0.4 * hash21(id + 1.3)) / (1.0 + fl * 0.6);
    }
    // Holding the orb turns it into a magnet: the drifting particles snap into
    // the field lines of a dipole around it, like iron filings. Field lines of a
    // vertical dipole satisfy r = L * sin^2(theta), so constant log(L) traces one.
    vec2 dm = q - uOrb;
    float r = length(dm) + 1e-4;
    float near = exp(-max(r - uOrbScreenR, 0.0) * 4.0) * smoothstep(uOrbScreenR * 0.95, uOrbScreenR * 1.05, r);
    float magnet = uGrab * near;
    float sinT = abs(dm.x) / r;
    float L = r / max(sinT * sinT, 1e-3);
    float onLine = smoothstep(0.14, 0.0, abs(fract(log(L) * 5.0) - 0.5));
    float filings = onLine * step(0.5, hash21(floor(q * 240.0))) * (0.5 + 0.5 * hash21(floor(q * 240.0) + 7.0));
    col += vec3(0.75, 0.95, 0.9) * snow * (1.0 - magnet);
    col += vec3(0.62, 0.86, 0.82) * filings * magnet * 0.8;

    // Drive heats the orb: past ~35% it boils, shedding bubbles that rise on tape time.
    float boil = smoothstep(0.35, 1.0, uSqueeze);
    if (boil > 0.0) {
        vec2 g = dm * 30.0 + vec2(0.0, -uWorld * 4.0);
        vec2 id = floor(g), f = fract(g) - 0.5;
        vec2 o = (vec2(hash21(id + 1.0), hash21(id + 2.0)) - 0.5) * 0.4;
        float ring = smoothstep(0.06, 0.0, abs(length(f - o) - (0.1 + 0.18 * hash21(id + 5.0))));
        float column = exp(-abs(dm.x) / (uOrbScreenR * 0.6)) * smoothstep(uOrbScreenR * 0.5, uOrbScreenR * 1.1, dm.y)
                     * exp(-max(dm.y - uOrbScreenR, 0.0) * 2.0);
        col += vec3(0.8, 0.95, 0.95) * ring * step(1.0 - boil * 0.6, hash21(id + 9.0)) * column * 0.7;
    }

    // Age: the water goes murky and yellow, like an old specimen jar.
    col = mix(col, vec3(0.15, 0.14, 0.05) + col * 0.45, uAge * 0.5);
    return col;
}

// ---------------------------------------------------------------- the creature
vec3 organelle(int i) {
    float fi = float(i);
    vec3 c = vec3(sin(fi * 2.4 + 0.3), cos(fi * 1.7 + 1.1) * 0.6, sin(fi * 3.1 + 2.0)) * 0.42;
    return c + 0.03 * sin(vec3(1.3, 1.7, 1.1) * uWorld + fi);
}

// Signed distance in the orb's local unit space. `mat` = 0 body, 1 organelle.
float creature(vec3 q, out float mat) {
    // It breathes and pulses with the output -- until it's fixed in place.
    float beat = (0.05 * uLevel + 0.012 * sin(uTime * 2.6)) * (1.0 - uDead);
    float swell = 1.0 + 0.4 * uSqueeze;
    float body = length(q * vec3(1.0, 1.15, 1.0)) - 0.33 * swell - beat;
    body += (fbm(q * 3.2 + vec3(0.0, uWorld * 0.25, 0.0)) - 0.5) * (0.16 + 0.14 * uFlutter);

    float org = 1e9;
    for (int i = 0; i < 5; i++) {
        org = min(org, length(q - organelle(i)) - (0.065 + 0.02 * float(i - 2 * (i / 2))));
    }

    // Filaments trailing off the body, waving on tape time.
    vec3 t = q;
    t.y += 0.06 * sin(t.x * 9.0 + uWorld * 2.0);
    float fil = length(t.yz) - 0.018 * smoothstep(0.8, 0.0, abs(t.x));
    vec3 t2 = q;
    t2.x += 0.06 * sin(t2.z * 8.0 - uWorld * 1.7);
    fil = min(fil, length(t2.xy) - 0.016 * smoothstep(0.8, 0.0, abs(t2.z)));

    float d = smin(body, fil, 0.05);
    mat = org < d ? 1.0 : 0.0;
    return smin(d, org, 0.12);
}
float creature(vec3 q) { float m; return creature(q, m); }

vec3 creatureNormal(vec3 q) {
    vec2 e = vec2(0.004, 0.0);
    return normalize(vec3(creature(q + e.xyy) - creature(q - e.xyy),
                          creature(q + e.yxy) - creature(q - e.yxy),
                          creature(q + e.yyx) - creature(q - e.yyx)));
}

// ---------------------------------------------------------------- the orb
vec2 hitEllipsoid(vec3 ro, vec3 rd, vec3 c, vec3 ax) {
    vec3 o = (ro - c) / ax, d = rd / ax;
    float a = dot(d, d), b = dot(o, d), cc = dot(o, o) - 1.0;
    float disc = b * b - a * cc;
    if (disc < 0.0) return vec2(-1.0);
    float s = sqrt(disc);
    return vec2((-b - s) / a, (-b + s) / a);
}

void main() {
    vec2 p = (vUv - 0.5) * vec2(uRes.x / uRes.y, 1.0);
    vec3 ro = vec3(0.0, 0.0, CAM_Z);
    vec3 rd = normalize(vec3(p, FOCAL));
    vec3 L = normalize(LIGHT);

    vec3 C = vec3(uOrb * (-CAM_Z) / FOCAL, 0.0);
    // Squeeze flattens it; wow makes it breathe.
    float breathe = 1.0 + uWow * 0.025 * sin(uTime * 2.1);
    vec3 ax = uOrbR * breathe * vec3(1.0 + 0.16 * uSqueeze, 1.0 - 0.2 * uSqueeze, 1.0 + 0.16 * uSqueeze);

    vec3 col = tank(rd);

    // Glow the creature casts into the water around the orb.
    float edge = length(p - uOrb) - uOrbScreenR;
    col += vec3(1.0, 0.35, 0.45) * (0.05 + 0.2 * uLevel) * exp(-max(edge, 0.0) * 9.0) * (1.0 - uDead);

    vec2 t = hitEllipsoid(ro, rd, C, ax);
    if (t.x > 0.0) {
        vec3 pw = ro + rd * t.x;
        vec3 n = normalize((pw - C) / (ax * ax));
        n = normalize(n + uWow * 0.06 * vec3(sin(pw.y * 7.0 + uTime * 2.0), 0.0, cos(pw.x * 6.0 + uTime * 1.6)));
        float cosT = clamp(dot(-rd, n), 0.0, 1.0);
        float F = 0.04 + 0.96 * pow(1.0 - cosT, 5.0);
        mat3 toLocal = transpose(uRot);

        // March the creature in local unit space.
        vec3 lo = toLocal * ((ro - C) / ax);
        vec3 ld = toLocal * (rd / ax);
        float speed = length(ld);
        float tt = t.x;
        float hit = -1.0, mat = 0.0;
        for (int i = 0; i < 56; i++) {
            if (tt > t.y) break;
            float m;
            float d = creature(lo + ld * tt, m);
            if (d < 0.002) { hit = tt; mat = m; break; }
            tt += max(d * 0.8 / speed, 0.002);
        }

        vec3 inside;
        float pathLen;
        if (hit > 0.0) {
            vec3 q = lo + ld * hit;
            vec3 cn = creatureNormal(q);
            vec3 Ll = normalize(toLocal * L);
            vec3 vd = normalize(ld);
            float diff = max(dot(cn, Ll), 0.0);
            float rim = pow(clamp(1.0 + dot(cn, vd), 0.0, 1.0), 2.0);
            float occ = clamp(0.4 + 0.6 * creature(q + cn * 0.08) / 0.08, 0.0, 1.0);
            vec3 flesh = mix(vec3(1.0, 0.56, 0.46), vec3(1.0, 0.16, 0.2), uSqueeze);
            vec3 organ = vec3(0.35, 0.95, 0.78);
            vec3 base = mix(flesh, organ, mat);
            float mottle = fbm(q * 9.0);
            inside = vec3(0.2, 0.02, 0.07) + base * (0.22 + 0.78 * diff) * occ * (0.75 + 0.5 * mottle);
            inside += mix(vec3(1.0, 0.55, 0.65), vec3(0.6, 1.0, 0.9), mat) * rim * 0.4;
            inside += base * uLevel * 0.25;
            inside = mix(inside, vec3(dot(inside, vec3(0.33))) * 0.6, uDead * 0.8);
            pathLen = hit - t.x;
        } else {
            inside = tank(refract(rd, n, 1.0 / 1.33)) * 1.1;
            pathLen = t.y - t.x;
        }

        // The fluid: aqua absorption, clouding with age.
        vec3 absorb = exp(-pathLen * vec3(0.9, 0.3, 0.38) * (0.6 + 1.6 * uAge));
        inside = inside * absorb + vec3(0.35, 0.36, 0.2) * uAge * (1.0 - exp(-pathLen * 2.0));

        // Glass: thin-film interference that spins with the orb, plus Aero gloss.
        vec3 nl = toLocal * n;
        float th = 0.55 + 0.45 * vnoise(nl * 2.5 + uTime * 0.04);
        vec3 film = 0.5 + 0.5 * cos(6.28318 * (th * 2.2 + (1.0 - cosT) * 1.3 + vec3(0.0, 0.33, 0.67)));
        vec3 refl = tank(reflect(rd, n));
        col = mix(inside, refl * (0.6 + 0.8 * film), F);
        col += film * pow(1.0 - cosT, 2.5) * 0.5;
        float gloss = smoothstep(0.35, 0.95, n.y * 0.92 - n.x * 0.32) * smoothstep(0.05, 0.5, cosT);
        col += vec3(1.0) * gloss * 0.26;
        col += pow(max(dot(reflect(rd, n), L), 0.0), 90.0) * 1.1;
    }

    frag = vec4(col, 1.0);
}
