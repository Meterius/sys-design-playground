use bevy::app::App;
use bevy::prelude::{Plugin, Resource};

#[derive(Default)]
pub struct SettingsPlugin {}

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Settings { debug_mode: false });
    }
}

#[derive(Resource)]
pub struct Settings {
    pub debug_mode: bool,
}
