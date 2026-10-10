precision mediump float;
// Sharp dots plus the wide, soft bloom.
uniform sampler2D u_scene;
uniform sampler2D u_bloom;
uniform float u_bloom_strength;
in vec2 v_uv;
out vec4 o_color;
void main() {
    vec3 c = texture(u_scene, v_uv).rgb + texture(u_bloom, v_uv).rgb * u_bloom_strength;
    o_color = vec4(c, 1.0);
}
