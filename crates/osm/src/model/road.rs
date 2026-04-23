use crate::model::parser::{
    FromStrParser, I32Parser, OptionParser, Parser, RecordExt, RecordParseError, ShapefileElement,
    StringParser, TfBoolParser, U32Parser,
};
#[cfg(feature = "bevy-reflect")]
use bevy_reflect::Reflect;
use generated_queries::types as sql_types;
use glam::DVec2;
use shapefile::dbase::FieldValue;
use strum::{AsRefStr, Display, EnumString};
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "bevy-reflect", derive(Reflect))]
pub enum OneWay {
    ForwardsOnly,
    BackwardsOnly,
    Bidirectional,
}

pub struct OneWayParser;

impl Parser for OneWayParser {
    type Output = OneWay;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        match value {
            FieldValue::Character(Some(v)) => match v.trim() {
                "F" => Ok(OneWay::ForwardsOnly),
                "T" => Ok(OneWay::BackwardsOnly),
                "B" | "" => Ok(OneWay::Bidirectional),
                other => Err(RecordParseError::InvalidField {
                    field,
                    value: other.to_string(),
                }),
            },
            FieldValue::Character(None) => Ok(OneWay::Bidirectional),
            other => Err(RecordParseError::UnexpectedType {
                field,
                value: format!("{other:?}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr)]
#[cfg_attr(feature = "bevy-reflect", derive(Reflect))]
pub enum RoadClassCategory {
    /// 511x
    #[strum(serialize = "major_roads")]
    MajorRoads,
    /// 512x
    #[strum(serialize = "minor_roads")]
    MinorRoads,
    /// 513x
    #[strum(serialize = "highway_links")]
    HighwayLinks,
    /// 514x
    #[strum(serialize = "very_small_roads")]
    VerySmallRoads,
    /// 515x
    #[strum(serialize = "paths_unsuitable_for_cars")]
    PathsUnsuitableForCars,
    /// 5199
    #[strum(serialize = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr)]
#[cfg_attr(feature = "bevy-reflect", derive(Reflect))]
pub enum RoadClass {
    // 511x Major roads
    #[strum(serialize = "motorway")]
    Motorway,
    #[strum(serialize = "trunk")]
    Trunk,
    #[strum(serialize = "primary")]
    Primary,
    #[strum(serialize = "secondary")]
    Secondary,
    #[strum(serialize = "tertiary")]
    Tertiary,

    // 512x Minor roads
    #[strum(serialize = "unclassified")]
    Unclassified,
    #[strum(serialize = "residential")]
    Residential,
    #[strum(serialize = "living_street")]
    LivingStreet,
    #[strum(serialize = "pedestrian")]
    Pedestrian,
    #[strum(serialize = "busway")]
    Busway,

    // 513x Highway links
    #[strum(serialize = "motorway_link")]
    MotorwayLink,
    #[strum(serialize = "trunk_link")]
    TrunkLink,
    #[strum(serialize = "primary_link")]
    PrimaryLink,
    #[strum(serialize = "secondary_link")]
    SecondaryLink,
    #[strum(serialize = "tertiary_link")]
    TertiaryLink,

    // 514x Very small roads
    #[strum(serialize = "service")]
    Service,
    #[strum(serialize = "track")]
    Track,
    #[strum(serialize = "track_grade1")]
    TrackGrade1,
    #[strum(serialize = "track_grade2")]
    TrackGrade2,
    #[strum(serialize = "track_grade3")]
    TrackGrade3,
    #[strum(serialize = "track_grade4")]
    TrackGrade4,
    #[strum(serialize = "track_grade5")]
    TrackGrade5,

    // 515x Paths unsuitable for cars
    #[strum(serialize = "bridleway")]
    Bridleway,
    #[strum(serialize = "cycleway")]
    Cycleway,
    #[strum(serialize = "footway")]
    Footway,
    #[strum(serialize = "path")]
    Path,
    #[strum(serialize = "steps")]
    Steps,

    // 5199 Unknown
    #[strum(serialize = "unknown")]
    Unknown,
}

impl RoadClass {
    pub const fn category(self) -> RoadClassCategory {
        match self {
            Self::Motorway | Self::Trunk | Self::Primary | Self::Secondary | Self::Tertiary => {
                RoadClassCategory::MajorRoads
            }

            Self::Unclassified
            | Self::Residential
            | Self::LivingStreet
            | Self::Pedestrian
            | Self::Busway => RoadClassCategory::MinorRoads,

            Self::MotorwayLink
            | Self::TrunkLink
            | Self::PrimaryLink
            | Self::SecondaryLink
            | Self::TertiaryLink => RoadClassCategory::HighwayLinks,

            Self::Service
            | Self::Track
            | Self::TrackGrade1
            | Self::TrackGrade2
            | Self::TrackGrade3
            | Self::TrackGrade4
            | Self::TrackGrade5 => RoadClassCategory::VerySmallRoads,

            Self::Bridleway | Self::Cycleway | Self::Footway | Self::Path | Self::Steps => {
                RoadClassCategory::PathsUnsuitableForCars
            }

            Self::Unknown => RoadClassCategory::Unknown,
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "bevy-reflect", derive(Reflect))]
pub struct Road {
    pub osm_id: i64,
    pub class: RoadClass,
    pub reference: String,
    pub oneway: OneWay,
    pub max_speed: Option<u32>,
    pub layer: i32,
    pub is_bridge: bool,
    pub is_tunnel: bool,

    pub geometry: Vec<DVec2>,
}

fn parse_polyline(shape: &shapefile::Shape) -> Result<Vec<DVec2>, RecordParseError> {
    match shape {
        shapefile::Shape::Polyline(poly) => {
            if poly.parts().len() != 1 {
                return Err(RecordParseError::InvalidField {
                    field: "shape",
                    value: format!("expected one part, got {}", poly.parts().len()),
                });
            }

            Ok(poly
                .part(0)
                .unwrap_or(&vec![])
                .iter()
                .map(|p| DVec2::new(p.x, p.y))
                .collect())
        }
        _ => Err(RecordParseError::InvalidField {
            field: "shape",
            value: "unsupported shape type".into(),
        }),
    }
}

impl ShapefileElement for Road {
    fn from_shapefile_item(
        (shape, record): (shapefile::Shape, &shapefile::dbase::Record),
    ) -> Result<Self, RecordParseError> {
        let geometry = parse_polyline(&shape)?;

        Ok(Self {
            osm_id: record.required::<FromStrParser<i64>>("osm_id")?,

            class: record.required::<FromStrParser<RoadClass>>("fclass")?,

            reference: record.optional::<StringParser>("ref")?.unwrap_or_default(),

            oneway: record
                .optional::<OneWayParser>("oneway")?
                .unwrap_or(OneWay::Bidirectional),

            max_speed: record
                .optional::<OptionParser<U32Parser>>("maxspeed")?
                .flatten(),

            layer: record.required::<I32Parser>("layer")?,

            is_bridge: record.optional::<TfBoolParser>("bridge")?.unwrap_or(false),

            is_tunnel: record.optional::<TfBoolParser>("tunnel")?.unwrap_or(false),

            geometry,
        })
    }
}

impl Road {
    pub fn aabb(&self) -> DAabb2 {
        DAabb2::new(
            self.geometry
                .iter()
                .cloned()
                .reduce(|a, b| a.min(b))
                .unwrap(),
            self.geometry
                .iter()
                .cloned()
                .reduce(|a, b| a.max(b))
                .unwrap(),
        )
    }
}
