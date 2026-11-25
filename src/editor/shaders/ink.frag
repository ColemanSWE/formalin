// Ink and fingerprints: a feedback buffer, ping-ponged every frame.
//   R = ink density in the water around the orb (the reverb, made visible)
//   G = grease on the front glass (where you've held the orb)
//
// Ink bleeds from the orb's rim in proportion to the output level, then
// drifts on a curl-noise current. It moves on *tape* time: it freezes when
// the tape stops and flows back into the orb when the tape runs backwards.

in vec2 vUv;
out vec4 frag;

uniform sampler2D uPrev;
uniform vec2 uRes;
uniform float uDt;        // wall-clock seconds this frame
uniform float uDtWorld;   // tape seconds this frame (negative in reverse)
uniform float uWorld;
uniform float uKeep;      // ink survival this frame (Decay)
uniform float uEmit;      // ink emission rate (level x wet/dry; negative = reabsorb)
uniform float uSmear;     // diffusion + swirl
uniform float uSmudgeKeep;// fingerprint survival this frame (Age)
uniform float uGrab;
uniform vec2 uOrb;
uniform float uOrbScreenR;
uniform vec2 uMouse;

void main() {
    float aspect = uRes.x / uRes.y;
    vec2 p = (vUv - 0.5) * vec2(aspect, 1.0);
    vec2 d = p - uOrb;
    float r = length(d) + 1e-4;

    // The current: curl of a noise field (divergence-free, so it swirls),
    // a slow rise, and a push away from the orb.
    float e = 0.02;
    vec3 nq = vec3(p * 2.2, uWorld * 0.08);
    float n0 = vnoise(nq);
    float nx = vnoise(nq + vec3(e * 2.2, 0.0, 0.0));
    float ny = vnoise(nq + vec3(0.0, e * 2.2, 0.0));
    vec2 curl = vec2(ny - n0, -(nx - n0)) / e;
    vec2 vel = curl * (0.04 + 0.12 * uSmear) + vec2(0.0, 0.03)
             + d / r * 0.05 * exp(-max(r - uOrbScreenR, 0.0) * 6.0);

    // Semi-Lagrangian advection: fetch whatever flowed into this pixel.
    vec2 buv = (p - vel * uDtWorld) / vec2(aspect, 1.0) + 0.5;
    vec2 tx = 1.0 / uRes;
    float ink = texture(uPrev, buv).r;
    float blur = 0.25 * (texture(uPrev, buv + vec2(tx.x, 0.0)).r + texture(uPrev, buv - vec2(tx.x, 0.0)).r
                       + texture(uPrev, buv + vec2(0.0, tx.y)).r + texture(uPrev, buv - vec2(0.0, tx.y)).r);
    float moving = min(abs(uDtWorld) / max(uDt, 1e-4), 1.0);
    ink = mix(ink, blur, (0.15 + 0.5 * uSmear) * moving) * uKeep;

    // Bleed from patches of the rim, which wander on tape time.
    float ang = atan(d.y, d.x);
    float rim = exp(-abs(r - uOrbScreenR * 1.02) * 70.0);
    float patchy = smoothstep(0.45, 0.8, vnoise(vec3(ang * 2.0, uWorld * 0.6, 3.0)));
    ink = clamp(ink + rim * patchy * uEmit * uDt * 6.0, 0.0, 3.0);

    // Fingerprints: whorled ridges pressed into the glass while you hold it.
    float smudge = texture(uPrev, vUv).g * uSmudgeKeep;
    vec2 m = p - uMouse;
    float fd = length(m * vec2(1.0, 0.8));
    float ridges = 0.5 + 0.5 * sin(fd * 520.0 + vnoise(vec3(m * 40.0, 1.0)) * 5.0);
    smudge = max(smudge, smoothstep(0.045, 0.02, fd) * ridges * uGrab * 0.9);

    frag = vec4(ink, smudge, 0.0, 1.0);
}
