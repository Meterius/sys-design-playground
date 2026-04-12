use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use smallvec::SmallVec;
use strum::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Projection {
    #[serde(rename_all = "camelCase")]
    BoundedMercator { min_lat: f64, max_lat: f64 },
}

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum TileSubKey {
    TopLeft = 3,
    TopRight = 2,
    BottomLeft = 1,
    BottomRight = 0,
}

pub type TileKey = SmallVec<[TileSubKey; 10]>;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Display, EnumString)]
pub enum GibsLayer {
    LayerModisTerraCorrectedReflectanceTrueColor,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Display, EnumString)]
pub enum LocalLayer {
    GlobalMosaicSen2,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Layer {
    Gibs(GibsLayer),
    SenHub,
    Local(LocalLayer),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTileRequest {
    pub layer: Layer,
    pub projection: Projection,
    pub tile_key: TileKey,
}
