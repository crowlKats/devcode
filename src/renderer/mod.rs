mod code_view;
mod rectangle;

use bytemuck::{Pod, Zeroable};
use futures::task::SpawnExt;
use wgpu_glyph::ab_glyph::Font;
use winit::dpi::{PhysicalPosition, PhysicalSize};

const RENDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub struct Renderer {
  pub window: winit::window::Window,
  surface: wgpu::Surface,
  pub size: PhysicalSize<u32>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  staging_belt: wgpu::util::StagingBelt,
  local_spawner: futures::executor::LocalSpawner,
  local_pool: futures::executor::LocalPool,
  glyph_brush: wgpu_glyph::GlyphBrush<()>,
  pub code_view: code_view::CodeView,
  rectangle_render_pipeline: wgpu::RenderPipeline,
}

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

    let px_per_em = (13.0 / 72.0) * (96.0 * window.scale_factor() as f32);
    let units_per_em = font.units_per_em().unwrap();
    let height = font.height_unscaled();

    let font_size = font.glyph_bounds(
      &font
        .glyph_id('0')
        .with_scale((px_per_em / units_per_em) * height),
    );

    let glyph_brush = wgpu_glyph::GlyphBrushBuilder::using_font(font)
      .build(&device, RENDER_FORMAT);

    let code_view = code_view::CodeView::new(text, font_size, &device, size);
    let rectangle_render_pipeline = rectangle::Rectangle::pipeline(&device);
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
      code_view,
      rectangle_render_pipeline,
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

    self.code_view.resize(size);

    self.scroll(PhysicalPosition { x: 0.0, y: 0.0 });
  }

  pub fn scroll(&mut self, offset: PhysicalPosition<f64>) {
    self.code_view.scroll(offset, self.size);
  }

  pub fn redraw(&mut self) -> Result<(), anyhow::Error> {
    let mut encoder =
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Redraw"),
        });

    let frame = self.swap_chain.get_current_frame()?.output;

    self.code_view.rect.write_buffer(&self.queue);
    self.code_view.cursor.write_buffer(&self.queue);
    {
      let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

      rpass.set_pipeline(&self.rectangle_render_pipeline);
      for rect in self.code_view.get_rects() {
        rpass.set_vertex_buffer(0, rect.vertex_buffer.slice(..));
        if let Some(ref region) = rect.region {
          rpass.set_scissor_rect(
            region.x,
            region.y,
            region.width,
            region.height,
          );
        } else {
          rpass.set_scissor_rect(0, 0, self.size.width, self.size.height);
        }
        rpass.draw(0..4, 0..1);
      }
    }

    self.code_view.redraw(
      &mut self.glyph_brush,
      &self.device,
      &mut self.staging_belt,
      &mut encoder,
      &frame.view,
      self.size,
    );

    self.staging_belt.finish();
    self.queue.submit(Some(encoder.finish()));
    self.local_spawner.spawn(self.staging_belt.recall())?;
    self.local_pool.run_until_stalled();

    Ok(())
  }
}

trait RenderElement {
  fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>);
  fn scroll(
    &mut self,
    offset: winit::dpi::PhysicalPosition<f64>,
    size: winit::dpi::PhysicalSize<u32>,
  );
  fn redraw(
    &mut self,
    glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
    device: &wgpu::Device,
    staging_belt: &mut wgpu::util::StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    size: PhysicalSize<u32>,
  );
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
  position: [f32; 2],
  color: [f32; 3],
}
