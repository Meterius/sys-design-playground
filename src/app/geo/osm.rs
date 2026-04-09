use crate::app::geo::map::{Map, MapView, MapViewTransform, MapViewWithMap};
use crate::app::utils::SoftExpect;
use crate::geo::coords::Projection2D;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use geojson::GeometryValue;
use glam::{DVec2, dvec2};
use itertools::Itertools;

pub struct OsmPlugin {}

impl Plugin for OsmPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_geometry);
    }
}

#[derive(Component)]
pub struct Geometry {
    pub geometry: GeometryValue,
}

fn setup_geometry(
    mut commands: Commands,
    geometries: Query<(Entity, &Geometry), Changed<Geometry>>,
    parents: Query<&ChildOf>,
    views: Query<(&MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
) {
    for (geo_id, geo) in geometries {
        if let Some((view, &MapViewWithMap(map_id))) = parents
            .iter_ancestors(geo_id)
            .filter_map(|p_id| views.get(p_id).ok())
            .next()
            .soft_expect("")
            && let Some(map) = maps.get(map_id).ok().soft_expect("")
        {
            let poly = match &geo.geometry {
                GeometryValue::Polygon { coordinates } => coordinates.clone(),
                GeometryValue::MultiPolygon { coordinates } => coordinates
                    .iter()
                    .flat_map(|poly| poly.iter().cloned())
                    .collect_vec(),
                _ => {
                    warn!("Unsupported geometry: {:?}", geo.geometry);
                    Vec::new()
                }
            }
            .into_iter()
            .map(|coords| {
                coords
                    .into_iter()
                    .map(|coord| {
                        map.projection
                            .gcs_to_abs(dvec2(coord[0].to_radians(), coord[1].to_radians()))
                    })
                    .collect_vec()
            })
            .collect_vec();

            let center = poly
                .iter()
                .map(|coords| coords.iter().sum::<DVec2>())
                .sum::<DVec2>()
                / poly.iter().map(|coords| coords.len() as f64).sum::<f64>();
            let center_local = view.abs_to_local(center);

            let mut shape = ShapePath::new();

            for coords in poly.iter() {
                shape = shape.move_to(view.abs_to_local(coords[coords.len() - 1]) - center_local);
                for &coord in coords.iter() {
                    shape = shape.line_to(view.abs_to_local(coord) - center_local);
                }
            }

            commands.entity(geo_id).insert((
                MapViewTransform {
                    translation: center,
                },
                ShapeBuilder::with(&shape)
                    .stroke((Color::srgb(1.0, 0.0, 0.0), 0.001))
                    .build(),
            ));
        }
    }
}
