use bevy::asset::RenderAssetUsages;
use bevy::math::{Vec2, Vec3, vec2};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

pub fn build_terrain_mesh_with_skirts(
    get_elevation: &impl Fn(Vec2) -> f32,
    grid_resolution: u32,
    skirt_delta: f32,
) -> Mesh {
    let grid_resolution = grid_resolution.max(1);
    let vertices_per_side = grid_resolution + 1;
    let top_vertex_count = vertices_per_side * vertices_per_side;

    let mut positions = Vec::with_capacity((top_vertex_count + vertices_per_side * 4) as usize);
    let mut uvs = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity(
        (grid_resolution * grid_resolution * 6 + grid_resolution * 4 * 6) as usize,
    );
    let mut top_indices = Vec::with_capacity((grid_resolution * grid_resolution * 6) as usize);
    let mut skirt_indices = Vec::with_capacity((grid_resolution * 4 * 6) as usize);

    let top_index = |x: u32, y: u32| -> u32 { y * vertices_per_side + x };

    for y in 0..=grid_resolution {
        for x in 0..=grid_resolution {
            let uv = Vec2::new(
                x as f32 / grid_resolution as f32,
                y as f32 / grid_resolution as f32,
            );
            let local_xy = uv - Vec2::splat(0.5);
            positions.push([local_xy.x, local_xy.y, get_elevation(uv)]);
            uvs.push([uv.x, uv.y]);
        }
    }

    for y in 0..grid_resolution {
        for x in 0..grid_resolution {
            let a = top_index(x, y);
            let b = top_index(x + 1, y);
            let c = top_index(x + 1, y + 1);
            let d = top_index(x, y + 1);
            top_indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }

    let mut push_skirt_vertices = |top: u32| -> (u32, u32) {
        let [x, y, z] = positions[top as usize];
        let uv = uvs[top as usize];
        let top_index = positions.len() as u32;
        positions.push([x, y, z]);
        uvs.push(uv);
        let bottom_index = positions.len() as u32;
        positions.push([x, y, z - skirt_delta]);
        uvs.push(uv);
        (top_index, bottom_index)
    };

    let bottom = (0..=grid_resolution)
        .map(|x| push_skirt_vertices(top_index(x, 0)))
        .collect::<Vec<_>>();
    let right = (0..=grid_resolution)
        .map(|y| push_skirt_vertices(top_index(grid_resolution, y)))
        .collect::<Vec<_>>();
    let top = (0..=grid_resolution)
        .map(|x| push_skirt_vertices(top_index(x, grid_resolution)))
        .collect::<Vec<_>>();
    let left = (0..=grid_resolution)
        .map(|y| push_skirt_vertices(top_index(0, y)))
        .collect::<Vec<_>>();

    for i in 0..grid_resolution as usize {
        add_skirt_quad(
            &mut skirt_indices,
            bottom[i].0,
            bottom[i + 1].0,
            bottom[i].1,
            bottom[i + 1].1,
        );
        add_skirt_quad(
            &mut skirt_indices,
            right[i].0,
            right[i + 1].0,
            right[i].1,
            right[i + 1].1,
        );
        add_skirt_quad(
            &mut skirt_indices,
            top[i + 1].0,
            top[i].0,
            top[i + 1].1,
            top[i].1,
        );
        add_skirt_quad(
            &mut skirt_indices,
            left[i + 1].0,
            left[i].0,
            left[i + 1].1,
            left[i].1,
        );
    }

    let sample_step = (0.25 / grid_resolution.max(1) as f32).max(f32::EPSILON);

    let mut normals = vec![[0.0, 0.0, 1.0]; positions.len()];

    for y in 0..=grid_resolution {
        for x in 0..=grid_resolution {
            let uv = Vec2::new(
                x as f32 / grid_resolution as f32,
                y as f32 / grid_resolution as f32,
            );

            let normal = make_terrain_gradient_normal(get_elevation, uv, sample_step);
            normals[(y * (grid_resolution + 1) + x) as usize] = [normal.x, normal.y, normal.z];
        }
    }

    set_skirt_extension_normals(
        &mut normals,
        &bottom,
        (0..=grid_resolution).map(|x| top_index(x, 0)),
    );
    set_skirt_extension_normals(
        &mut normals,
        &right,
        (0..=grid_resolution).map(|y| top_index(grid_resolution, y)),
    );
    set_skirt_extension_normals(
        &mut normals,
        &top,
        (0..=grid_resolution).map(|x| top_index(x, grid_resolution)),
    );
    set_skirt_extension_normals(
        &mut normals,
        &left,
        (0..=grid_resolution).map(|y| top_index(0, y)),
    );

    indices.extend_from_slice(&top_indices);
    indices.extend_from_slice(&skirt_indices);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn add_skirt_quad(indices: &mut Vec<u32>, top_a: u32, top_b: u32, skirt_a: u32, skirt_b: u32) {
    indices.extend_from_slice(&[top_a, skirt_a, skirt_b, top_a, skirt_b, top_b]);
}

fn set_skirt_extension_normals(
    normals: &mut [[f32; 3]],
    side: &[(u32, u32)],
    source_indices: impl Iterator<Item = u32>,
) {
    for (&(top, bottom), source) in side.iter().zip(source_indices) {
        let normal = normals[source as usize];
        normals[top as usize] = normal;
        normals[bottom as usize] = normal;
    }
}

fn make_terrain_gradient_normal(
    get_elevation: &impl Fn(Vec2) -> f32,
    uv: Vec2,
    sample_step: f32,
) -> Vec3 {
    let left = vec2((uv.x - sample_step).max(0.0), uv.y);
    let right = vec2((uv.x + sample_step).min(1.0), uv.y);
    let bottom = vec2(uv.x, (uv.y - sample_step).max(0.0));
    let top = vec2(uv.x, (uv.y + sample_step).min(1.0));

    let dx = (right.x - left.x).max(f32::EPSILON);
    let dy = (top.y - bottom.y).max(f32::EPSILON);
    let dz_dx = (get_elevation(right) - get_elevation(left)) / dx;
    let dz_dy = (get_elevation(top) - get_elevation(bottom)) / dy;

    Vec3::new(-dz_dx, -dz_dy, 1.0).normalize_or(Vec3::Z)
}
