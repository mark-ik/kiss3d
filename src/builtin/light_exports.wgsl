// The layouts of the buffers `Window::light_exports` hands out, copied
// verbatim from default.wgsl (a test keeps them identical).

// Single light data structure
struct LightData {
    position: vec3<f32>,
    light_type: u32,
    direction: vec3<f32>,
    intensity: f32,
    color: vec3<f32>,
    inner_cone_cos: f32,
    outer_cone_cos: f32,
    attenuation_radius: f32,
    // Index into ShadowUniforms.lights, or 0xffffffff when the light casts no
    // shadow. Used by the clustered tier (the primary tier uses its slot index).
    shadow_slot: u32,
    // Light-layer bitmask (lighting channels). The fragment skips this light when
    // `(object.light_layers & layers) == 0`. `0xffffffff` affects every object.
    layers: u32,
}

// Maximum number of atlas views (must match builtin/shadow.rs MAX_SHADOW_VIEWS).
const MAX_SHADOW_VIEWS: u32 = 16u;
// Per-light shadow-metadata slots (must match shadow.rs MAX_SHADOW_LIGHTS =
// MAX_LIGHTS + MAX_SHADOW_VIEWS). Primary tier in 0..MAX_LIGHTS, clustered above.
const MAX_SHADOW_LIGHTS: u32 = 24u;

// Per-light shadow metadata (mirrors GpuLightShadow in builtin/shadow.rs).
struct LightShadow {
    base_view: u32,
    num_views: u32,
    light_type: u32,
    enabled: f32,
    light_pos: vec3<f32>,
    far_plane: f32,
}

struct ShadowUniforms {
    view_proj: array<mat4x4<f32>, MAX_SHADOW_VIEWS>,
    lights: array<LightShadow, MAX_SHADOW_LIGHTS>,
    shadows_enabled: f32,
    texel_size: f32,
    depth_bias: f32,
    // 1.0 when translucent casters tinted the colored transmittance atlas.
    transmittance_enabled: f32,
    // PCF kernel scale (shadow softness/blur): 1.0 = default, larger = softer.
    softness: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    // Far view-space distance of each directional cascade (0..num_cascades).
    cascade_splits: vec4<f32>,
}
