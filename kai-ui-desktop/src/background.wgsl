// Window background effects (`background.rs`), after the GlowStudio CSS
// reference: blurred radial-gradient orbs (`filter: blur()`, multiply or
// normal blending) and a blurred, rotating conic gradient.
//
// `fs_effect` evaluates the effect at low resolution (the shapes are blurred
// by tens of points, so 1/8 of the physical pixels is plenty) into an
// `Rgba16Float` texture; `fs_composite` stretches it over the window with
// bilinear filtering and a little dither against banding. The blur is
// analytic: each orb is a radial profile of linear segments, blurred along
// the radius with the closed form of a Gaussian over a linear function; the
// aurora's box edges use the Gaussian CDF and its conic gradient is
// averaged over the angle the blur covers. Colors are sRGB-encoded and
// blended in gamma space, as browsers do (Paso 14).

struct Params {
    // Window size in logical points; animation time and cycle in seconds.
    size: vec2<f32>,
    time: f32,
    cycle: f32,
    base: vec4<f32>,
    c1: vec4<f32>,
    c2: vec4<f32>,
    c3: vec4<f32>,
    c4: vec4<f32>,
    // kind (1 mesh, 2 aurora, 3 glow), dark (0/1), blur (points), intensity
    effect: vec4<f32>,
    // low-resolution target size, surface size (physical pixels)
    target_size: vec4<f32>,
    // linear output (0/1), dither amplitude, extra white over the aurora
    // (`AURORA_WHITEN`, light only), unused
    flags: vec4<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var effect_texture: texture_2d<f32>;
@group(0) @binding(2) var effect_sampler: sampler;

const PI: f32 = 3.14159265;

@vertex
fn vs_full(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    // One triangle that covers the target.
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// Standard normal CDF and density.
fn cdf(z: f32) -> f32 {
    return 1.0 / (1.0 + exp(-1.5976 * z - 0.07056 * z * z * z));
}

fn pdf(z: f32) -> f32 {
    return 0.39894228 * exp(-0.5 * z * z);
}

// Gaussian blur (deviation `s`) at `x` of `a + b·y` restricted to [l, u].
fn segment(x: f32, l: f32, u: f32, a: vec4<f32>, b: vec4<f32>, s: f32) -> vec4<f32> {
    if (u - l < 1e-3) {
        return vec4<f32>(0.0);
    }
    let zl = (l - x) / s;
    let zu = (u - x) / s;
    return (a + b * x) * (cdf(zu) - cdf(zl)) + b * s * (pdf(zl) - pdf(zu));
}

// A radial profile v0 at r = 0, v1 at r1, v2 at r2 (linear in between, 0
// beyond r2; v2 > 0 is a hard edge such as `rounded-full`), blurred by `s`
// at distance `r` (premultiplied rgba). The profile is mirrored to negative
// radii so the center is blurred too.
fn radial(r: f32, r1: f32, r2: f32, v0: vec4<f32>, v1: vec4<f32>, v2: vec4<f32>, s: f32) -> vec4<f32> {
    let s1 = max(s, 0.5);
    let b1 = (v1 - v0) / max(r1, 1e-3);
    let a1 = v0;
    let b2 = (v2 - v1) / max(r2 - r1, 1e-3);
    let a2 = v1 - b2 * r1;
    return segment(r, 0.0, r1, a1, b1, s1)
        + segment(r, r1, r2, a2, b2, s1)
        + segment(r, -r1, 0.0, a1, -b1, s1)
        + segment(r, -r2, -r1, a2, -b2, s1);
}

// CSS `radial-gradient(circle, c 0%, transparent 70%)` on a `w`-wide
// rounded square: the farthest-corner radius is w·√2/2, so the color
// fades out at 0.495 w, inside the circle.
fn orb(p: vec2<f32>, center: vec2<f32>, w: f32, color: vec4<f32>, s: f32) -> vec4<f32> {
    let fade = 0.495 * w;
    let c = vec4<f32>(color.rgb, 1.0);
    return radial(distance(p, center), fade, fade, c, vec4<f32>(0.0), vec4<f32>(0.0), s);
}

// CSS `ease-in-out` (close to smoothstep).
fn ease(t: f32) -> f32 {
    let x = clamp(t, 0.0, 1.0);
    return x * x * (3.0 - 2.0 * x);
}

// The reference's keyframes: translate x, y (points) and scale.
fn float_1(u: f32) -> vec3<f32> {
    let k0 = vec3<f32>(0.0, 0.0, 1.0);
    let k1 = vec3<f32>(30.0, -50.0, 1.1);
    let k2 = vec3<f32>(-20.0, 20.0, 0.95);
    if (u < 0.33) {
        return mix(k0, k1, ease(u / 0.33));
    }
    if (u < 0.66) {
        return mix(k1, k2, ease((u - 0.33) / 0.33));
    }
    return mix(k2, k0, ease((u - 0.66) / 0.34));
}

fn float_2(u: f32) -> vec3<f32> {
    let k0 = vec3<f32>(0.0, 0.0, 1.0);
    let k1 = vec3<f32>(-40.0, 30.0, 1.05);
    let k2 = vec3<f32>(25.0, -35.0, 0.9);
    if (u < 0.33) {
        return mix(k0, k1, ease(u / 0.33));
    }
    if (u < 0.66) {
        return mix(k1, k2, ease((u - 0.33) / 0.33));
    }
    return mix(k2, k0, ease((u - 0.66) / 0.34));
}

fn float_3(u: f32) -> vec3<f32> {
    let k0 = vec3<f32>(0.0, 0.0, 1.0);
    let k1 = vec3<f32>(40.0, 40.0, 1.15);
    if (u < 0.5) {
        return mix(k0, k1, ease(u / 0.5));
    }
    return mix(k1, k0, ease((u - 0.5) / 0.5));
}

// An animated orb: `left`, `top` and `w` as in CSS, `motion` from a
// keyframe function; transforms scale around the center, blur included.
fn floating_orb(p: vec2<f32>, left: f32, top: f32, w: f32, motion: vec3<f32>, color: vec4<f32>, blur: f32) -> vec4<f32> {
    let center = vec2<f32>(left + w * 0.5, top + w * 0.5) + motion.xy;
    return orb(p, center, w * motion.z, color, blur * motion.z);
}

// Blend premultiplied `src` with opacity `o` over `dst`.
fn blend_normal(dst: vec3<f32>, src: vec4<f32>, o: f32) -> vec3<f32> {
    return dst * (1.0 - o * src.a) + o * src.rgb;
}

fn multiply(dst: vec3<f32>, src: vec4<f32>, o: f32) -> vec3<f32> {
    return dst * (1.0 - o * src.a + o * src.rgb);
}

fn screen(dst: vec3<f32>, src: vec4<f32>, o: f32) -> vec3<f32> {
    return dst + o * src.rgb * (1.0 - dst);
}

fn opacity(o: f32) -> f32 {
    return clamp(o * params.effect.w, 0.0, 1.0);
}

fn phase(scale: f32) -> f32 {
    return fract(params.time / (params.cycle * scale));
}

fn mesh(p: vec2<f32>) -> vec3<f32> {
    let size = params.size;
    let vw = size.x / 100.0;
    let blur = params.effect.z;
    let dark = params.effect.y > 0.5;
    let u = phase(1.0);
    var orbs = array<vec4<f32>, 4>();
    orbs[0] = floating_orb(p, -0.1 * size.x, -0.1 * size.y, 60.0 * vw, float_1(u), params.c1, blur);
    orbs[1] = floating_orb(p, 1.1 * size.x - 50.0 * vw, 0.2 * size.y, 50.0 * vw, float_2(u), params.c2, blur);
    orbs[2] = floating_orb(p, 0.2 * size.x, 1.1 * size.y - 55.0 * vw, 55.0 * vw, float_3(u), params.c3, blur);
    orbs[3] = floating_orb(p, 0.4 * size.x, 0.4 * size.y, 35.0 * vw, float_1(u), params.c4, max(blur - 10.0, 0.0));
    var alphas = array<f32, 4>(0.8, 0.75, 0.75, 0.6);
    var color = params.base.rgb;
    for (var i = 0; i < 4; i++) {
        if (dark) {
            color = screen(color, orbs[i], opacity(alphas[i]));
        } else {
            color = multiply(color, orbs[i], opacity(alphas[i]));
        }
    }
    return color;
}

// Conic gradient c1 c2 c3 c4 c1 from the top, clockwise.
fn conic(angle: f32) -> vec3<f32> {
    let t = fract(angle / (2.0 * PI)) * 4.0;
    let i = floor(t);
    let f = t - i;
    if (i < 1.0) {
        return mix(params.c1.rgb, params.c2.rgb, f);
    }
    if (i < 2.0) {
        return mix(params.c2.rgb, params.c3.rgb, f);
    }
    if (i < 3.0) {
        return mix(params.c3.rgb, params.c4.rgb, f);
    }
    return mix(params.c4.rgb, params.c1.rgb, f);
}

fn aurora(p: vec2<f32>) -> vec3<f32> {
    let size = params.size;
    let u = phase(1.0);
    // `animation: aurora-wave linear`: one turn per cycle, scale 1 → 1.2 → 1.
    let theta = 2.0 * PI * u;
    let scale = 1.0 + 0.2 * (1.0 - abs(2.0 * u - 1.0));
    // `-top-[30%] -left-[20%] w-[140%] h-[140%]`, `blur(blur + 20)`.
    let center = vec2<f32>(0.5 * size.x, 0.4 * size.y);
    let half_size = 0.7 * size * scale;
    let s = max((params.effect.z + 20.0) * scale, 0.5);
    let d = p - center;
    // CSS rotates clockwise on screen (y down).
    let q = vec2<f32>(cos(theta) * d.x + sin(theta) * d.y, -sin(theta) * d.x + cos(theta) * d.y);
    let cover_x = cdf((half_size.x - q.x) / s) - cdf((-half_size.x - q.x) / s);
    let cover_y = cdf((half_size.y - q.y) / s) - cdf((-half_size.y - q.y) / s);
    let cover = cover_x * cover_y;
    // Blur along the angle: about s / distance radians.
    let angle = atan2(q.x, -q.y);
    let spread = min(PI, s / max(length(q), 1.0));
    var gradient = vec3<f32>(0.0);
    var total = 0.0;
    for (var k = -3; k <= 3; k++) {
        let offset = f32(k) * 0.5;
        let weight = exp(-0.5 * offset * offset);
        gradient += weight * conic(angle + offset * spread);
        total += weight;
    }
    gradient /= total;
    let base = params.base.rgb;
    let color = mix(base, gradient, opacity(0.6) * cover);
    // `bg-white/40` on top (the window color in dark).
    let veil = select(vec3<f32>(1.0), base, params.effect.y > 0.5);
    let veiled = mix(color, veil, 0.4);
    // Lighter tint, same hues: a further share of white (0 in dark).
    return mix(veiled, vec3<f32>(1.0), params.flags.z);
}

fn glow(p: vec2<f32>) -> vec3<f32> {
    let size = params.size;
    let vw = size.x / 100.0;
    let blur = params.effect.z;
    let u = phase(1.0);
    // Main glow: `radial-gradient(circle, c1 0%, c2 50%, transparent 80%)`
    // in a 45vw circle centered at 50% 40%; the circle cuts it at 0.5 w,
    // where it is still 31 % opaque.
    let w = 45.0 * vw;
    let r1 = 0.3536 * w;
    let r2 = 0.5 * w;
    let edge = 1.0 - (r2 - r1) / (0.5657 * w - r1);
    let c1 = vec4<f32>(params.c1.rgb, 1.0);
    let c2 = vec4<f32>(params.c2.rgb, 1.0);
    let center_glow = radial(distance(p, vec2<f32>(0.5 * size.x, 0.4 * size.y)), r1, r2, c1, c2, c2 * edge, blur);
    let left = floating_orb(p, 0.15 * size.x, 0.15 * size.y, 30.0 * vw, float_2(u), params.c3, blur);
    let right = floating_orb(p, 0.85 * size.x - 35.0 * vw, 0.9 * size.y - 35.0 * vw, 35.0 * vw, float_3(u), params.c4, blur);
    var color = params.base.rgb;
    color = blend_normal(color, center_glow, opacity(0.8));
    color = blend_normal(color, left, opacity(0.6));
    color = blend_normal(color, right, opacity(0.6));
    return color;
}

@fragment
fn fs_effect(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let p = position.xy / params.target_size.xy * params.size;
    let kind = params.effect.x;
    var color = params.base.rgb;
    if (kind < 1.5) {
        color = mesh(p);
    } else if (kind < 2.5) {
        color = aurora(p);
    } else {
        color = glow(p);
    }
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_composite(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = position.xy / params.target_size.zw;
    var color = textureSample(effect_texture, effect_sampler, uv).rgb;
    // Ordered noise of ±½ step of 8 bits, fixed per pixel.
    let n = fract(sin(dot(floor(position.xy), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    color += (n - 0.5) * params.flags.y;
    if (params.flags.x > 0.5) {
        color = to_linear(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)));
    }
    return vec4<f32>(color, 1.0);
}
