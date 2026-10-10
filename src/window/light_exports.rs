//! The frame's shadow atlas and light buffer, for other passes to read.

use crate::resource::ShadowResources;
use crate::window::Window;

/// WGSL declarations of the exported buffers: `LightData` (one clustered
/// light), `LightShadow`, `ShadowUniforms` and their array bounds.
pub const LIGHT_EXPORTS_WGSL: &str = include_str!("../builtin/light_exports.wgsl");

/// Handles to the lighting state of the last frame. Cloning is cheap.
///
/// The atlas is a `Depth32Float` 2D array, one layer per shadow view, in the
/// light-space clip space of `ShadowUniforms.view_proj`; sample it with the
/// comparison sampler. The primary lights (at most `MAX_LIGHTS`) live in the
/// materials' frame uniform and are not exported: their source is the scene.
#[derive(Clone)]
pub struct LightExports {
    /// Shadow atlas, comparison sampler, `ShadowUniforms` buffer and the
    /// coloured-transmittance atlas.
    pub shadow: ShadowResources,
    /// The clustered lights, `array<LightData>`, when the device runs
    /// clustered lighting and a frame has culled them.
    pub clustered_lights: Option<wgpu::Buffer>,
    /// Lights written to `clustered_lights` by the last frame.
    pub clustered_light_count: u32,
}

impl Window {
    /// The shadow atlas and light buffer of the last frame.
    pub fn light_exports(&self) -> LightExports {
        let clustered = self.clustered.as_ref();
        LightExports {
            shadow: self.shadow_mapper.resources(),
            clustered_lights: clustered.map(|c| c.lights_buffer().clone()),
            clustered_light_count: clustered.map_or(0, |c| c.light_count()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exported_layouts_match_the_shader() {
        // Line endings follow the checkout.
        let shader = include_str!("../builtin/default.wgsl").replace('\r', "");
        let exported = super::LIGHT_EXPORTS_WGSL.replace('\r', "");
        for name in [
            "struct LightData {",
            "struct LightShadow {",
            "struct ShadowUniforms {",
        ] {
            let block = |src: &str| {
                let start = src.find(name).expect(name);
                src[start..start + src[start..].find("\n}").unwrap()].to_string()
            };
            assert_eq!(block(&exported), block(&shader), "{}", name);
        }
        for bound in [
            "const MAX_SHADOW_VIEWS: u32 = 16u;",
            "const MAX_SHADOW_LIGHTS: u32 = 24u;",
        ] {
            assert!(
                shader.contains(bound) && exported.contains(bound),
                "{}",
                bound
            );
        }
        assert_eq!(crate::builtin::MAX_SHADOW_VIEWS, 16);
    }
}
