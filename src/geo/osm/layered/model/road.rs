use clorinde::types as sql_types;
use glam::DVec2;
use std::str::FromStr;
use strum::{AsRefStr, Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneWay {
    ForwardsOnly,
    BackwardsOnly,
    Bidirectional,
}

impl From<OneWay> for sql_types::RoadOneway {
    fn from(value: OneWay) -> Self {
        match value {
            OneWay::ForwardsOnly => Self::forwards_only,
            OneWay::BackwardsOnly => Self::backwards_only,
            OneWay::Bidirectional => Self::bidirectional,
        }
    }
}

impl From<sql_types::RoadOneway> for OneWay {
    fn from(value: sql_types::RoadOneway) -> Self {
        match value {
            sql_types::RoadOneway::forwards_only => Self::ForwardsOnly,
            sql_types::RoadOneway::backwards_only => Self::BackwardsOnly,
            sql_types::RoadOneway::bidirectional => Self::Bidirectional,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr)]
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

impl From<RoadClassCategory> for sql_types::RoadClassCategory {
    fn from(value: RoadClassCategory) -> Self {
        match value {
            RoadClassCategory::MajorRoads => Self::major_roads,
            RoadClassCategory::MinorRoads => Self::minor_roads,
            RoadClassCategory::HighwayLinks => Self::highway_links,
            RoadClassCategory::VerySmallRoads => Self::very_small_roads,
            RoadClassCategory::PathsUnsuitableForCars => Self::paths_unsuitable_for_cars,
            RoadClassCategory::Unknown => Self::unknown,
        }
    }
}

impl From<sql_types::RoadClassCategory> for RoadClassCategory {
    fn from(value: sql_types::RoadClassCategory) -> Self {
        match value {
            sql_types::RoadClassCategory::major_roads => Self::MajorRoads,
            sql_types::RoadClassCategory::minor_roads => Self::MinorRoads,
            sql_types::RoadClassCategory::highway_links => Self::HighwayLinks,
            sql_types::RoadClassCategory::very_small_roads => Self::VerySmallRoads,
            sql_types::RoadClassCategory::paths_unsuitable_for_cars => Self::PathsUnsuitableForCars,
            sql_types::RoadClassCategory::unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr)]
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

impl From<RoadClass> for sql_types::RoadClass {
    fn from(value: RoadClass) -> Self {
        match value {
            RoadClass::Motorway => Self::motorway,
            RoadClass::Trunk => Self::trunk,
            RoadClass::Primary => Self::primary,
            RoadClass::Secondary => Self::secondary,
            RoadClass::Tertiary => Self::tertiary,
            RoadClass::Unclassified => Self::unclassified,
            RoadClass::Residential => Self::residential,
            RoadClass::LivingStreet => Self::living_street,
            RoadClass::Pedestrian => Self::pedestrian,
            RoadClass::Busway => Self::busway,
            RoadClass::MotorwayLink => Self::motorway_link,
            RoadClass::TrunkLink => Self::trunk_link,
            RoadClass::PrimaryLink => Self::primary_link,
            RoadClass::SecondaryLink => Self::secondary_link,
            RoadClass::TertiaryLink => Self::tertiary_link,
            RoadClass::Service => Self::service,
            RoadClass::Track => Self::track,
            RoadClass::TrackGrade1 => Self::track_grade1,
            RoadClass::TrackGrade2 => Self::track_grade2,
            RoadClass::TrackGrade3 => Self::track_grade3,
            RoadClass::TrackGrade4 => Self::track_grade4,
            RoadClass::TrackGrade5 => Self::track_grade5,
            RoadClass::Bridleway => Self::bridleway,
            RoadClass::Cycleway => Self::cycleway,
            RoadClass::Footway => Self::footway,
            RoadClass::Path => Self::path,
            RoadClass::Steps => Self::steps,
            RoadClass::Unknown => Self::unknown,
        }
    }
}

impl From<sql_types::RoadClass> for RoadClass {
    fn from(value: sql_types::RoadClass) -> Self {
        match value {
            sql_types::RoadClass::motorway => Self::Motorway,
            sql_types::RoadClass::trunk => Self::Trunk,
            sql_types::RoadClass::primary => Self::Primary,
            sql_types::RoadClass::secondary => Self::Secondary,
            sql_types::RoadClass::tertiary => Self::Tertiary,
            sql_types::RoadClass::unclassified => Self::Unclassified,
            sql_types::RoadClass::residential => Self::Residential,
            sql_types::RoadClass::living_street => Self::LivingStreet,
            sql_types::RoadClass::pedestrian => Self::Pedestrian,
            sql_types::RoadClass::busway => Self::Busway,
            sql_types::RoadClass::motorway_link => Self::MotorwayLink,
            sql_types::RoadClass::trunk_link => Self::TrunkLink,
            sql_types::RoadClass::primary_link => Self::PrimaryLink,
            sql_types::RoadClass::secondary_link => Self::SecondaryLink,
            sql_types::RoadClass::tertiary_link => Self::TertiaryLink,
            sql_types::RoadClass::service => Self::Service,
            sql_types::RoadClass::track => Self::Track,
            sql_types::RoadClass::track_grade1 => Self::TrackGrade1,
            sql_types::RoadClass::track_grade2 => Self::TrackGrade2,
            sql_types::RoadClass::track_grade3 => Self::TrackGrade3,
            sql_types::RoadClass::track_grade4 => Self::TrackGrade4,
            sql_types::RoadClass::track_grade5 => Self::TrackGrade5,
            sql_types::RoadClass::bridleway => Self::Bridleway,
            sql_types::RoadClass::cycleway => Self::Cycleway,
            sql_types::RoadClass::footway => Self::Footway,
            sql_types::RoadClass::path => Self::Path,
            sql_types::RoadClass::steps => Self::Steps,
            sql_types::RoadClass::unknown => Self::Unknown,
        }
    }
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

#[derive(Debug, thiserror::Error)]
pub enum RoadRecordParseError {
    #[error("missing field `{0}`")]
    MissingField(&'static str),
    #[error("invalid field `{field}`: {value}")]
    InvalidField { field: &'static str, value: String },
}

impl Road {
    pub fn from_shapefile_item(
        (shape, record): (&shapefile::Shape, &shapefile::dbase::Record),
    ) -> Result<Self, RoadRecordParseError> {
        let geometry = match shape {
            shapefile::Shape::Polyline(poly) => {
                if poly.parts().len() != 1 {
                    return Err(RoadRecordParseError::InvalidField {
                        field: "shape",
                        value: format!("expected one polyline part, got {}", poly.parts().len()),
                    });
                }
                poly.part(0)
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|p| DVec2::new(p.x, p.y))
                    .collect::<Vec<_>>()
            }
            other => {
                return Err(RoadRecordParseError::InvalidField {
                    field: "shape",
                    value: format!("unsupported shape type"),
                });
            }
        };

        let class_str = match record.get("fclass") {
            Some(shapefile::dbase::FieldValue::Character(Some(v))) => v.trim().to_owned(),
            Some(other) => {
                return Err(RoadRecordParseError::InvalidField {
                    field: "fclass",
                    value: format!("{other:?}"),
                });
            }
            None => return Err(RoadRecordParseError::MissingField("fclass")),
        };
        let class =
            RoadClass::from_str(&class_str).map_err(|_| RoadRecordParseError::InvalidField {
                field: "fclass",
                value: class_str.clone(),
            })?;

        let oneway = match record.get("oneway") {
            Some(shapefile::dbase::FieldValue::Character(Some(v))) => match v.trim() {
                "F" => OneWay::ForwardsOnly,
                "T" => OneWay::BackwardsOnly,
                "B" | "" => OneWay::Bidirectional,
                other => {
                    return Err(RoadRecordParseError::InvalidField {
                        field: "oneway",
                        value: other.to_owned(),
                    });
                }
            },
            Some(shapefile::dbase::FieldValue::Character(None)) | None => OneWay::Bidirectional,
            Some(other) => {
                return Err(RoadRecordParseError::InvalidField {
                    field: "oneway",
                    value: format!("{other:?}"),
                });
            }
        };

        let max_speed = match record.get("maxspeed") {
            Some(shapefile::dbase::FieldValue::Numeric(Some(v))) => {
                if *v < 0.0 {
                    return Err(RoadRecordParseError::InvalidField {
                        field: "maxspeed",
                        value: v.to_string(),
                    });
                }
                Some(*v as u32)
            }
            Some(shapefile::dbase::FieldValue::Numeric(None)) | None => None,
            Some(other) => {
                return Err(RoadRecordParseError::InvalidField {
                    field: "maxspeed",
                    value: format!("{other:?}"),
                });
            }
        };

        let layer = match record.get("layer") {
            Some(shapefile::dbase::FieldValue::Numeric(Some(v))) => *v as i32,
            Some(other) => {
                return Err(RoadRecordParseError::InvalidField {
                    field: "layer",
                    value: format!("{other:?}"),
                });
            }
            None => return Err(RoadRecordParseError::MissingField("layer")),
        };

        let bridge = match record.get("bridge") {
            Some(shapefile::dbase::FieldValue::Character(Some(v))) => match v.trim() {
                "T" => true,
                "F" | "" => false,
                other => {
                    return Err(RoadRecordParseError::InvalidField {
                        field: "bridge",
                        value: other.to_owned(),
                    });
                }
            },
            Some(shapefile::dbase::FieldValue::Character(None)) | None => false,
            Some(other) => {
                return Err(RoadRecordParseError::InvalidField {
                    field: "bridge",
                    value: format!("{other:?}"),
                });
            }
        };

        let tunnel = match record.get("tunnel") {
            Some(shapefile::dbase::FieldValue::Character(Some(v))) => match v.trim() {
                "T" => true,
                "F" | "" => false,
                other => {
                    return Err(RoadRecordParseError::InvalidField {
                        field: "tunnel",
                        value: other.to_owned(),
                    });
                }
            },
            Some(shapefile::dbase::FieldValue::Character(None)) | None => false,
            Some(other) => {
                return Err(RoadRecordParseError::InvalidField {
                    field: "tunnel",
                    value: format!("{other:?}"),
                });
            }
        };

        Ok(Self {
            class,
            osm_id: match record.get("osm_id") {
                Some(shapefile::dbase::FieldValue::Character(Some(v))) => i64::from_str(v)
                    .map_err(|_| RoadRecordParseError::InvalidField {
                        field: "osm_id",
                        value: v.to_string(),
                    })?,
                other => {
                    return Err(RoadRecordParseError::InvalidField {
                        field: "osm_id",
                        value: format!("{other:?}"),
                    });
                }
            },
            reference: match record.get("ref") {
                Some(shapefile::dbase::FieldValue::Character(Some(v))) => v.trim().to_owned(),
                _ => String::new(),
            },
            oneway,
            max_speed,
            layer,
            is_bridge: bridge,
            is_tunnel: tunnel,
            geometry,
        })
    }
}
