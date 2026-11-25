#version 150
// Phosphor burn-in: bright things leave trails that fade at `uPersist` per frame.
// Driven by the reverb's Smear, so the picture smears when the sound does.
in vec2 vUv;
out vec4 frag;
uniform sampler2D uScene;
uniform sampler2D uPrev;
uniform float uPersist;
void main() {
    vec3 s = texture(uScene, vUv).rgb;
    vec3 p = texture(uPrev, vUv).rgb * uPersist;
    frag = vec4(max(s, p), 1.0);
}
