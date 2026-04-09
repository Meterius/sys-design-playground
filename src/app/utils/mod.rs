use bevy::log::warn;
use bevy::prelude::{Bundle, EntityCommands, Transform};
use bevy::prelude::{Commands, Visibility};
use big_space::prelude::CellCoord;

pub trait SoftExpect {
    fn soft_expect(self, msg: &str) -> Self;
}

impl<T> SoftExpect for Option<T> {
    fn soft_expect(self, msg: &str) -> Self {
        if self.is_none() {
            warn!(
                "{}",
                if msg.is_empty() {
                    "Expected to be Some but was None"
                } else {
                    msg
                }
            );
        }
        self
    }
}

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
