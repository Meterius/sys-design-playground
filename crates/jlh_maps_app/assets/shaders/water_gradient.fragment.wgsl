#import bevy_pbr::{
    decal::clustered::apply_decals,
    forward_io::{FragmentOutput, VertexOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{
        alpha_discard,
        apply_pbr_lighting,
        main_pass_post_lighting_processing,
    },
    pbr_types,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}

struct WaterMaterial {
    params: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var edge_distance_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var edge_distance_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> water_material: WaterMaterial;

const WATER_COL: vec3<f32> = vec3<f32>(0.6666667, 0.8235294, 0.9529412);
const WATER2_COL: vec3<f32> = vec3<f32>(0.5647059, 0.7764706, 0.9215686);
const FOAM_COL: vec3<f32> = vec3<f32>(0.9568627, 0.96862745, 0.99215686);
const M_2PI: f32 = 6.283185307;
const M_6PI: f32 = 18.84955592;

fn circ(pos: vec2<f32>, c: vec2<f32>, s: f32) -> f32 {
    var d = abs(pos - c);
    d = min(d, vec2<f32>(1.0) - d);

    return smoothstep(0.0, 0.002, sqrt(s) - sqrt(dot(d, d))) * -1.0;
}

fn waterlayer(input_uv: vec2<f32>) -> f32 {
    let uv = input_uv - floor(input_uv);
    var ret = 1.0;
    ret += circ(uv, vec2<f32>(0.37378, 0.277169), 0.0268181);
    ret += circ(uv, vec2<f32>(0.0317477, 0.540372), 0.0193742);
    ret += circ(uv, vec2<f32>(0.430044, 0.882218), 0.0232337);
    ret += circ(uv, vec2<f32>(0.641033, 0.695106), 0.0117864);
    ret += circ(uv, vec2<f32>(0.0146398, 0.0791346), 0.0299458);
    ret += circ(uv, vec2<f32>(0.43871, 0.394445), 0.0289087);
    ret += circ(uv, vec2<f32>(0.909446, 0.878141), 0.028466);
    ret += circ(uv, vec2<f32>(0.310149, 0.686637), 0.0128496);
    ret += circ(uv, vec2<f32>(0.928617, 0.195986), 0.0152041);
    ret += circ(uv, vec2<f32>(0.0438506, 0.868153), 0.0268601);
    ret += circ(uv, vec2<f32>(0.308619, 0.194937), 0.00806102);
    ret += circ(uv, vec2<f32>(0.349922, 0.449714), 0.00928667);
    ret += circ(uv, vec2<f32>(0.0449556, 0.953415), 0.023126);
    ret += circ(uv, vec2<f32>(0.117761, 0.503309), 0.0151272);
    ret += circ(uv, vec2<f32>(0.563517, 0.244991), 0.0292322);
    ret += circ(uv, vec2<f32>(0.566936, 0.954457), 0.00981141);
    ret += circ(uv, vec2<f32>(0.0489944, 0.200931), 0.0178746);
    ret += circ(uv, vec2<f32>(0.569297, 0.624893), 0.0132408);
    ret += circ(uv, vec2<f32>(0.298347, 0.710972), 0.0114426);
    ret += circ(uv, vec2<f32>(0.878141, 0.771279), 0.00322719);
    ret += circ(uv, vec2<f32>(0.150995, 0.376221), 0.00216157);
    ret += circ(uv, vec2<f32>(0.119673, 0.541984), 0.0124621);
    ret += circ(uv, vec2<f32>(0.629598, 0.295629), 0.0198736);
    ret += circ(uv, vec2<f32>(0.334357, 0.266278), 0.0187145);
    ret += circ(uv, vec2<f32>(0.918044, 0.968163), 0.0182928);
    ret += circ(uv, vec2<f32>(0.965445, 0.505026), 0.006348);
    ret += circ(uv, vec2<f32>(0.514847, 0.865444), 0.00623523);
    ret += circ(uv, vec2<f32>(0.710575, 0.0415131), 0.00322689);
    ret += circ(uv, vec2<f32>(0.71403, 0.576945), 0.0215641);
    ret += circ(uv, vec2<f32>(0.748873, 0.413325), 0.0110795);
    ret += circ(uv, vec2<f32>(0.0623365, 0.896713), 0.0236203);
    ret += circ(uv, vec2<f32>(0.980482, 0.473849), 0.00573439);
    ret += circ(uv, vec2<f32>(0.647463, 0.654349), 0.0188713);
    ret += circ(uv, vec2<f32>(0.651406, 0.981297), 0.00710875);
    ret += circ(uv, vec2<f32>(0.428928, 0.382426), 0.0298806);
    ret += circ(uv, vec2<f32>(0.811545, 0.62568), 0.00265539);
    ret += circ(uv, vec2<f32>(0.400787, 0.74162), 0.00486609);
    ret += circ(uv, vec2<f32>(0.331283, 0.418536), 0.00598028);
    ret += circ(uv, vec2<f32>(0.894762, 0.0657997), 0.00760375);
    ret += circ(uv, vec2<f32>(0.525104, 0.572233), 0.0141796);
    ret += circ(uv, vec2<f32>(0.431526, 0.911372), 0.0213234);
    ret += circ(uv, vec2<f32>(0.658212, 0.910553), 0.000741023);
    ret += circ(uv, vec2<f32>(0.514523, 0.243263), 0.0270685);
    ret += circ(uv, vec2<f32>(0.0249494, 0.252872), 0.00876653);
    ret += circ(uv, vec2<f32>(0.502214, 0.47269), 0.0234534);
    ret += circ(uv, vec2<f32>(0.693271, 0.431469), 0.0246533);
    ret += circ(uv, vec2<f32>(0.415, 0.884418), 0.0271696);
    ret += circ(uv, vec2<f32>(0.149073, 0.41204), 0.00497198);
    ret += circ(uv, vec2<f32>(0.533816, 0.897634), 0.00650833);
    ret += circ(uv, vec2<f32>(0.0409132, 0.83406), 0.0191398);
    ret += circ(uv, vec2<f32>(0.638585, 0.646019), 0.0206129);
    ret += circ(uv, vec2<f32>(0.660342, 0.966541), 0.0053511);
    ret += circ(uv, vec2<f32>(0.513783, 0.142233), 0.00471653);
    ret += circ(uv, vec2<f32>(0.124305, 0.644263), 0.00116724);
    ret += circ(uv, vec2<f32>(0.99871, 0.583864), 0.0107329);
    ret += circ(uv, vec2<f32>(0.894879, 0.233289), 0.00667092);
    ret += circ(uv, vec2<f32>(0.246286, 0.682766), 0.00411623);
    ret += circ(uv, vec2<f32>(0.0761895, 0.16327), 0.0145935);
    ret += circ(uv, vec2<f32>(0.949386, 0.802936), 0.0100873);
    ret += circ(uv, vec2<f32>(0.480122, 0.196554), 0.0110185);
    ret += circ(uv, vec2<f32>(0.896854, 0.803707), 0.013969);
    ret += circ(uv, vec2<f32>(0.292865, 0.762973), 0.00566413);
    ret += circ(uv, vec2<f32>(0.0995585, 0.117457), 0.00869407);
    ret += circ(uv, vec2<f32>(0.377713, 0.00335442), 0.0063147);
    ret += circ(uv, vec2<f32>(0.506365, 0.531118), 0.0144016);
    ret += circ(uv, vec2<f32>(0.408806, 0.894771), 0.0243923);
    ret += circ(uv, vec2<f32>(0.143579, 0.85138), 0.00418529);
    ret += circ(uv, vec2<f32>(0.0902811, 0.181775), 0.0108896);
    ret += circ(uv, vec2<f32>(0.780695, 0.394644), 0.00475475);
    ret += circ(uv, vec2<f32>(0.298036, 0.625531), 0.00325285);
    ret += circ(uv, vec2<f32>(0.218423, 0.714537), 0.00157212);
    ret += circ(uv, vec2<f32>(0.658836, 0.159556), 0.00225897);
    ret += circ(uv, vec2<f32>(0.987324, 0.146545), 0.0288391);
    ret += circ(uv, vec2<f32>(0.222646, 0.251694), 0.00092276);
    ret += circ(uv, vec2<f32>(0.159826, 0.528063), 0.00605293);
    return max(ret, 0.0);
}

fn water_color(input_uv: vec2<f32>, time: f32) -> vec3<f32> {
    var uv = input_uv * vec2<f32>(0.25);

    let cdir = vec3<f32>(0.0, 1.0, 0.0);
    let a = 0.025 * cdir.xz / cdir.y;
    var h = sin(uv.x + time);
    uv += a * h;
    h = sin(0.841471 * uv.x - 0.540302 * uv.y + time);
    uv += a * h;

    var d1 = uv.x + uv.y;
    d1 = d1 - floor(d1 / M_2PI) * M_2PI;
    var d2 = (uv.x + uv.y + 0.25) * 1.3;
    d2 = d2 - floor(d2 / M_6PI) * M_6PI;
    d1 = time * 0.07 + d1;
    d2 = time * 0.5 + d2;

    let dist = vec2<f32>(
        sin(d1) * 0.15 + sin(d2) * 0.05,
        cos(d1) * 0.15 + cos(d2) * 0.05,
    );

    var ret = mix(WATER_COL, WATER2_COL, waterlayer(uv + dist.xy));
    ret = mix(ret, FOAM_COL, waterlayer(vec2<f32>(1.0) - uv - dist.yx));
    return ret;
}

fn surf_stripe(dist: f32, phase_offset: f32, time: f32) -> f32 {
    let phase = fract(dist * 5.0 + time * 0.55 + phase_offset);
    return 1.0 - smoothstep(0.035, 0.32, abs(phase - 0.5));
}

fn shore_surf(uv: vec2<f32>, edge_distance: f32, time: f32) -> f32 {
    let shore_line = 1.0 - smoothstep(0.0, 0.2, edge_distance);
    let surf_zone = 1.0 - smoothstep(0.15, 0.72, edge_distance);

    var wave = sin(uv.x * 0.035 + uv.y * 0.018 + time * 2.7) * 0.025;
    wave += sin(uv.x * -0.018 + uv.y * 0.04 - time * 2.15) * 0.018;

    let distorted_distance = edge_distance + 2.0 * wave * surf_zone;
    var stripes = surf_stripe(distorted_distance, 0.0, time);
    stripes += surf_stripe(distorted_distance + 0.16, 0.0, time) * 0.55;

    stripes = smoothstep(0.45, 0.78, stripes) * surf_zone;
    return clamp(max(shore_line, stripes), 0.0, 1.0);
}

fn dyn_water_color(in: VertexOutput) -> vec3<f32> {
    let time = water_material.params.x;
    let tile_uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
    let edge_distance = clamp(textureSample(edge_distance_texture, edge_distance_sampler, tile_uv).r, 0.0, 1.0);
    let uv = in.uv * 500.0;

    let dst = vec2<f32>(
        sin(uv.x * 0.035 + uv.y * 0.018 + time * 1.7) * 0.025,
        sin(uv.x * -0.018 + uv.y * 0.04 - time * 1.15) * 0.018,
    );
    let water_col = water_color(uv + 5.0 * dst * edge_distance, time);

    let surf = shore_surf(uv, clamp(2.0 * edge_distance, 0.0, 1.0), time);
    let toon_water = mix(WATER_COL, water_col, 0.55);
    return mix(toon_water, FOAM_COL, surf);
}

@fragment
fn fragment(
    vertex_output: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    if (vertex_output.uv.x < 0.0 || vertex_output.uv.x > 1.0 || vertex_output.uv.y < 0.0 || vertex_output.uv.y > 1.0) {
        discard;
    }

    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);
    pbr_input.material.base_color = vec4<f32>(dyn_water_color(vertex_output), 1.0);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    apply_decals(&pbr_input);

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
