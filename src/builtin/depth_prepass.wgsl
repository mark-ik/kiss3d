// Writes a caller's depth (standard z, the raster's clip space) into the
// frame's depth buffer, every sample of it, before the scene draws.

@group(0) @binding(0) var prepass: texture_depth_2d;

@vertex
fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) position: vec4<f32>) -> @builtin(frag_depth) f32 {
    return textureLoad(prepass, vec2<i32>(position.xy), 0);
}
