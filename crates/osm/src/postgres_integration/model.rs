use crate::model::road::{OneWay, RoadClass, RoadClassCategory};
use crate::model::water::WaterClass;
use generated_queries::types as sql_types;
use crate::model::landuse::LanduseClass;

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

impl From<WaterClass> for sql_types::WaterClass {
    fn from(value: WaterClass) -> Self {
        match value {
            WaterClass::Dock => Self::dock,
            WaterClass::Reservoir => Self::reservoir,
            WaterClass::River => Self::river,
            WaterClass::Water => Self::water,
            WaterClass::Wetland => Self::wetland,
            WaterClass::Glacier => Self::glacier,
            WaterClass::Riverbank => Self::riverbank,
            WaterClass::WetlandFen => Self::wetland_fen,
            WaterClass::WetlandReedbed => Self::wetland_reedbed,
            WaterClass::WetlandSwamp => Self::wetland_swamp,
            WaterClass::WetlandTidalflat => Self::wetland_tidalflat,
            WaterClass::WetlandWetMeadow => Self::wetland_wet_meadow,
            WaterClass::WetlandMangrove => Self::wetland_mangrove,
            WaterClass::WetlandMarsh => Self::wetland_marsh,
            WaterClass::WetlandSaltmarsh => Self::wetland_saltmarsh,
            WaterClass::WetlandBog => Self::wetland_bog,
        }
    }
}

impl From<sql_types::WaterClass> for WaterClass {
    fn from(value: sql_types::WaterClass) -> Self {
        match value {
            sql_types::WaterClass::water => Self::Water,
            sql_types::WaterClass::river => Self::River,
            sql_types::WaterClass::reservoir => Self::Reservoir,
            sql_types::WaterClass::dock => Self::Dock,
            sql_types::WaterClass::wetland => Self::Wetland,
            sql_types::WaterClass::glacier => Self::Glacier,
            sql_types::WaterClass::riverbank => Self::Riverbank,
            sql_types::WaterClass::wetland_fen => Self::WetlandFen,
            sql_types::WaterClass::wetland_reedbed => Self::WetlandReedbed,
            sql_types::WaterClass::wetland_swamp => Self::WetlandSwamp,
            sql_types::WaterClass::wetland_tidalflat => Self::WetlandTidalflat,
            sql_types::WaterClass::wetland_wet_meadow => Self::WetlandWetMeadow,
            sql_types::WaterClass::wetland_mangrove => Self::WetlandMangrove,
            sql_types::WaterClass::wetland_marsh => Self::WetlandMarsh,
            sql_types::WaterClass::wetland_saltmarsh => Self::WetlandSaltmarsh,
            sql_types::WaterClass::wetland_bog => Self::WetlandBog,
        }
    }
}

impl From<LanduseClass> for sql_types::LanduseClass {
    fn from(value: LanduseClass) -> Self {
        match value {
            LanduseClass::Forest => Self::forest,
            LanduseClass::Park => Self::park,
            LanduseClass::Residential => Self::residential,
            LanduseClass::Industrial => Self::industrial,
            LanduseClass::Cemetery => Self::cemetery,
            LanduseClass::Allotments => Self::allotments,
            LanduseClass::Meadow => Self::meadow,
            LanduseClass::Commercial => Self::commercial,
            LanduseClass::RecreationGround => Self::recreation_ground,
            LanduseClass::Retail => Self::retail,
            LanduseClass::Military => Self::military,
            LanduseClass::Quarry => Self::quarry,
            LanduseClass::Orchard => Self::orchard,
            LanduseClass::Vineyard => Self::vineyard,
            LanduseClass::Scrub => Self::scrub,
            LanduseClass::Grass => Self::grass,
            LanduseClass::Heath => Self::heath,
            LanduseClass::Farmland => Self::farmland,
            LanduseClass::Farmyard => Self::farmyard,
            LanduseClass::Landfill => Self::landfill,
        }
    }
}

impl From<sql_types::LanduseClass> for LanduseClass {
    fn from(value: sql_types::LanduseClass) -> Self {
        match value {
            sql_types::LanduseClass::forest => Self::Forest,
            sql_types::LanduseClass::park => Self::Park,
            sql_types::LanduseClass::residential => Self::Residential,
            sql_types::LanduseClass::industrial => Self::Industrial,
            sql_types::LanduseClass::cemetery => Self::Cemetery,
            sql_types::LanduseClass::allotments => Self::Allotments,
            sql_types::LanduseClass::meadow => Self::Meadow,
            sql_types::LanduseClass::commercial => Self::Commercial,
            sql_types::LanduseClass::recreation_ground => Self::RecreationGround,
            sql_types::LanduseClass::retail => Self::Retail,
            sql_types::LanduseClass::military => Self::Military,
            sql_types::LanduseClass::quarry => Self::Quarry,
            sql_types::LanduseClass::orchard => Self::Orchard,
            sql_types::LanduseClass::vineyard => Self::Vineyard,
            sql_types::LanduseClass::scrub => Self::Scrub,
            sql_types::LanduseClass::grass => Self::Grass,
            sql_types::LanduseClass::heath => Self::Heath,
            sql_types::LanduseClass::farmland => Self::Farmland,
            sql_types::LanduseClass::farmyard => Self::Farmyard,
            sql_types::LanduseClass::landfill => Self::Landfill,
        }
    }
}