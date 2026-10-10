precision mediump float;
// 9-tap Gaussian blur in one direction.
uniform sampler2D u_src;
uniform vec2 u_step;  // one pixel along the blur direction, in uv
in vec2 v_uv;
out vec4 o_color;
const float W[5] = float[](0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
void main() {
    vec3 c = texture(u_src, v_uv).rgb * W[0];
    for (int i = 1; i < 5; i++) {
        c += texture(u_src, v_uv + u_step * float(i)).rgb * W[i];
        c += texture(u_src, v_uv - u_step * float(i)).rgb * W[i];
    }
    o_color = vec4(c, 1.0);
}
