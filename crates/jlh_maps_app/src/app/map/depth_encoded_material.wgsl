#import bevy_pbr::forward_io::VertexOutput

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = clamp(in.position.z, 0.0, 1.0);
    return vec4<f32>(depth, 0.0, 0.0, 1.0);
}
