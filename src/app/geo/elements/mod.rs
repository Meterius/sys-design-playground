pub mod manager;

use crate::app::geo::elements::manager::{Bounds, ElementId, ManagerPlugin};
use crate::app::utils::async_requests::RequestKind;
use bevy::app::App;
use bevy::prelude::{Plugin, Reflect, TypePath};
use std::marker::PhantomData;

pub struct ElementsPlugin<T, K>
where
    T: Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
{
    marker_t: PhantomData<T>,
    marker_k: PhantomData<K>,
}

impl<T, K> ElementsPlugin<T, K>
where
    T: Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            marker_t: PhantomData,
            marker_k: PhantomData,
        }
    }
}

impl<T, K> Plugin for ElementsPlugin<T, K>
where
    T: ElementId + Reflect + TypePath + Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Reflect + TypePath + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_plugins(ManagerPlugin::<T, K>::new());
    }
}
