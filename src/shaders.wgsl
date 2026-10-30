// Claudium shaders

struct Globals {
    view_proj: mat4x4<f32>,
    cam_base: vec4<i32>,   // integer camera position; chunk origins are made relative to it
    cam_frac: vec4<f32>,   // xyz = fractional eye position, w = daylight 0..1
    fog: vec4<f32>,        // x = fog start, y = fog end, z = underwater, w = time
    sky: vec4<f32>,
    atlas: vec4<f32>,      // x = tiles per row, y = tile width in uv, z = tile height in uv
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
    @location(2) tint: vec3<f32>,
    @location(3) dist: f32,
};

fn tint_color(t: u32) -> vec3<f32> {
    switch t {
        case 1u: { return vec3<f32>(0.57, 0.74, 0.35); }
        case 2u: { return vec3<f32>(0.47, 0.67, 0.19); }
        case 3u: { return vec3<f32>(0.25, 0.46, 0.89); }
        default: { return vec3<f32>(1.0, 1.0, 1.0); }
    }
}

fn face_shade(f: u32) -> f32 {
    switch f {
        case 0u, 1u: { return 0.62; }
        case 2u: { return 1.0; }
        case 3u: { return 0.5; }
        case 4u, 5u: { return 0.8; }
        default: { return 0.9; }
    }
}

@vertex
fn vs_main(@location(0) d: vec2<u32>, @location(1) origin: vec4<i32>) -> VOut {
    let a = d.x;
    let b = d.y;
    let local = vec3<f32>(f32(a & 31u), f32((a >> 10u) & 8191u) / 16.0, f32((a >> 5u) & 31u));
    let face = (a >> 23u) & 7u;
    let ao = (a >> 26u) & 3u;
    let corner = vec2<f32>(f32((a >> 28u) & 1u), f32((a >> 29u) & 1u));
    let tile = b & 65535u;
    let sky = f32((b >> 16u) & 15u);
    let blk = f32((b >> 20u) & 15u);
    let tint = (b >> 24u) & 3u;

    let rel = vec3<f32>(origin.xyz - g.cam_base.xyz) + local;
    var out: VOut;
    out.pos = g.view_proj * vec4<f32>(rel, 1.0);

    let per_row = u32(g.atlas.x);
    let cell = vec2<f32>(f32(tile % per_row), f32(tile / per_row));
    let inset = mix(vec2<f32>(0.002), vec2<f32>(0.998), corner);
    out.uv = (cell + inset) * g.atlas.yz;

    let level = max(sky * g.cam_frac.w, blk);
    let light = 0.05 + 0.95 * pow(0.82, 15.0 - level);
    var ao_table = array<f32, 4>(0.45, 0.65, 0.83, 1.0);
    let ao_f = ao_table[ao];
    out.shade = light * ao_f * face_shade(face);
    out.tint = tint_color(tint);
    out.dist = length(rel - g.cam_frac.xyz);
    return out;
}

fn apply_fog(c: vec3<f32>, dist: f32) -> vec3<f32> {
    let f = clamp((dist - g.fog.x) / max(g.fog.y - g.fog.x, 1.0), 0.0, 1.0);
    return mix(c, g.sky.rgb, f);
}

@fragment
fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(atlas_tex, atlas_smp, v.uv);
    if (t.a < 0.5) {
        discard;
    }
    return vec4<f32>(apply_fog(t.rgb * v.tint * v.shade, v.dist), 1.0);
}

@fragment
fn fs_water(v: VOut) -> @location(0) vec4<f32> {
    let t = textureSample(atlas_tex, atlas_smp, v.uv);
    return vec4<f32>(apply_fog(t.rgb * v.tint * v.shade, v.dist), 0.72);
}

// Block selection outline
@vertex
fn vs_line(@location(0) p: vec3<f32>) -> @builtin(position) vec4<f32> {
    return g.view_proj * vec4<f32>(p, 1.0);
}

@fragment
fn fs_line() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.7);
}

// 2D overlay (crosshair, hotbar)
struct HudOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_hud(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> HudOut {
    var o: HudOut;
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.uv = uv;
    o.color = color;
    return o;
}

@fragment
fn fs_hud(v: HudOut) -> @location(0) vec4<f32> {
    if (v.uv.x < 0.0) {
        return v.color;
    }
    let t = textureSample(atlas_tex, atlas_smp, v.uv);
    return vec4<f32>(t.rgb * v.color.rgb, t.a * v.color.a);
}
