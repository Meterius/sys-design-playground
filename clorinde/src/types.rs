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
