use futures::task::SpawnExt;
use wgpu_glyph::{Section, Text};

pub struct Renderer {
  pub window: winit::window::Window,
  offset: winit::dpi::PhysicalPosition<f64>,
  surface: wgpu::Surface,
  size: winit::dpi::PhysicalSize<u32>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  staging_belt: wgpu::util::StagingBelt,
  local_spawner: futures::executor::LocalSpawner,
  local_pool: futures::executor::LocalPool,
  glyph_brush: wgpu_glyph::GlyphBrush<()>,
  text: String,
  font_height: f32,
}

const RENDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const TOP_PADDING: f32 = 10.0;

impl Renderer {
  pub async fn new(
    event_loop: &winit::event_loop::EventLoop<()>,
    font: wgpu_glyph::ab_glyph::FontArc,
    text: String,
  ) -> Result<Self, anyhow::Error> {
    let window = winit::window::WindowBuilder::new()
      .with_title(env!("CARGO_CRATE_NAME"))
      .build(event_loop)
      .unwrap();
    let instance = wgpu::Instance::new(wgpu::BackendBit::all());

    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
      })
      .await
      .ok_or_else(|| anyhow::anyhow!("Request adapter"))?;

    let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor::default(), None)
      .await?;

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

    Ok(Self {
      window,
      offset: winit::dpi::PhysicalPosition { x: 0f64, y: 0f64 },
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
      font_height: 40.0,
    })
  }

  pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
    self.size = size;

    self.swap_chain = self.device.create_swap_chain(
      &self.surface,
      &wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: RENDER_FORMAT,
        width: self.size.width,
        height: self.size.height,
        present_mode: wgpu::PresentMode::Mailbox,
      },
    );
  }

  pub fn scroll(&mut self, offset: winit::dpi::PhysicalPosition<f64>) {
    let line_count = self.text.lines().count();

    self.offset.x = (self.offset.x + offset.x).min(0f64);
    self.offset.y = (self.offset.y + offset.y).min(0f64).max(
      -(((line_count as f32 - 3.0) * self.font_height) + TOP_PADDING) as f64,
    );
  }

  pub fn redraw(&mut self) -> Result<(), anyhow::Error> {
    let mut encoder =
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Redraw"),
        });

    let frame = self.swap_chain.get_current_frame()?.output;

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
      screen_position: (20.0, TOP_PADDING + self.offset.y as f32),
      text: vec![Text::new(&self.text)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_height)],
      ..Section::default()
    });

    self
      .glyph_brush
      .draw_queued(
        &self.device,
        &mut self.staging_belt,
        &mut encoder,
        &frame.view,
        self.size.width,
        self.size.height,
      )
      .unwrap();

    self.staging_belt.finish();
    self.queue.submit(Some(encoder.finish()));
    self.local_spawner.spawn(self.staging_belt.recall())?;
    self.local_pool.run_until_stalled();

    Ok(())
  }
}
