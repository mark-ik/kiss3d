//! Rendering into a caller's colour target, with no window.

use std::future::Future;
use std::pin::pin;
use std::task::{Context as TaskContext, Poll, Waker};

use crate::camera::Camera3d;
use crate::context::Context;
use crate::post_processing::PostProcessingEffect;
use crate::scene::SceneNode3d;
use crate::window::Window;

impl Window {
    /// Renders one frame of `scene` into `target`, a caller-owned texture,
    /// and returns once the frame's commands are submitted.
    ///
    /// The window takes the target's size. The target must be single-sampled,
    /// carry `RENDER_ATTACHMENT`, and have the current context's
    /// `surface_format`. Readback, recording and frame pacing are the
    /// caller's: the target is never copied and the call never waits.
    ///
    /// # Panics
    /// If no context is current, if the target's format or sample count
    /// does not match, or if the frame would wait on the platform.
    pub fn render_into(
        &mut self,
        target: &wgpu::Texture,
        scene: Option<&mut SceneNode3d>,
        camera: &mut dyn Camera3d,
        post_processing: &mut [&mut dyn PostProcessingEffect],
    ) -> bool {
        let format = Context::get().surface_format;
        assert_eq!(target.format(), format, "caller target format");
        assert_eq!(target.sample_count(), 1, "caller target sample count");
        let (w, h) = (target.width(), target.height());
        if self.canvas.size() != (w, h) {
            self.canvas.resize(w, h);
        }
        self.caller_target = Some(target.create_view(&wgpu::TextureViewDescriptor::default()));
        let frame = self.render_chain(scene, None, Some(camera), None, None, post_processing);
        let open = poll_once(frame);
        self.caller_target = None;
        open
    }
}

// With a caller target the frame never awaits, so one poll completes it.
fn poll_once<F: Future>(future: F) -> F::Output {
    let mut cx = TaskContext::from_waker(Waker::noop());
    match pin!(future).as_mut().poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("kiss3d: a caller-target frame waited on the platform"),
    }
}

impl Window {
    /// Starts every frame from `depth` instead of a cleared depth buffer,
    /// or clears again with `None`.
    ///
    /// `depth` is `Depth32Float`, single-sampled, `TEXTURE_BINDING`, the
    /// frame's size, written in the raster's clip space (standard z):
    /// a tracer's surfaces, so the scene draws only where it is nearer.
    pub fn set_depth_prepass(&mut self, depth: Option<wgpu::Texture>) {
        self.depth_prepass = depth.map(crate::renderer::DepthPrepass::new);
    }

    /// The frame's depth attachment, holding the pre-pass and the scene
    /// after a frame. Multisampled when the canvas is.
    pub fn depth_view(&self) -> &wgpu::TextureView {
        self.canvas.depth_view()
    }
}
