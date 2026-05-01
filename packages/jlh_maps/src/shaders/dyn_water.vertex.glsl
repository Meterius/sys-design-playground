precision highp float;

attribute vec2 a_pos;
uniform mat4 u_matrix;
uniform vec2 u_center;
uniform mat4 u_projection_matrix;
uniform vec4 u_projection_tile_mercator_coords;

varying vec2 v_world;

void main() {
    vec2 mercator_coords = u_projection_tile_mercator_coords.xy + a_pos * u_projection_tile_mercator_coords.zw;
    v_world = mod(mercator_coords, 0.01) * 10000000.0;

    gl_Position = u_projection_matrix * vec4(a_pos, 0.0, 1.0);
}
