use bevy::app::App;
use bevy::prelude::ReflectResource;
use bevy::prelude::{Plugin, Reflect, Resource};

#[derive(Default)]
pub struct SettingsPlugin {}

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Settings>()
            .insert_resource(Settings { debug_mode: false });
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct Settings {
    pub debug_mode: bool,
}
