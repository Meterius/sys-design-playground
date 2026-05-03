precision mediump float;

attribute vec2 a_pos;
uniform mat4 u_matrix;
uniform vec2 u_center;
uniform mat4 u_projection_matrix;
uniform vec4 u_projection_tile_mercator_coords;

uniform bool u_has_terrain;
uniform sampler2D u_terrain;
uniform float u_terrain_dim;
uniform mat4 u_terrain_matrix;
uniform vec4 u_terrain_unpack;
uniform float u_terrain_exaggeration;

varying vec2 v_world;
varying vec2 v_tile_pos;

float ele(vec2 pos) {
    vec4 rgb = (texture2D(u_terrain, pos) * 255.0) * u_terrain_unpack;
    return rgb.r + rgb.g + rgb.b - u_terrain_unpack.a;
}

float get_elevation(vec2 pos) {
    if (!u_has_terrain) {
        return 0.0;
    }

    vec2 coord = (u_terrain_matrix * vec4(pos, 0.0, 1.0)).xy * u_terrain_dim + 1.0;
    vec2 f = fract(coord);
    vec2 c = (floor(coord) + 0.5) / (u_terrain_dim + 2.0);
    float d = 1.0 / (u_terrain_dim + 2.0);
    float tl = ele(c);
    float tr = ele(c + vec2(d, 0.0));
    float bl = ele(c + vec2(0.0, d));
    float br = ele(c + vec2(d, d));
    float elevation = mix(mix(tl, tr, f.x), mix(bl, br, f.x), f.y);
    return elevation * u_terrain_exaggeration;
}

void main() {
    vec2 mercator_coords = u_projection_tile_mercator_coords.xy + a_pos * u_projection_tile_mercator_coords.zw;
    v_world = mod(mercator_coords, 0.005) * 10000000.0;
    v_tile_pos = a_pos / 8192.0;

    gl_Position = u_projection_matrix * vec4(a_pos, get_elevation(a_pos), 1.0);
}
