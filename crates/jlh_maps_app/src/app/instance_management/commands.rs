use crate::app::instance_management::instance::{InstanceState, is_instance_mounted};
use bevy::prelude::*;
use std::cell::{LazyCell, RefCell};
use std::collections::HashMap;

thread_local! {
    static COMMAND_QUEUE: RefCell<LazyCell<HashMap<String, Vec<InstanceCommand>>>> = const { RefCell::new(LazyCell::new(HashMap::new)) };
}

pub(super) struct CommandsPlugin;

impl Plugin for CommandsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, drain_command_queue);
    }
}

pub struct InstanceCommand {
    command: Box<dyn FnOnce(&mut World)>,
}

fn drain_command_queue(world: &mut World) {
    let instance_id = world.get_resource::<InstanceState>().unwrap().id.clone();

    COMMAND_QUEUE.with(|command_queue| {
        let mut commands = command_queue.borrow_mut();

        if let Some(instance_commands) = commands.get_mut(&instance_id) {
            for command in instance_commands.drain(0..) {
                (command.command)(world);
            }
        }
    })
}

pub fn enqueue_instance_command(
    instance_id: &str,
    command: impl FnOnce(&mut World) + 'static,
) -> anyhow::Result<()> {
    if is_instance_mounted(instance_id) {
        COMMAND_QUEUE.with(|command_queue| {
            let mut commands = command_queue.borrow_mut();
            let instance_commands = commands.entry(instance_id.to_owned()).or_default();
            instance_commands.push(InstanceCommand {
                command: Box::new(command),
            });

            Ok(())
        })
    } else {
        anyhow::bail!("Instance {} is not mounted", instance_id);
    }
}
