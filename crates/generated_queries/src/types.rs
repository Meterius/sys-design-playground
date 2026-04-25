// This file was generated with `clorinde`. Do not modify.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum RoadClass {
    motorway,
    trunk,
    primary,
    secondary,
    tertiary,
    unclassified,
    residential,
    living_street,
    pedestrian,
    busway,
    motorway_link,
    trunk_link,
    primary_link,
    secondary_link,
    tertiary_link,
    service,
    track,
    track_grade1,
    track_grade2,
    track_grade3,
    track_grade4,
    track_grade5,
    bridleway,
    cycleway,
    footway,
    path,
    steps,
    unknown,
}
impl<'a> postgres_types::ToSql for RoadClass {
    fn to_sql(
        &self,
        ty: &postgres_types::Type,
        buf: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let s = match *self {
            RoadClass::motorway => "motorway",
            RoadClass::trunk => "trunk",
            RoadClass::primary => "primary",
            RoadClass::secondary => "secondary",
            RoadClass::tertiary => "tertiary",
            RoadClass::unclassified => "unclassified",
            RoadClass::residential => "residential",
            RoadClass::living_street => "living_street",
            RoadClass::pedestrian => "pedestrian",
            RoadClass::busway => "busway",
            RoadClass::motorway_link => "motorway_link",
            RoadClass::trunk_link => "trunk_link",
            RoadClass::primary_link => "primary_link",
            RoadClass::secondary_link => "secondary_link",
            RoadClass::tertiary_link => "tertiary_link",
            RoadClass::service => "service",
            RoadClass::track => "track",
            RoadClass::track_grade1 => "track_grade1",
            RoadClass::track_grade2 => "track_grade2",
            RoadClass::track_grade3 => "track_grade3",
            RoadClass::track_grade4 => "track_grade4",
            RoadClass::track_grade5 => "track_grade5",
            RoadClass::bridleway => "bridleway",
            RoadClass::cycleway => "cycleway",
            RoadClass::footway => "footway",
            RoadClass::path => "path",
            RoadClass::steps => "steps",
            RoadClass::unknown => "unknown",
        };
        buf.extend_from_slice(s.as_bytes());
        std::result::Result::Ok(postgres_types::IsNull::No)
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "road_class" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 28 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "motorway" => true,
                    "trunk" => true,
                    "primary" => true,
                    "secondary" => true,
                    "tertiary" => true,
                    "unclassified" => true,
                    "residential" => true,
                    "living_street" => true,
                    "pedestrian" => true,
                    "busway" => true,
                    "motorway_link" => true,
                    "trunk_link" => true,
                    "primary_link" => true,
                    "secondary_link" => true,
                    "tertiary_link" => true,
                    "service" => true,
                    "track" => true,
                    "track_grade1" => true,
                    "track_grade2" => true,
                    "track_grade3" => true,
                    "track_grade4" => true,
                    "track_grade5" => true,
                    "bridleway" => true,
                    "cycleway" => true,
                    "footway" => true,
                    "path" => true,
                    "steps" => true,
                    "unknown" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        postgres_types::__to_sql_checked(self, ty, out)
    }
}
impl<'a> postgres_types::FromSql<'a> for RoadClass {
    fn from_sql(
        ty: &postgres_types::Type,
        buf: &'a [u8],
    ) -> Result<RoadClass, Box<dyn std::error::Error + Sync + Send>> {
        match std::str::from_utf8(buf)? {
            "motorway" => Ok(RoadClass::motorway),
            "trunk" => Ok(RoadClass::trunk),
            "primary" => Ok(RoadClass::primary),
            "secondary" => Ok(RoadClass::secondary),
            "tertiary" => Ok(RoadClass::tertiary),
            "unclassified" => Ok(RoadClass::unclassified),
            "residential" => Ok(RoadClass::residential),
            "living_street" => Ok(RoadClass::living_street),
            "pedestrian" => Ok(RoadClass::pedestrian),
            "busway" => Ok(RoadClass::busway),
            "motorway_link" => Ok(RoadClass::motorway_link),
            "trunk_link" => Ok(RoadClass::trunk_link),
            "primary_link" => Ok(RoadClass::primary_link),
            "secondary_link" => Ok(RoadClass::secondary_link),
            "tertiary_link" => Ok(RoadClass::tertiary_link),
            "service" => Ok(RoadClass::service),
            "track" => Ok(RoadClass::track),
            "track_grade1" => Ok(RoadClass::track_grade1),
            "track_grade2" => Ok(RoadClass::track_grade2),
            "track_grade3" => Ok(RoadClass::track_grade3),
            "track_grade4" => Ok(RoadClass::track_grade4),
            "track_grade5" => Ok(RoadClass::track_grade5),
            "bridleway" => Ok(RoadClass::bridleway),
            "cycleway" => Ok(RoadClass::cycleway),
            "footway" => Ok(RoadClass::footway),
            "path" => Ok(RoadClass::path),
            "steps" => Ok(RoadClass::steps),
            "unknown" => Ok(RoadClass::unknown),
            s => Result::Err(Into::into(format!("invalid variant `{}`", s))),
        }
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "road_class" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 28 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "motorway" => true,
                    "trunk" => true,
                    "primary" => true,
                    "secondary" => true,
                    "tertiary" => true,
                    "unclassified" => true,
                    "residential" => true,
                    "living_street" => true,
                    "pedestrian" => true,
                    "busway" => true,
                    "motorway_link" => true,
                    "trunk_link" => true,
                    "primary_link" => true,
                    "secondary_link" => true,
                    "tertiary_link" => true,
                    "service" => true,
                    "track" => true,
                    "track_grade1" => true,
                    "track_grade2" => true,
                    "track_grade3" => true,
                    "track_grade4" => true,
                    "track_grade5" => true,
                    "bridleway" => true,
                    "cycleway" => true,
                    "footway" => true,
                    "path" => true,
                    "steps" => true,
                    "unknown" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum RoadClassCategory {
    major_roads,
    minor_roads,
    highway_links,
    very_small_roads,
    paths_unsuitable_for_cars,
    unknown,
}
impl<'a> postgres_types::ToSql for RoadClassCategory {
    fn to_sql(
        &self,
        ty: &postgres_types::Type,
        buf: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let s = match *self {
            RoadClassCategory::major_roads => "major_roads",
            RoadClassCategory::minor_roads => "minor_roads",
            RoadClassCategory::highway_links => "highway_links",
            RoadClassCategory::very_small_roads => "very_small_roads",
            RoadClassCategory::paths_unsuitable_for_cars => "paths_unsuitable_for_cars",
            RoadClassCategory::unknown => "unknown",
        };
        buf.extend_from_slice(s.as_bytes());
        std::result::Result::Ok(postgres_types::IsNull::No)
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "road_class_category" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 6 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "major_roads" => true,
                    "minor_roads" => true,
                    "highway_links" => true,
                    "very_small_roads" => true,
                    "paths_unsuitable_for_cars" => true,
                    "unknown" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        postgres_types::__to_sql_checked(self, ty, out)
    }
}
impl<'a> postgres_types::FromSql<'a> for RoadClassCategory {
    fn from_sql(
        ty: &postgres_types::Type,
        buf: &'a [u8],
    ) -> Result<RoadClassCategory, Box<dyn std::error::Error + Sync + Send>> {
        match std::str::from_utf8(buf)? {
            "major_roads" => Ok(RoadClassCategory::major_roads),
            "minor_roads" => Ok(RoadClassCategory::minor_roads),
            "highway_links" => Ok(RoadClassCategory::highway_links),
            "very_small_roads" => Ok(RoadClassCategory::very_small_roads),
            "paths_unsuitable_for_cars" => Ok(RoadClassCategory::paths_unsuitable_for_cars),
            "unknown" => Ok(RoadClassCategory::unknown),
            s => Result::Err(Into::into(format!("invalid variant `{}`", s))),
        }
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "road_class_category" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 6 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "major_roads" => true,
                    "minor_roads" => true,
                    "highway_links" => true,
                    "very_small_roads" => true,
                    "paths_unsuitable_for_cars" => true,
                    "unknown" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum RoadOneway {
    forwards_only,
    backwards_only,
    bidirectional,
}
impl<'a> postgres_types::ToSql for RoadOneway {
    fn to_sql(
        &self,
        ty: &postgres_types::Type,
        buf: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let s = match *self {
            RoadOneway::forwards_only => "forwards_only",
            RoadOneway::backwards_only => "backwards_only",
            RoadOneway::bidirectional => "bidirectional",
        };
        buf.extend_from_slice(s.as_bytes());
        std::result::Result::Ok(postgres_types::IsNull::No)
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "road_oneway" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 3 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "forwards_only" => true,
                    "backwards_only" => true,
                    "bidirectional" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        postgres_types::__to_sql_checked(self, ty, out)
    }
}
impl<'a> postgres_types::FromSql<'a> for RoadOneway {
    fn from_sql(
        ty: &postgres_types::Type,
        buf: &'a [u8],
    ) -> Result<RoadOneway, Box<dyn std::error::Error + Sync + Send>> {
        match std::str::from_utf8(buf)? {
            "forwards_only" => Ok(RoadOneway::forwards_only),
            "backwards_only" => Ok(RoadOneway::backwards_only),
            "bidirectional" => Ok(RoadOneway::bidirectional),
            s => Result::Err(Into::into(format!("invalid variant `{}`", s))),
        }
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "road_oneway" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 3 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "forwards_only" => true,
                    "backwards_only" => true,
                    "bidirectional" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum WaterClass {
    water,
    reservoir,
    river,
    dock,
    glacier,
    wetland,
    wetland_fen,
    riverbank,
    wetland_mangrove,
    wetland_marsh,
    wetland_tidalflat,
    wetland_reedbed,
    wetland_wet_meadow,
    wetland_swamp,
    wetland_saltmarsh,
    wetland_bog,
}
impl<'a> postgres_types::ToSql for WaterClass {
    fn to_sql(
        &self,
        ty: &postgres_types::Type,
        buf: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let s = match *self {
            WaterClass::water => "water",
            WaterClass::reservoir => "reservoir",
            WaterClass::river => "river",
            WaterClass::dock => "dock",
            WaterClass::glacier => "glacier",
            WaterClass::wetland => "wetland",
            WaterClass::wetland_fen => "wetland_fen",
            WaterClass::riverbank => "riverbank",
            WaterClass::wetland_mangrove => "wetland_mangrove",
            WaterClass::wetland_marsh => "wetland_marsh",
            WaterClass::wetland_tidalflat => "wetland_tidalflat",
            WaterClass::wetland_reedbed => "wetland_reedbed",
            WaterClass::wetland_wet_meadow => "wetland_wet_meadow",
            WaterClass::wetland_swamp => "wetland_swamp",
            WaterClass::wetland_saltmarsh => "wetland_saltmarsh",
            WaterClass::wetland_bog => "wetland_bog",
        };
        buf.extend_from_slice(s.as_bytes());
        std::result::Result::Ok(postgres_types::IsNull::No)
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "water_class" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 16 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "water" => true,
                    "reservoir" => true,
                    "river" => true,
                    "dock" => true,
                    "glacier" => true,
                    "wetland" => true,
                    "wetland_fen" => true,
                    "riverbank" => true,
                    "wetland_mangrove" => true,
                    "wetland_marsh" => true,
                    "wetland_tidalflat" => true,
                    "wetland_reedbed" => true,
                    "wetland_wet_meadow" => true,
                    "wetland_swamp" => true,
                    "wetland_saltmarsh" => true,
                    "wetland_bog" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        postgres_types::__to_sql_checked(self, ty, out)
    }
}
impl<'a> postgres_types::FromSql<'a> for WaterClass {
    fn from_sql(
        ty: &postgres_types::Type,
        buf: &'a [u8],
    ) -> Result<WaterClass, Box<dyn std::error::Error + Sync + Send>> {
        match std::str::from_utf8(buf)? {
            "water" => Ok(WaterClass::water),
            "reservoir" => Ok(WaterClass::reservoir),
            "river" => Ok(WaterClass::river),
            "dock" => Ok(WaterClass::dock),
            "glacier" => Ok(WaterClass::glacier),
            "wetland" => Ok(WaterClass::wetland),
            "wetland_fen" => Ok(WaterClass::wetland_fen),
            "riverbank" => Ok(WaterClass::riverbank),
            "wetland_mangrove" => Ok(WaterClass::wetland_mangrove),
            "wetland_marsh" => Ok(WaterClass::wetland_marsh),
            "wetland_tidalflat" => Ok(WaterClass::wetland_tidalflat),
            "wetland_reedbed" => Ok(WaterClass::wetland_reedbed),
            "wetland_wet_meadow" => Ok(WaterClass::wetland_wet_meadow),
            "wetland_swamp" => Ok(WaterClass::wetland_swamp),
            "wetland_saltmarsh" => Ok(WaterClass::wetland_saltmarsh),
            "wetland_bog" => Ok(WaterClass::wetland_bog),
            s => Result::Err(Into::into(format!("invalid variant `{}`", s))),
        }
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "water_class" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 16 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "water" => true,
                    "reservoir" => true,
                    "river" => true,
                    "dock" => true,
                    "glacier" => true,
                    "wetland" => true,
                    "wetland_fen" => true,
                    "riverbank" => true,
                    "wetland_mangrove" => true,
                    "wetland_marsh" => true,
                    "wetland_tidalflat" => true,
                    "wetland_reedbed" => true,
                    "wetland_wet_meadow" => true,
                    "wetland_swamp" => true,
                    "wetland_saltmarsh" => true,
                    "wetland_bog" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum LanduseClass {
    forest,
    park,
    residential,
    industrial,
    cemetery,
    allotments,
    meadow,
    commercial,
    recreation_ground,
    retail,
    military,
    quarry,
    orchard,
    vineyard,
    scrub,
    grass,
    heath,
    farmland,
    farmyard,
    landfill,
}
impl<'a> postgres_types::ToSql for LanduseClass {
    fn to_sql(
        &self,
        ty: &postgres_types::Type,
        buf: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let s = match *self {
            LanduseClass::forest => "forest",
            LanduseClass::park => "park",
            LanduseClass::residential => "residential",
            LanduseClass::industrial => "industrial",
            LanduseClass::cemetery => "cemetery",
            LanduseClass::allotments => "allotments",
            LanduseClass::meadow => "meadow",
            LanduseClass::commercial => "commercial",
            LanduseClass::recreation_ground => "recreation_ground",
            LanduseClass::retail => "retail",
            LanduseClass::military => "military",
            LanduseClass::quarry => "quarry",
            LanduseClass::orchard => "orchard",
            LanduseClass::vineyard => "vineyard",
            LanduseClass::scrub => "scrub",
            LanduseClass::grass => "grass",
            LanduseClass::heath => "heath",
            LanduseClass::farmland => "farmland",
            LanduseClass::farmyard => "farmyard",
            LanduseClass::landfill => "landfill",
        };
        buf.extend_from_slice(s.as_bytes());
        std::result::Result::Ok(postgres_types::IsNull::No)
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "landuse_class" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 20 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "forest" => true,
                    "park" => true,
                    "residential" => true,
                    "industrial" => true,
                    "cemetery" => true,
                    "allotments" => true,
                    "meadow" => true,
                    "commercial" => true,
                    "recreation_ground" => true,
                    "retail" => true,
                    "military" => true,
                    "quarry" => true,
                    "orchard" => true,
                    "vineyard" => true,
                    "scrub" => true,
                    "grass" => true,
                    "heath" => true,
                    "farmland" => true,
                    "farmyard" => true,
                    "landfill" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        postgres_types::__to_sql_checked(self, ty, out)
    }
}
impl<'a> postgres_types::FromSql<'a> for LanduseClass {
    fn from_sql(
        ty: &postgres_types::Type,
        buf: &'a [u8],
    ) -> Result<LanduseClass, Box<dyn std::error::Error + Sync + Send>> {
        match std::str::from_utf8(buf)? {
            "forest" => Ok(LanduseClass::forest),
            "park" => Ok(LanduseClass::park),
            "residential" => Ok(LanduseClass::residential),
            "industrial" => Ok(LanduseClass::industrial),
            "cemetery" => Ok(LanduseClass::cemetery),
            "allotments" => Ok(LanduseClass::allotments),
            "meadow" => Ok(LanduseClass::meadow),
            "commercial" => Ok(LanduseClass::commercial),
            "recreation_ground" => Ok(LanduseClass::recreation_ground),
            "retail" => Ok(LanduseClass::retail),
            "military" => Ok(LanduseClass::military),
            "quarry" => Ok(LanduseClass::quarry),
            "orchard" => Ok(LanduseClass::orchard),
            "vineyard" => Ok(LanduseClass::vineyard),
            "scrub" => Ok(LanduseClass::scrub),
            "grass" => Ok(LanduseClass::grass),
            "heath" => Ok(LanduseClass::heath),
            "farmland" => Ok(LanduseClass::farmland),
            "farmyard" => Ok(LanduseClass::farmyard),
            "landfill" => Ok(LanduseClass::landfill),
            s => Result::Err(Into::into(format!("invalid variant `{}`", s))),
        }
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        if ty.name() != "landuse_class" {
            return false;
        }
        match *ty.kind() {
            postgres_types::Kind::Enum(ref variants) => {
                if variants.len() != 20 {
                    return false;
                }
                variants.iter().all(|v| match &**v {
                    "forest" => true,
                    "park" => true,
                    "residential" => true,
                    "industrial" => true,
                    "cemetery" => true,
                    "allotments" => true,
                    "meadow" => true,
                    "commercial" => true,
                    "recreation_ground" => true,
                    "retail" => true,
                    "military" => true,
                    "quarry" => true,
                    "orchard" => true,
                    "vineyard" => true,
                    "scrub" => true,
                    "grass" => true,
                    "heath" => true,
                    "farmland" => true,
                    "farmyard" => true,
                    "landfill" => true,
                    _ => false,
                })
            }
            _ => false,
        }
    }
}
