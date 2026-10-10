precision highp float;

uniform sampler2D u_frame;   // per dot: rgb = its own colour (0 = the look's), a = brightness
uniform vec2 u_screen;       // framebuffer size in pixels
uniform int u_rotate;        // quarter turns (0..3) from panel to landscape
uniform float u_time;        // seconds
uniform float u_pitch;       // screen pixels per dot (a whole number)
uniform vec2 u_origin;       // the panel's top-left, in landscape pixels
uniform float u_progress;    // 0..1: time until the next scene; < 0 hides the line

// The look: the colour of a lit dot, and a palette by brightness.
uniform vec3 u_color;
uniform vec3 u_palette[16];
uniform float u_palette_weight;  // 0 = the colour, 1 = the palette

// The glass (see glass.rs).
uniform float u_dot_size;
uniform float u_roundness;
uniform float u_halo;
uniform float u_halo_width;
uniform float u_unlit;
uniform float u_variation;
uniform float u_flicker;
uniform float u_gamma;
uniform float u_hot_core;

out vec4 o_color;

const vec3 WHITE_HOT = vec3(1.0, 0.85, 0.6);  // bright dot centres lean to this
const float PROGRESS_TRACK = 0.05;            // progress line: unfilled part
const float PROGRESS_FILL = 0.30;             // progress line: filled part

float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

vec4 dot_at(ivec2 cell) {
    ivec2 size = textureSize(u_frame, 0);
    if (any(lessThan(cell, ivec2(0))) || any(greaterThanEqual(cell, size))) return vec4(0.0);
    return texelFetch(u_frame, cell, 0);
}

vec3 palette(float a) {
    float x = clamp(a, 0.0, 1.0) * 15.0;
    int i = int(floor(x));
    int j = min(i + 1, 15);
    return mix(u_palette[i], u_palette[j], x - float(i));
}

float maxc(vec3 c) {
    return max(c.r, max(c.g, c.b));
}

// A dot's lit colour, for brightness v (after gamma and gain), and how
// strong it is (for the white-hot core). A dot with its own colour uses
// it; the rest take the look's colour, or its palette.
vec3 lit(vec4 d, float v, float gain, out float strength) {
    if (d.r + d.g + d.b > 0.0) {
        strength = v * maxc(d.rgb);
        return d.rgb * v;
    }
    strength = v;
    vec3 tint = u_color * v;
    if (u_palette_weight <= 0.0) return tint;
    vec3 pal = palette(d.a) * gain;
    strength = mix(v, maxc(pal), u_palette_weight);
    return mix(tint, pal, u_palette_weight);
}

void main() {
    vec2 dmd = vec2(textureSize(u_frame, 0));

    // Panel pixel -> landscape pixel (origin top-left, y down).
    vec2 p = gl_FragCoord.xy;
    vec2 s = u_screen;
    vec2 q;
    if      (u_rotate == 0) q = vec2(p.x, s.y - p.y);
    else if (u_rotate == 1) q = vec2(p.y, p.x);
    else if (u_rotate == 2) q = vec2(s.x - p.x, p.y);
    else                    q = vec2(s.y - p.y, s.x - p.x);

    float pitch = u_pitch;
    vec2 origin = u_origin;
    vec2 d = (q - origin) / pitch;  // position in dot units
    vec3 UNLIT = u_color * u_unlit;

    if (any(lessThan(d, vec2(-1.0))) || any(greaterThan(d, dmd + 1.0))) {
        // Below the panel: a thin, quiet line that fills up until the
        // next scene.
        vec3 bar = vec3(0.0);
        float top = origin.y + dmd.y * pitch + pitch * 1.5;
        if (u_progress >= 0.0 && q.y >= top && q.y < top + 3.0 && q.x >= origin.x && q.x < origin.x + dmd.x * pitch) {
            float filled = step(q.x, origin.x + dmd.x * pitch * u_progress);
            bar = u_color * mix(PROGRESS_TRACK, PROGRESS_FILL, filled);
        }
        o_color = vec4(bar, 1.0);
        return;
    }

    ivec2 cell = ivec2(floor(d));
    vec2 f = fract(d) - 0.5;
    float r;
    if (u_roundness >= 1.0) {
        r = length(f);
    } else {
        // A rounded square: a superellipse between square and circle.
        float e = mix(10.0, 2.0, clamp(u_roundness, 0.0, 1.0));
        vec2 af = abs(f);
        r = pow(pow(af.x, e) + pow(af.y, e), 1.0 / e);
    }
    float aa = 1.0 / pitch;  // one screen pixel, in cell units
    bool on_grid = all(greaterThanEqual(cell, ivec2(0))) && all(lessThan(cell, ivec2(dmd)));

    // Every dot is a little different, and the plasma shimmers.
    vec2 id = vec2(cell);
    float gain = 1.0 - u_variation * hash(id);
    gain *= 1.0 - u_flicker * hash(id + floor(u_time * 60.0) * 0.731);

    vec4 here = dot_at(cell);
    float v = pow(here.a, u_gamma) * gain;
    float strength;
    vec3 body = lit(here, v, gain, strength);
    float inside = 1.0 - smoothstep(u_dot_size - aa, u_dot_size + aa, r);
    float core = pow(clamp(1.0 - r / u_dot_size, 0.0, 1.0), 2.0) * v * v;
    vec3 col = vec3(0.0);
    if (on_grid) col = inside * (UNLIT + mix(body, WHITE_HOT * strength, core * 0.7 * u_hot_core));

    // Tight halo from this dot and its 8 neighbours. Bloom adds the wide glow.
    vec3 halo = vec3(0.0);
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec4 n = dot_at(cell + ivec2(x, y));
            float dist = length(f - vec2(x, y));
            float unused;
            halo += lit(n, pow(n.a, u_gamma), 1.0, unused) * exp(-(dist * dist) / (u_halo_width * u_halo_width));
        }
    }
    col += halo * u_halo * (1.0 - inside * 0.5);

    o_color = vec4(col, 1.0);
}
