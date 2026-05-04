use bevy::prelude::{Component, Reflect};

#[derive(Component, Debug, Eq, PartialEq, Clone, Copy, Reflect)]
pub enum DespawnIndicator {
    Active,
    Despawning,
}
