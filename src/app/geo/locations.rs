use bevy::prelude::*;

#[derive(Default)]
pub struct LocationsPlugin {}

impl Plugin for LocationsPlugin {
    fn build(&self, app: &mut App) {}
}

#[derive(Component)]
pub struct LocationsManager {}
