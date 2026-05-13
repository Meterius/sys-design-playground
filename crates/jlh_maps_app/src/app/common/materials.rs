use bevy::app::{App, Plugin};
use bevy::asset::{Asset, Handle, load_internal_asset, uuid_handle};
use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::{Reflect, Shader};
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

const DEPTH_TEXTURE_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("5bc825bd-3fd3-49e7-8542-38a1d2426f04");
const TRANSPARENT_OVERWRITE_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("95b136fb-2f6c-4698-8506-ea8ca8367ff7");

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            DEPTH_TEXTURE_MATERIAL_SHADER_HANDLE,
            "../../../assets/shaders/depth.fragment.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            TRANSPARENT_OVERWRITE_MATERIAL_SHADER_HANDLE,
            "../../../assets/shaders/transparent_overwrite.fragment.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins((
            MaterialPlugin::<DepthTextureMaterial>::default(),
            MaterialPlugin::<TransparentOverwriteMaterial>::default(),
        ));
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct DepthTextureMaterial {}

impl Material for DepthTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        DEPTH_TEXTURE_MATERIAL_SHADER_HANDLE.into()
    }
}

#[derive(ShaderType, Reflect, Debug, Clone, Copy)]
pub struct TransparentOverwriteMaterialUniform {
    pub max_shadow_alpha: f32,
    _webgl2_padding_8b: u32,
    _webgl2_padding_12b: u32,
    _webgl2_padding_16b: u32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct TransparentOverwriteMaterial {
    #[uniform(0)]
    pub uniform: TransparentOverwriteMaterialUniform,
}

impl TransparentOverwriteMaterial {
    pub fn new(max_shadow_alpha: f32) -> Self {
        Self {
            uniform: TransparentOverwriteMaterialUniform {
                max_shadow_alpha,
                _webgl2_padding_8b: 0,
                _webgl2_padding_12b: 0,
                _webgl2_padding_16b: 0,
            },
        }
    }
}

impl Material for TransparentOverwriteMaterial {
    fn fragment_shader() -> ShaderRef {
        TRANSPARENT_OVERWRITE_MATERIAL_SHADER_HANDLE.into()
    }
}
