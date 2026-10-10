//! A caller's depth as the frame's starting depth.
//!
//! A tracer that writes its surfaces' depth in the raster's clip space
//! (standard z, cleared to 1.0, `Less`) hands that depth here; the frame
//! loads it before drawing, so hardware depth testing settles every pixel
//! between the traced surfaces and the rasterised scene.

use crate::context::Context;

/// The pre-pass: the caller's depth texture and the blit that loads it.
pub struct DepthPrepass {
    depth: wgpu::Texture,
    view: wgpu::TextureView,
    layout: wgpu::BindGroupLayout,
    // Built per sample count, since the frame's depth may be multisampled.
    pipeline: Option<(u32, wgpu::RenderPipeline)>,
}

impl DepthPrepass {
    /// Takes `depth`: `Depth32Float`, single-sampled, `TEXTURE_BINDING`.
    ///
    /// # Panics
    /// If the texture does not have that shape.
    pub fn new(depth: wgpu::Texture) -> DepthPrepass {
        assert_eq!(depth.format(), wgpu::TextureFormat::Depth32Float);
        assert_eq!(depth.sample_count(), 1);
        assert!(depth.usage().contains(wgpu::TextureUsages::TEXTURE_BINDING));
        let ctxt = Context::get();
        let layout = ctxt.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("depth_prepass_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        DepthPrepass {
            depth,
            view,
            layout,
            pipeline: None,
        }
    }

    /// The caller's depth texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.depth
    }

    /// Writes the caller's depth into `target`, a `Depth32Float` attachment
    /// of `sample_count` samples and the same size.
    ///
    /// # Panics
    /// If the sizes differ: a smaller texture would load zeros, the nearest
    /// depth, and hide the whole scene.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: (u32, u32),
        sample_count: u32,
    ) {
        assert_eq!(
            (self.depth.width(), self.depth.height()),
            size,
            "depth pre-pass size"
        );
        let ctxt = Context::get();
        if self.pipeline.as_ref().map(|(n, _)| *n) != Some(sample_count) {
            self.pipeline = Some((sample_count, self.build(&ctxt, sample_count)));
        }
        let bind = ctxt.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("depth_prepass_bind"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&self.view),
            }],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("depth_prepass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline.as_ref().expect("pipeline built").1);
        pass.set_bind_group(0, &bind, &[]);
        pass.draw(0..3, 0..1);
    }

    fn build(&self, ctxt: &Context, sample_count: u32) -> wgpu::RenderPipeline {
        let shader = ctxt.create_shader_module(
            Some("depth_prepass"),
            include_str!("../builtin/depth_prepass.wgsl"),
        );
        let layout = ctxt.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("depth_prepass_pipeline_layout"),
            bind_group_layouts: &[Some(&self.layout)],
            immediate_size: 0,
        });
        ctxt.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("depth_prepass_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Context::depth_format(),
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        })
    }
}
