#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct BuildingMaterial {
    height_gradient_strength: f32,
    height_gradient_upper_altitude: f32,

    base_shadow_strength: f32,
    base_shadow_upper_altitude: f32,

    lambert_tint_strength: f32,
    lambert_shade_strength: f32,
    _webgl2_padding_24b: u32,
    _webgl2_padding_28b: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> building_material: BuildingMaterial;

fn feature_altitude(vertex_output: VertexOutput) -> f32 {
#ifdef VERTEX_UVS_B
    return vertex_output.uv_b.x;
#else
    return 0.0;
#endif
}

fn base_shadow_factor(vertex_output: VertexOutput) -> f32 {
    let upper_altitude = max(building_material.base_shadow_upper_altitude, 0.0001);
    let height_fraction = clamp(feature_altitude(vertex_output) / upper_altitude, 0.0, 1.0);

    return -max(building_material.base_shadow_strength, 0.0) * (1.0 - height_fraction);
}

fn height_gradient_factor(vertex_output: VertexOutput) -> f32 {
    let upper_altitude = max(building_material.height_gradient_upper_altitude, 0.0001);
    let height_fraction = clamp(feature_altitude(vertex_output) / upper_altitude, 0.0, 1.0);

    return max(building_material.height_gradient_strength, 0.0) * height_fraction;
}

fn lambert_term(vertex_output: VertexOutput) -> f32 {
    let light_dir = normalize(vec3<f32>(0.45, -0.28, 0.85));
    return clamp(dot(normalize(vertex_output.world_normal), light_dir), 0.0, 1.0);
}

fn lambert_tint(color: vec3<f32>, lambert: f32) -> vec3<f32> {
    let cool = vec3<f32>(0.80, 0.88, 1.08);
    let warm = vec3<f32>(1.08, 1.02, 0.90);
    let tint = mix(cool, warm, lambert);
    let tint_strength = clamp(building_material.lambert_tint_strength, 0.0, 1.0);
    let shade = 1.0 - clamp(building_material.lambert_shade_strength, 0.0, 1.0) * (1.0 - lambert);

    return color * mix(vec3<f32>(1.0), tint, tint_strength) * shade;
}

@fragment
fn fragment(
    vertex_output: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);
    let gradient = 1.0 + height_gradient_factor(vertex_output) + base_shadow_factor(vertex_output);
    let lambert = lambert_term(vertex_output);

    pbr_input.material.base_color = vec4<f32>(
        lambert_tint(pbr_input.material.base_color.rgb * gradient, lambert),
        pbr_input.material.base_color.a,
    );
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    return deferred_output(vertex_output, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
