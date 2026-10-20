// Instanced raster images: a rounded rectangle (the same signed distance
// field as `quad.wgsl`) filled with a region of the RGBA image atlas,
// sampled with a linear filter. The atlas holds premultiplied,
// sRGB-encoded pixels; they are blended in gamma space like everything else
// (Paso 14), or turned linear first with `KAI_LINEAR_BLEND=1`.

struct Uniforms {
    surface: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct Instance {
    // x, y, width, height in physical pixels
    @location(0) rect: vec4<f32>,
    // region shown, in atlas texels: x, y, width, height
    @location(1) source: vec4<f32>,
    // left, top, right, bottom
    @location(2) clip: vec4<f32>,
    // radius, opacity, linear output (0/1), unused
    @location(3) params: vec4<f32>,
    // whole atlas region of the image (for clamping): x, y, width, height
    @location(4) region: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) texel: vec2<f32>,
    @location(3) clip: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) region: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32, instance: Instance) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    // One pixel of room for anti-aliasing the rounded edge.
    let origin = instance.rect.xy - vec2<f32>(1.0);
    let size = instance.rect.zw + vec2<f32>(2.0);
    let pixel = origin + corners[index] * size;

    var output: VertexOutput;
    output.position = vec4<f32>(
        pixel.x / uniforms.surface.x * 2.0 - 1.0,
        1.0 - pixel.y / uniforms.surface.y * 2.0,
        0.0,
        1.0,
    );
    output.half_size = instance.rect.zw * 0.5;
    output.local = pixel - (instance.rect.xy + output.half_size);
    let along = (pixel - instance.rect.xy) / max(instance.rect.zw, vec2<f32>(1.0));
    output.texel = instance.source.xy + along * instance.source.zw;
    output.clip = instance.clip;
    output.params = instance.params;
    output.region = instance.region;
    return output;
}

fn rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = input.position.xy;
    if (pixel.x < input.clip.x || pixel.y < input.clip.y || pixel.x > input.clip.z || pixel.y > input.clip.w) {
        discard;
    }
    // Stay half a texel inside the image so the filter never reads the
    // padding around it.
    let low = input.region.xy + vec2<f32>(0.5);
    let high = input.region.xy + input.region.zw - vec2<f32>(0.5);
    let texel = clamp(input.texel, low, high);
    let atlas_size = vec2<f32>(textureDimensions(atlas));
    var color = textureSampleLevel(atlas, atlas_sampler, texel / atlas_size, 0.0);
    if (input.params.z > 0.5 && color.a > 0.0) {
        color = vec4<f32>(srgb_to_linear(color.rgb / color.a) * color.a, color.a);
    }
    let radius = min(input.params.x, min(input.half_size.x, input.half_size.y));
    let distance = rounded_box(input.local, input.half_size, radius);
    let coverage = clamp(0.5 - distance, 0.0, 1.0);
    return color * (input.params.y * coverage);
}
