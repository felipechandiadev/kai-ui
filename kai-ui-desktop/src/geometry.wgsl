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
    // tx, ty in physical pixels; half the stroke width; unused.
    translate: vec4<f32>,
    // Premultiplied, already encoded for the target.
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> draw: Draw;

struct Vertex {
    @location(0) pos: vec2<f32>,
    @location(1) normal: vec2<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> @builtin(position) vec4<f32> {
    let m = mat2x2<f32>(draw.linear.xy, draw.linear.zw);
    var pixel = m * vertex.pos + draw.translate.xy;
    // The normal follows the transform's direction but keeps its length
    // (longer at miter joins).
    let turned = m * vertex.normal;
    let length_turned = length(turned);
    if length_turned > 0.0 {
        pixel += turned / length_turned * length(vertex.normal) * draw.translate.z;
    }
    return vec4<f32>(
        pixel.x / globals.surface.x * 2.0 - 1.0,
        1.0 - pixel.y / globals.surface.y * 2.0,
        0.0,
        1.0,
    );
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return draw.color;
}
