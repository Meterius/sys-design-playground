pub mod commands;
pub mod instance;
pub mod interop;

use bevy::prelude::*;

pub struct InstanceManagementPlugin {
    pub id: String,
}

impl Plugin for InstanceManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            instance::InstancePlugin {
                id: self.id.clone(),
            },
            commands::CommandsPlugin,
        ));
    }
}
