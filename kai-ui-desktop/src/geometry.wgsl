// Triangle meshes of `DrawOp::Geometry`, antialiased by MSAA 4×. One draw
// call per mesh; its transform, stroke width and color come from a uniform
// at a dynamic offset, so panning or zooming re-uploads no vertices.
// Stroke vertices lie on the path and are pushed out along their normal
// after the transform, which keeps the width in screen pixels.

struct Globals {
    surface: vec2<f32>,
    _pad: vec2<f32>,
};

struct Draw {
    // Columns of the linear part, in physical pixels: (a, b), (c, d).
    linear: vec4<f32>,
    // tx, ty in physical pixels; half the stroke width; clip radius (0 = sharp).
    translate: vec4<f32>,
    // Premultiplied, already encoded for the target.
    color: vec4<f32>,
    // Rounded overflow mask: left, top, right, bottom (physical pixels).
    clip_round: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> draw: Draw;

struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) normal: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

fn rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    let m = mat2x2<f32>(draw.linear.xy, draw.linear.zw);
    var pixel = m * vertex.pos + draw.translate.xy;
    // The normal follows the transform's direction but keeps its length
    // (longer at miter joins).
    let turned = m * vertex.normal;
    let length_turned = length(turned);
    if length_turned > 0.0 {
        pixel += turned / length_turned * length(vertex.normal) * draw.translate.z;
    }
    var output: VertexOutput;
    output.position = vec4<f32>(
        pixel.x / globals.surface.x * 2.0 - 1.0,
        1.0 - pixel.y / globals.surface.y * 2.0,
        0.0,
        1.0,
    );
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let clip_radius = draw.translate.w;
    if (clip_radius > 0.0) {
        let pixel = input.position.xy;
        let half = (draw.clip_round.zw - draw.clip_round.xy) * 0.5;
        let center = (draw.clip_round.xy + draw.clip_round.zw) * 0.5;
        let radius = min(clip_radius, min(half.x, half.y));
        let distance = rounded_box(pixel - center, half, radius);
        let coverage = clamp(0.5 - distance, 0.0, 1.0);
        if (coverage <= 0.0) {
            discard;
        }
        return draw.color * coverage;
    }
    return draw.color;
}
