#[derive(Clone, Debug)]
pub struct DEMData {
    pub data: Vec<u32>,
    pub stride: u32,
    pub dim: u32,
    pub min: f64,
    pub max: f64,
    pub red_factor: f64,
    pub green_factor: f64,
    pub blue_factor: f64,
    pub base_shift: f64,
}

impl DEMData {
    pub fn get(&self, x: i32, y: i32) -> Option<f32> {
        let index = self.idx(x, y)?;
        let pixel = *self.data.get(index)?;
        let r = (pixel & 0xff) as f64;
        let g = ((pixel >> 8) & 0xff) as f64;
        let b = ((pixel >> 16) & 0xff) as f64;

        Some(self.unpack(r, g, b) as f32)
    }

    pub fn get_unpack_vector(&self) -> [f64; 4] {
        [
            self.red_factor,
            self.green_factor,
            self.blue_factor,
            self.base_shift,
        ]
    }

    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        let dim = self.dim as i32;
        if x < -1 || x > dim || y < -1 || y > dim {
            return None;
        }

        Some(((y + 1) as u32 * self.stride + (x + 1) as u32) as usize)
    }

    pub fn unpack(&self, r: f64, g: f64, b: f64) -> f64 {
        r * self.red_factor + g * self.green_factor + b * self.blue_factor - self.base_shift
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PackedDEMData {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn pack_dem_data(v: f64, unpack_vector: [f64; 4]) -> PackedDEMData {
    let red_factor = unpack_vector[0];
    let green_factor = unpack_vector[1];
    let blue_factor = unpack_vector[2];
    let base_shift = unpack_vector[3];
    let min_scale = red_factor.min(green_factor).min(blue_factor);
    let v_scaled = ((v + base_shift) / min_scale).round();

    PackedDEMData {
        r: ((v_scaled * min_scale / red_factor).floor() as u32 % 256) as u8,
        g: ((v_scaled * min_scale / green_factor).floor() as u32 % 256) as u8,
        b: ((v_scaled * min_scale / blue_factor).floor() as u32 % 256) as u8,
    }
}
