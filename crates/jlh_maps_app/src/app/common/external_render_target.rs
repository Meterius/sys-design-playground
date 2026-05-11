use crate::app::common::external_render_target;
use crate::app::instance_management::commands::enqueue_instance_command;
use crate::app::map::core::MapViewRenderTextureReference;
use bevy::app::{Plugin, Startup};
use bevy::camera::ManualTextureViewHandle;
use bevy::math::UVec2;
use bevy::prelude;
use bevy::prelude::{ManualTextureViews, Res, ResMut, Resource};
use bevy::render::renderer::RenderDevice;
#[cfg(target_arch = "wasm32")]
use bevy::render::renderer::WgpuWrapper;
use bevy::render::texture::ManualTextureView;
use tracing::{info, warn};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

pub const EXTERNAL_COLOR_TARGET_HANDLE: ManualTextureViewHandle = ManualTextureViewHandle(9101);
pub const EXTERNAL_R32F_TARGET_HANDLE: ManualTextureViewHandle = ManualTextureViewHandle(9102);

pub struct ExternalRenderTargetPlugin {
    pub render_texture_id: u32,
    pub render_texture_width: u32,
    pub render_texture_height: u32,
    pub external_framebuffer: JsValue,
    pub external_r32f_framebuffer: JsValue,
}

impl Plugin for ExternalRenderTargetPlugin {
    fn build(&self, app: &mut prelude::App) {
        app.add_systems(Startup, external_render_target::setup_external_targets);

        app.insert_resource(BevyRenderTextureConfig {
            reference: MapViewRenderTextureReference {
                id: self.render_texture_id,
                width: self.render_texture_width,
                height: self.render_texture_height,
            },
        });

        app.insert_resource(ExternalDepthTargetConfig {
            framebuffer: self.external_framebuffer.clone(),
            r32f_framebuffer: self.external_r32f_framebuffer.clone(),
            width: self.render_texture_width,
            height: self.render_texture_height,
        });
    }
}

#[wasm_bindgen]
pub fn resize_external_targets(
    instance_id: String,
    width: u32,
    height: u32,
) -> prelude::Result<(), String> {
    enqueue_instance_command(&instance_id, move |world| {
        let Some(render_device) = world.get_resource::<RenderDevice>() else {
            warn!("Cannot resize external targets before RenderDevice is available");
            return;
        };
        let Some(external_target) = world.get_resource::<ExternalDepthTargetConfig>() else {
            warn!("Cannot resize external targets before target config is available");
            return;
        };

        let color_framebuffer = external_target.framebuffer.clone();
        let r32f_framebuffer = external_target.r32f_framebuffer.clone();

        let Some(color_texture_view) = import_external_framebuffer_as_texture_view(
            render_device,
            color_framebuffer,
            width,
            height,
            wgpu::TextureFormat::Rgba8Unorm,
            "external_color_resized_target",
        ) else {
            warn!("Failed to resize external color render target");
            return;
        };

        let Some(r32f_texture_view) = import_external_framebuffer_as_texture_view(
            render_device,
            r32f_framebuffer,
            width,
            height,
            wgpu::TextureFormat::R32Float,
            "external_r32f_resized_target",
        ) else {
            warn!("Failed to resize external R32F render target");
            return;
        };

        let Some(mut manual_texture_views) = world.get_resource_mut::<ManualTextureViews>() else {
            return;
        };

        let size = UVec2::new(width, height);
        manual_texture_views.insert(
            EXTERNAL_COLOR_TARGET_HANDLE,
            ManualTextureView {
                texture_view: color_texture_view.into(),
                size,
                view_format: wgpu::TextureFormat::Rgba8Unorm,
            },
        );
        manual_texture_views.insert(
            EXTERNAL_R32F_TARGET_HANDLE,
            ManualTextureView {
                texture_view: r32f_texture_view.into(),
                size,
                view_format: wgpu::TextureFormat::R32Float,
            },
        );

        if let Some(mut external_target) = world.get_resource_mut::<ExternalDepthTargetConfig>() {
            external_target.width = width;
            external_target.height = height;
        }

        // if let Ok(mut window) = world
        //     .query_filtered::<&mut Window, With<PrimaryWindow>>()
        //     .single_mut(world)
        // {
        //     window.resolution.set_physical_resolution(width, height);
        // }
    })
    .map_err(|err| err.to_string())
}

#[derive(Resource)]
pub struct BevyRenderTextureConfig {
    pub reference: MapViewRenderTextureReference,
}

#[derive(Resource)]
pub struct ExternalDepthTargetConfig {
    pub framebuffer: JsValue,
    pub r32f_framebuffer: JsValue,
    pub width: u32,
    pub height: u32,
}

pub fn setup_external_targets(
    render_device: Res<RenderDevice>,
    external_target: Res<ExternalDepthTargetConfig>,
    mut manual_texture_views: ResMut<ManualTextureViews>,
) {
    let Some(texture_view) = import_external_framebuffer_as_texture_view(
        &render_device,
        external_target.framebuffer.clone(),
        external_target.width,
        external_target.height,
        wgpu::TextureFormat::Rgba8Unorm,
        "external_depth_probe_target",
    ) else {
        warn!("Failed to import external WebGL framebuffer as a Bevy render target");
        return;
    };

    manual_texture_views.insert(
        EXTERNAL_COLOR_TARGET_HANDLE,
        ManualTextureView {
            texture_view: texture_view.into(),
            size: UVec2::new(external_target.width, external_target.height),
            view_format: wgpu::TextureFormat::Rgba8Unorm,
        },
    );
    info!("Imported external WebGL framebuffer as a Bevy manual render target");

    let Some(r32f_texture_view) = import_external_framebuffer_as_texture_view(
        &render_device,
        external_target.r32f_framebuffer.clone(),
        external_target.width,
        external_target.height,
        wgpu::TextureFormat::R32Float,
        "external_r32f_probe_target",
    ) else {
        warn!("Failed to import external R32F WebGL framebuffer as a Bevy render target");
        return;
    };

    manual_texture_views.insert(
        EXTERNAL_R32F_TARGET_HANDLE,
        ManualTextureView {
            texture_view: r32f_texture_view.into(),
            size: UVec2::new(external_target.width, external_target.height),
            view_format: wgpu::TextureFormat::R32Float,
        },
    );
    info!("Imported external R32F WebGL framebuffer as a Bevy manual render target");
}

#[cfg(target_arch = "wasm32")]
fn import_external_framebuffer_as_texture_view(
    render_device: &RenderDevice,
    framebuffer: JsValue,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: &'static str,
) -> Option<wgpu::TextureView> {
    let framebuffer = framebuffer.dyn_into::<web_sys::WebGlFramebuffer>().ok()?;
    let device = unsafe { render_device_as_wgpu_device(render_device) };
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let hal_texture = wgpu::hal::gles::Texture {
        inner: wgpu::hal::gles::TextureInner::ExternalFramebuffer { inner: framebuffer },
        drop_guard: None,
        mip_level_count: 1,
        array_layer_count: 1,
        format,
        format_desc: wgpu::hal::gles::TextureFormatDesc {
            internal: 0,
            external: 0,
            data_type: 0,
        },
        copy_size: wgpu::hal::CopyExtent {
            width,
            height,
            depth: 1,
        },
    };
    let texture = unsafe {
        device.create_texture_from_hal::<wgpu::hal::api::Gles>(
            hal_texture,
            &wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        )
    };

    Some(texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

#[cfg(not(target_arch = "wasm32"))]
fn import_external_framebuffer_as_texture_view(
    _render_device: &RenderDevice,
    _framebuffer: JsValue,
    _width: u32,
    _height: u32,
    _format: wgpu::TextureFormat,
    _label: &'static str,
) -> Option<wgpu::TextureView> {
    None
}

#[cfg(target_arch = "wasm32")]
unsafe fn render_device_as_wgpu_device(render_device: &RenderDevice) -> &wgpu::Device {
    // Diagnostic bridge: Bevy wraps the wgpu device but does not expose
    // create_texture_from_hal. RenderDevice is a single-field wrapper in Bevy 0.18.

    (unsafe { &*(render_device as *const RenderDevice as *const WgpuWrapper<wgpu::Device>) }) as _
}
