precision mediump float;
// Shrink the scene for bloom: 4 bilinear taps average a 4x4 block.
uniform sampler2D u_src;
uniform vec2 u_texel;  // one source pixel, in uv
in vec2 v_uv;
out vec4 o_color;
void main() {
    vec3 c = texture(u_src, v_uv + u_texel * vec2(-1.0, -1.0)).rgb
           + texture(u_src, v_uv + u_texel * vec2( 1.0, -1.0)).rgb
           + texture(u_src, v_uv + u_texel * vec2(-1.0,  1.0)).rgb
           + texture(u_src, v_uv + u_texel * vec2( 1.0,  1.0)).rgb;
    o_color = vec4(c * 0.25, 1.0);
}
