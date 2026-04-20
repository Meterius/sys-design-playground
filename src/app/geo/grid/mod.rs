use crate::app::geo::grid::manager::ManagerPlugin;
use bevy::prelude::*;

pub mod manager;

pub struct GridPlugin {}

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ManagerPlugin {});
    }
}
