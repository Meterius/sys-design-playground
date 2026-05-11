use crate::app::maplibre_gl_js::utils::dem_data::DEMData;
use bevy::math::{DMat4, IVec2, Vec2, Vec4Swizzles, vec4};

pub const EXTENT: u32 = 8192;

#[derive(Clone, Debug)]
pub struct TerrainData {
    pub dem_data: DEMData,
    pub terrain_matrix: DMat4,
}

pub fn get_dem_elevation(data: &TerrainData, pos: Vec2) -> Option<f32> {
    let pos = pos * EXTENT as f32;
    let pos = (data.terrain_matrix.as_mat4() * vec4(pos.x, pos.y, 0., 1.)).xy();

    let coord = pos * data.dem_data.dim as f32;

    // bilinear interpolation

    let c = coord.floor();
    let t = coord - c;

    let c = c.as_ivec2().clamp(
        IVec2::ZERO,
        IVec2::new(EXTENT as i32 - 1, EXTENT as i32 - 1),
    );
    let cn = (c + IVec2::ONE).clamp(IVec2::ZERO, IVec2::splat(data.dem_data.dim as i32));

    Some(
        data.dem_data.get(c.x, c.y)? * (1. - t.x) * (1. - t.y)
            + data.dem_data.get(cn.x, c.y)? * (t.x) * (1. - t.y)
            + data.dem_data.get(c.x, cn.y)? * (1. - t.x) * (t.y)
            + data.dem_data.get(cn.x, cn.y)? * (t.x) * (t.y),
    )
}
