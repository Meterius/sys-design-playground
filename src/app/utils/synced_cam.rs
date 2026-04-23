use crate::app::utils::debug::SoftExpect;
use bevy::app::{App, Plugin};
use bevy::camera::Camera;
use bevy::prelude::{
    Component, Entity, IntoScheduleConfigs, Projection, Query, Update, With, Without,
};
use bevy_pancam::PanCamSystems;

pub struct SyncedCamPlugin;

impl Plugin for SyncedCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_cam.after(PanCamSystems));
    }
}

#[derive(Component)]
pub struct SyncedCam {
    pub main_camera_id: Entity,
}

fn sync_cam(
    main_cameras: Query<(&Projection, &Camera), Without<SyncedCam>>,
    mut sync_cameras: Query<(&mut Projection, &mut Camera, &SyncedCam), With<SyncedCam>>,
) {
    for (mut sync_proj, mut sync_cam, sync) in sync_cameras.iter_mut() {
        if let Some((main_proj, main_cam)) =
            main_cameras.get(sync.main_camera_id).ok().soft_expect("")
        {
            *sync_proj = main_proj.clone();
            let order = sync_cam.order;
            *sync_cam = main_cam.clone();
            sync_cam.order = order;
        }
    }
}
