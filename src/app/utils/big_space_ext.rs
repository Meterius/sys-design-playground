use bevy::prelude::*;
use big_space::prelude::*;

pub trait CommandsWithSpatial<'w, 's> {
    fn spawn_spatial(&mut self, bundle: impl Bundle) -> EntityCommands<'_>;
}

impl<'w, 's> CommandsWithSpatial<'w, 's> for Commands<'w, 's> {
    fn spawn_spatial(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        let mut entity_commands = self.spawn((
            Visibility::default(),
            Transform::default(),
            CellCoord::default(),
        ));

        entity_commands.insert(bundle);

        entity_commands
    }
}
