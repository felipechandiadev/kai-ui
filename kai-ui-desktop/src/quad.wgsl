// Instanced rounded rectangles drawn with a signed distance field, or icons
// whose coverage comes from a mask atlas. All coordinates are physical
// pixels; colors are sRGB-encoded, premultiplied in the fragment stage and
// blended with premultiplied alpha in gamma space (non-sRGB target, Paso 14).
// With `KAI_LINEAR_BLEND=1` the inputs are linear and the target sRGB.

struct Uniforms {
    surface: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
// Icon coverage, rasterized at the drawn size (one texel per pixel).
@group(0) @binding(1) var masks: texture_2d<f32>;

struct Instance {
    // x, y, width, height
    @location(0) rect: vec4<f32>,
    @location(1) fill: vec4<f32>,
    @location(2) border: vec4<f32>,
    // left, top, right, bottom (AABB clip)
    @location(3) clip: vec4<f32>,
    // radius, border width, blur, mask mode
    @location(4) params: vec4<f32>,
    // mask region in the atlas: x, y, size, unused
    @location(5) uv: vec4<f32>,
    // rounded overflow mask: left, top, right, bottom; radius in clip_round_params.x
    @location(6) clip_round: vec4<f32>,
    @location(7) clip_round_params: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) fill: vec4<f32>,
    @location(3) border: vec4<f32>,
    @location(4) clip: vec4<f32>,
    @location(5) params: vec4<f32>,
    @location(6) uv: vec4<f32>,
    @location(7) clip_round: vec4<f32>,
    @location(8) clip_round_params: vec4<f32>,
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
    // Room for blur and one pixel of anti-aliasing.
    let pad = instance.params.z + 1.0;
    let origin = instance.rect.xy - vec2<f32>(pad);
    let size = instance.rect.zw + vec2<f32>(pad * 2.0);
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
    output.fill = instance.fill;
    output.border = instance.border;
    output.clip = instance.clip;
    output.params = instance.params;
    output.uv = instance.uv;
    output.clip_round = instance.clip_round;
    output.clip_round_params = instance.clip_round_params;
    return output;
}

fn rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn premultiply(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb * color.a, color.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = input.position.xy;
    if (pixel.x < input.clip.x || pixel.y < input.clip.y || pixel.x > input.clip.z || pixel.y > input.clip.w) {
        discard;
    }
    var clip_coverage = 1.0;
    let clip_radius = input.clip_round_params.x;
    if (clip_radius > 0.0) {
        let half = (input.clip_round.zw - input.clip_round.xy) * 0.5;
        let center = (input.clip_round.xy + input.clip_round.zw) * 0.5;
        let radius = min(clip_radius, min(half.x, half.y));
        let distance = rounded_box(pixel - center, half, radius);
        clip_coverage = clamp(0.5 - distance, 0.0, 1.0);
        if (clip_coverage <= 0.0) {
            discard;
        }
    }
    if (input.params.w > 0.5) {
        let texel = input.local + input.half_size;
        let size = input.uv.z;
        if (texel.x < 0.0 || texel.y < 0.0 || texel.x >= size || texel.y >= size) {
            discard;
        }
        let coverage = textureLoad(masks, vec2<i32>(floor(input.uv.xy + texel)), 0).r;
        return premultiply(input.fill) * coverage * clip_coverage;
    }
    let radius = min(input.params.x, min(input.half_size.x, input.half_size.y));
    let distance = rounded_box(input.local, input.half_size, radius);
    let blur = input.params.z;
    if (blur > 0.0) {
        let alpha = 1.0 - smoothstep(-blur, blur, distance);
        return premultiply(input.fill) * alpha * clip_coverage;
    }
    let coverage = clamp(0.5 - distance, 0.0, 1.0);
    let border_width = input.params.y;
    var color = premultiply(input.fill);
    if (border_width > 0.0) {
        let inside = clamp(0.5 - (distance + border_width), 0.0, 1.0);
        color = mix(premultiply(input.border), color, inside);
    }
    return color * coverage * clip_coverage;
}
