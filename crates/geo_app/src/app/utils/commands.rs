use bevy::ecs::entity::EntityNotSpawnedError;
use bevy::ecs::system::Command;
use bevy::ecs::world::error::EntityMutableFetchError;
use bevy::prelude::*;

pub struct InsertIfActive<T: Bundle> {
    pub entity: Entity,
    pub bundle: T,
}

impl<T: Bundle> Command<Result<(), BevyError>> for InsertIfActive<T> {
    fn apply(self, world: &mut World) -> Result<(), BevyError> {
        if let Some(mut entity) =
            world
                .get_entity_mut(self.entity)
                .map(Some)
                .or_else(|err| match err {
                    EntityMutableFetchError::NotSpawned(EntityNotSpawnedError::Invalid(_)) => {
                        Ok(None)
                    }
                    _ => Err(err),
                })?
        {
            entity.insert(self.bundle);
        }

        Ok(())
    }
}
