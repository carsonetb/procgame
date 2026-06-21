#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var depth_texture: texture_depth_2d; // Your depth buffer!

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Get the base color
    let color = textureSample(screen_texture, screen_sampler, in.uv);

    // Read the depth buffer exactly at this pixel coordinate.
    // Because we are in screen-space, we can use the fragment's position
    // and textureLoad to get the exact depth value (0.0 to 1.0).
    let pixel_coords = vec2<i32>(in.position.xy);
    let depth = textureLoad(depth_texture, pixel_coords, 0);

    // TODO: Your raymarching / shadow collision logic here!

    return vec4(0.0, 0.0, 0.0, 1.0);
}
