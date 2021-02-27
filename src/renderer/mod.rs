mod code_view;

use futures::task::SpawnExt;
use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct Renderer {
  pub window: winit::window::Window,
  surface: wgpu::Surface,
  size: winit::dpi::PhysicalSize<u32>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  staging_belt: wgpu::util::StagingBelt,
  local_spawner: futures::executor::LocalSpawner,
  local_pool: futures::executor::LocalPool,
  glyph_brush: wgpu_glyph::GlyphBrush<()>,
  code_view: code_view::CodeView,
}

const RENDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

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
      surface,
      size,
      device,
      queue,
      swap_chain,
      staging_belt,
      local_spawner,
      local_pool,
      glyph_brush,
      code_view: code_view::CodeView::new(text),
    })
  }

  pub fn resize(&mut self, size: PhysicalSize<u32>) {
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

  pub fn scroll(&mut self, offset: PhysicalPosition<f64>) {
    self.code_view.scroll(offset);
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
        label: None,
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

    self.code_view.redraw(&mut self.glyph_brush);

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

trait RenderElement {
  fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>);
  fn scroll(&mut self, offset: winit::dpi::PhysicalPosition<f64>);
  fn redraw(&mut self, glyph_brush: &mut wgpu_glyph::GlyphBrush<()>);
}
