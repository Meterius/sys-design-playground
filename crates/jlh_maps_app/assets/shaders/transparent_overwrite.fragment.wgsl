#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    shadows,
}
#import bevy_render::view

struct TransparentOverwriteMaterial {
    max_shadow_alpha: f32,
    _webgl2_padding_8b: u32,
    _webgl2_padding_12b: u32,
    _webgl2_padding_16b: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: TransparentOverwriteMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_position = view::position_world_to_view(
        in.world_position.xyz,
        view_bindings::view.view_from_world,
    );
    let normal = normalize(in.world_normal);

    var shadow = 1.0;
    for (var i = 0u; i < view_bindings::lights.n_directional_lights; i = i + 1u) {
        let light = &view_bindings::lights.directional_lights[i];
        if (((*light).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = min(
                shadow,
                shadows::fetch_directional_shadow(i, in.world_position, normal, view_position.z),
            );
        }
    }

    let shadow_alpha = min(1.0 - shadow, material.max_shadow_alpha);
    return vec4<f32>(0.0, 0.0, 0.0, shadow_alpha);
}
