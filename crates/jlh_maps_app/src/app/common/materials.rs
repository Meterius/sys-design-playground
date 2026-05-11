use bevy::app::{App, Plugin};
use bevy::asset::{Asset, Handle, load_internal_asset, uuid_handle};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::{Shader, TypePath};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

const DEPTH_TEXTURE_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("5bc825bd-3fd3-49e7-8542-38a1d2426f04");

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            DEPTH_TEXTURE_MATERIAL_SHADER_HANDLE,
            "../../../assets/shaders/depth.fragment.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<DepthTextureMaterial>::default());
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DepthTextureMaterial {}

impl Material for DepthTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        DEPTH_TEXTURE_MATERIAL_SHADER_HANDLE.into()
    }
}
