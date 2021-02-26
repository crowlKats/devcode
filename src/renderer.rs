use futures::task::SpawnExt;
use wgpu_glyph::{Region, Section, Text};

pub struct Renderer {
  pub window: winit::window::Window,
  pub surface: wgpu::Surface,
  pub size: winit::dpi::PhysicalSize<u32>,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
  pub swap_chain: wgpu::SwapChain,
  pub staging_belt: wgpu::util::StagingBelt,
  pub local_spawner: futures::executor::LocalSpawner,
  pub local_pool: futures::executor::LocalPool,
  pub glyph_brush: wgpu_glyph::GlyphBrush<()>,
  pub text: String,
}

pub(crate) const RENDER_FORMAT: wgpu::TextureFormat =
  wgpu::TextureFormat::Bgra8UnormSrgb;

impl Renderer {
  pub fn new(
    event_loop: &winit::event_loop::EventLoop<()>,
    font: wgpu_glyph::ab_glyph::FontArc,
    text: String,
  ) -> Self {
    let window = winit::window::WindowBuilder::new()
      .with_title(env!("CARGO_CRATE_NAME"))
      .build(event_loop)
      .unwrap();
    let instance = wgpu::Instance::new(wgpu::BackendBit::all());

    let surface = unsafe { instance.create_surface(&window) };
    let (device, queue) = futures::executor::block_on(async {
      let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
          power_preference: wgpu::PowerPreference::HighPerformance,
          compatible_surface: Some(&surface),
        })
        .await
        .expect("Request adapter");

      adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Request device")
    });
    let staging_belt = wgpu::util::StagingBelt::new(1024);
    let local_pool = futures::executor::LocalPool::new();
    let local_spawner = local_pool.spawner();

    let size = window.inner_size();
    let swap_chain = device.create_swap_chain(
      &surface,
      &wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: RENDER_FORMAT,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
      },
    );

    let glyph_brush = wgpu_glyph::GlyphBrushBuilder::using_font(font)
      .build(&device, RENDER_FORMAT);

    Self {
      window,
      surface,
      size,
      device,
      queue,
      swap_chain,
      staging_belt,
      local_spawner,
      local_pool,
      glyph_brush,
      text,
    }
  }

  pub fn redraw(&mut self) -> Result<(), anyhow::Error> {
    let mut encoder =
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Redraw"),
        });

    // Get the next frame
    let frame = self.swap_chain.get_current_frame()?.output;

    // Clear frame
    {
      let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render pass"),
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
          attachment: &frame.view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.01,
              g: 0.01,
              b: 0.01,
              a: 1.0,
            }),
            store: true,
          },
        }],
        depth_stencil_attachment: None,
      });
    }

    self.glyph_brush.queue(Section {
      screen_position: (20.0, 10.0),
      text: vec![Text::new(&self.text)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(40.0)],
      ..Section::default()
    });

    // Draw the text!
    self
      .glyph_brush
      .draw_queued_with_transform_and_scissoring(
        &self.device,
        &mut self.staging_belt,
        &mut encoder,
        &frame.view,
        wgpu_glyph::orthographic_projection(self.size.width, self.size.height),
        Region {
          x: 0,
          y: 0,
          width: self.size.width,
          height: self.size.height,
        },
      )
      .unwrap();

    // Submit the work!
    self.staging_belt.finish();
    self.queue.submit(Some(encoder.finish()));

    // Recall unused staging buffers
    self.local_spawner.spawn(self.staging_belt.recall())?;

    self.local_pool.run_until_stalled();

    Ok(())
  }
}
