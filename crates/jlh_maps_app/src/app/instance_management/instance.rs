use bevy::prelude::*;
use std::cell::LazyCell;
use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    static MOUNTED_INSTANCES: RefCell<LazyCell<HashSet<String>>> = const { RefCell::new(LazyCell::new(HashSet::new)) };
}

pub fn is_instance_mounted(instance_id: &str) -> bool {
    MOUNTED_INSTANCES.with(|mounted_instances| mounted_instances.borrow().contains(instance_id))
}

pub(super) struct InstancePlugin {
    pub id: String,
}

impl Plugin for InstancePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InstanceState {
            id: self.id.clone(),
        });
    }
}

#[derive(Resource, Debug, Reflect)]
pub struct InstanceState {
    pub id: String,
}

pub fn register_instance(instance_id: String) {
    MOUNTED_INSTANCES.with(|mounted_instances| {
        mounted_instances.borrow_mut().insert(instance_id);
    })
}

pub fn unregister_instance(instance_id: &str) {
    MOUNTED_INSTANCES.with(|mounted_instances| {
        mounted_instances.borrow_mut().remove(instance_id);
    })
}
