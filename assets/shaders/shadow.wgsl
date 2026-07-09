#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var heightmap: texture_2d<f32>;
@group(2) @binding(1) var heightmap_sampler: sampler;

struct ShadowSettings {
    light_dir: vec3<f32>,
    height_scale: f32,
    shadow_color: vec4<f32>,
    step_size: f32,
    max_steps: i32,
};
@group(2) @binding(2) var<uniform> settings: ShadowSettings;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    let base_color = textureSample(heightmap, heightmap_sampler, uv);
    let base_height = base_color.r * settings.height_scale;

    if base_color.a < 0.1 {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    let ray_dir_2d = normalize(settings.light_dir.xy);

    let slope = settings.light_dir.z;

    var is_in_shadow = false;
    var current_uv = uv;
    var current_ray_height = base_height;

    for (var i = 0; i < settings.max_steps; i++) {
        current_uv += ray_dir_2d * settings.step_size;

        current_ray_height += slope;

        if current_uv.x < 0.0 || current_uv.x > 1.0 ||
            current_uv.y < 0.0 || current_uv.y > 1.0 {
            break;
        }

        let sample_color = textureSample(heightmap, heightmap_sampler, current_uv);
        let sampled_height = sample_color.r * settings.height_scale;

        if sampled_height > current_ray_height {
            is_in_shadow = true;
            break;
        }
    }

    if is_in_shadow {
        return settings.shadow_color;
    }

    return vec4(0.0, 0.0, 0.0, 0.0);
}
