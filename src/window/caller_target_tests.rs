//! A host-owned device, a caller target and a depth pre-pass, on a real
//! adapter. Skips when none is present.

use crate::camera::OrbitCamera3d;
use crate::color::{Color, TRANSPARENT};
use crate::context::Context;
use crate::scene::SceneNode3d;
use crate::window::{CanvasSetup, NumSamples, Window};

const SIZE: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn host() -> Option<(wgpu::Instance, wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .ok()?;
    Some((instance, adapter, device, queue))
}

fn texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

// Left half at the near plane (hides everything), right half cleared.
fn split_depth(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let depth = texture(
        device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            "@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
                let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
                return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
            }"
            .into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: None,
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_scissor_rect(0, 0, SIZE / 2, SIZE);
        pass.draw(0..3, 0..1);
    }
    queue.submit([encoder.finish()]);
    depth
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::Texture) -> Vec<u8> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (SIZE * SIZE * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        target.size(),
    );
    queue.submit([encoder.finish()]);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let bytes = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    bytes
}

fn frame(
    window: &mut Window,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    target: &wgpu::Texture,
    scene: &mut SceneNode3d,
    camera: &mut OrbitCamera3d,
) {
    let mut encoder = device.create_command_encoder(&Default::default());
    let report = window.render_into(&mut encoder, target, Some(scene), camera, &mut []);
    assert_eq!(report.internal_submissions, 0, "the caller submits");
    queue.submit([encoder.finish()]);
}

#[test]
fn host_device_caller_target_and_prepass() {
    let Some((instance, adapter, device, queue)) = host() else {
        eprintln!("no adapter; skipped");
        return;
    };
    let ctx = Context::from_host(instance, device.clone(), queue.clone(), adapter, FORMAT);
    let target = texture(
        &device,
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    {
        let _scope = ctx.enter();
        let setup = CanvasSetup {
            samples: NumSamples::One,
            ..Default::default()
        };
        let mut window = Window::new_on_context(SIZE, SIZE, setup);
        window.set_background_color(TRANSPARENT);
        let mut scene = SceneNode3d::empty();
        scene
            .add_cube(1.0, 1.0, 1.0)
            .set_color(Color::new(1.0, 0.0, 0.0, 1.0));
        let mut camera = OrbitCamera3d::default();

        frame(
            &mut window,
            &queue,
            &device,
            &target,
            &mut scene,
            &mut camera,
        );
        let full = read(&device, &queue, &target);
        let alpha = |px: &[u8], x: u32| px[((SIZE / 2 * SIZE + x) * 4 + 3) as usize];
        assert!(
            alpha(&full, SIZE / 2 - 4) > 0 && alpha(&full, SIZE / 2 + 4) > 0,
            "cube drawn"
        );

        window.set_depth_prepass(Some(split_depth(&device, &queue)));
        frame(
            &mut window,
            &queue,
            &device,
            &target,
            &mut scene,
            &mut camera,
        );
        let joined = read(&device, &queue, &target);
        assert_eq!(
            alpha(&joined, SIZE / 2 - 4),
            0,
            "hidden behind the pre-pass"
        );
        assert!(
            alpha(&joined, SIZE / 2 + 4) > 0,
            "drawn where the pre-pass is far"
        );
    }
    assert!(
        Context::current().is_none(),
        "scope restored the empty slot"
    );
    // The last surface dropped; the host's device must still work.
    let _ = read(&device, &queue, &target);
}
