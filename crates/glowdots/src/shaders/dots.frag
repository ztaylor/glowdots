precision highp float;

uniform sampler2D u_frame;  // DMD dots, R8, 0 = off, 1 = full brightness
uniform vec2 u_screen;      // physical framebuffer size in pixels
uniform int u_rotate;       // quarter turns (0..3) from panel to landscape
uniform float u_time;       // seconds
uniform vec3 u_color;       // color of a fully lit dot (the scene's look)
uniform float u_progress;   // 0..1: time until the next scene; < 0 hides it

out vec4 o_color;

// ---- Look ---------------------------------------------------------------
// The dot color comes from u_color. These are relative to it.
const float UNLIT_GLOW = 0.065;                 // dark dots stay faintly visible
const vec3 WHITE_HOT   = vec3(1.0, 0.85, 0.6);  // bright dot centers lean to white
const float PROGRESS_TRACK = 0.05;              // progress line: unfilled part
const float PROGRESS_FILL  = 0.30;              // progress line: filled part
const float DOT_RADIUS = 0.43;                  // in cell units (0.5 = touching)
const float HALO       = 0.12;                  // tight glow around each dot
const float HALO_WIDTH = 0.45;                  // halo falloff, in cell units
const float DOT_SPREAD = 0.18;                  // dot-to-dot brightness spread (old panel)
const float FLICKER    = 0.05;                  // fast plasma shimmer
const float GAMMA      = 1.6;                   // spreads the 4 shades apart
// -------------------------------------------------------------------------

float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float level(ivec2 cell) {
    ivec2 size = textureSize(u_frame, 0);
    if (any(lessThan(cell, ivec2(0))) || any(greaterThanEqual(cell, size))) return 0.0;
    return pow(texelFetch(u_frame, cell, 0).r, GAMMA);
}

void main() {
    vec2 dmd = vec2(textureSize(u_frame, 0));

    // Panel pixel -> landscape pixel (origin top-left, y down).
    vec2 p = gl_FragCoord.xy;
    vec2 s = u_screen;
    vec2 q, land;
    if      (u_rotate == 0) { q = vec2(p.x, s.y - p.y);       land = s; }
    else if (u_rotate == 1) { q = vec2(p.y, p.x);             land = s.yx; }
    else if (u_rotate == 2) { q = vec2(s.x - p.x, p.y);       land = s; }
    else                    { q = vec2(s.y - p.y, s.x - p.x); land = s.yx; }

    // Whole pixels per dot, so every dot is the same shape.
    float pitch = floor(min(land.x / dmd.x, land.y / dmd.y));
    vec2 origin = floor((land - dmd * pitch) * 0.5);
    vec2 d = (q - origin) / pitch;  // position in dot units

    vec3 PLASMA = u_color;
    vec3 HOT_CORE = mix(u_color, WHITE_HOT, 0.6);
    vec3 UNLIT = u_color * UNLIT_GLOW;

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
    float r = length(f);
    float aa = 1.0 / pitch;  // one screen pixel, in cell units
    bool on_grid = all(greaterThanEqual(cell, ivec2(0))) && all(lessThan(cell, ivec2(dmd)));

    // Every dot is a little different, and the plasma shimmers.
    vec2 id = vec2(cell);
    float gain = 1.0 - DOT_SPREAD * hash(id);
    gain *= 1.0 - FLICKER * hash(id + floor(u_time * 60.0) * 0.731);

    float v = level(cell) * gain;
    float inside = 1.0 - smoothstep(DOT_RADIUS - aa, DOT_RADIUS + aa, r);
    float core = pow(clamp(1.0 - r / DOT_RADIUS, 0.0, 1.0), 2.0) * v * v;
    vec3 col = vec3(0.0);
    if (on_grid) col = inside * (UNLIT + mix(PLASMA, HOT_CORE, core * 0.7) * v);

    // Tight halo from this dot and its 8 neighbours. Bloom adds the wide glow.
    float halo = 0.0;
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            float dist = length(f - vec2(x, y));
            halo += level(cell + ivec2(x, y)) * exp(-(dist * dist) / (HALO_WIDTH * HALO_WIDTH));
        }
    }
    col += PLASMA * halo * HALO * (1.0 - inside * 0.5);

    o_color = vec4(col, 1.0);
}
