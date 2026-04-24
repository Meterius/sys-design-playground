use crate::model::parser::{
    FromStrParser, I32Parser, OptionParser, Parser, RecordExt, RecordParseError, ShapefileElement,
    StringParser, TfBoolParser, U32Parser,
};
#[cfg(feature = "bevy-reflect")]
use bevy_reflect::Reflect;
use generated_queries::types as sql_types;
use geo_types::{Geometry, MultiPolygon};
use glam::DVec2;
use shapefile::dbase::FieldValue;
use strum::{AsRefStr, Display, EnumString};
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr)]
#[cfg_attr(feature = "bevy-reflect", derive(Reflect))]
#[strum(serialize_all = "snake_case")]
pub enum WaterClass {
    Water,
    Reservoir,
    River,
    Dock,
    Glacier,
    Riverbank,
    Wetland,
    WetlandWetMeadow,
    WetlandFen,
    WetlandSwamp,
    WetlandReedbed,
    WetlandTidalflat,
    WetlandMangrove,
    WetlandMarsh,
    WetlandSaltmarsh,
    WetlandBog,
}

#[derive(Debug, Clone)]
pub struct Water {
    pub osm_id: i64,
    pub class: WaterClass,
    pub geometry: MultiPolygon<f64>,
}

impl ShapefileElement for Water {
    fn id(&self) -> i64 {
        self.osm_id
    }

    fn from_shapefile_item(
        (shape, record): (shapefile::Shape, &shapefile::dbase::Record),
    ) -> Result<Self, RecordParseError> {
        let shape_type = shape.shapetype();

        let geometry = Geometry::<f64>::try_from(shape)?;
        let geometry = match geometry {
            Geometry::Polygon(polygon) => MultiPolygon::from(polygon),
            Geometry::MultiPolygon(polygon) => polygon,
            _ => return Err(RecordParseError::UnexpectedShapeType { shape_type }),
        };

        if let Some(poly) = geometry.iter().find(|p| {
            p.exterior().points().count() < 4
                || p.interiors().iter().any(|r| r.points().count() < 4)
        }) {
            return Err(RecordParseError::InvalidShape {
                message: format!("Polygon with less than 4 points: {poly:?}"),
            });
        }

        Ok(Self {
            osm_id: record.required::<FromStrParser<i64>>("osm_id")?,
            class: record.required::<FromStrParser<WaterClass>>("fclass")?,
            geometry,
        })
    }
}
