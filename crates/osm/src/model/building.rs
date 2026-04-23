use crate::model::parser::{
    FromStrParser, RecordExt, RecordParseError, ShapefileElement, StringParser,
};
use geo_types::{Geometry, MultiPolygon};

#[derive(Clone)]
pub struct Building {
    pub osm_id: i64,
    pub kind: Option<String>,
    pub geometry: MultiPolygon<f64>,
}

impl ShapefileElement for Building {
    fn from_shapefile_item(
        (shape, record): (shapefile::Shape, &shapefile::dbase::Record),
    ) -> Result<Self, RecordParseError> {
        let shape_type = shape.shapetype();

        let geometry = geo_types::Geometry::<f64>::try_from(shape)?;
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
            kind: record.optional::<StringParser>("type")?,
            geometry,
        })
    }
}
