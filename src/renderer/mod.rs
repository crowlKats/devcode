mod code_view;

use bytemuck::{Pod, Zeroable};
use futures::task::SpawnExt;
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use wgpu_glyph::ab_glyph::Font;
use winit::dpi::{PhysicalPosition, PhysicalSize};

const RENDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub struct Renderer {
  pub window: winit::window::Window,
  surface: wgpu::Surface,
  size: PhysicalSize<u32>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  staging_belt: wgpu::util::StagingBelt,
  local_spawner: futures::executor::LocalSpawner,
  local_pool: futures::executor::LocalPool,
  glyph_brush: wgpu_glyph::GlyphBrush<()>,
  code_view: code_view::CodeView,
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
      self.code_view.rpass(&mut rpass);
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
  fn scroll(
    &mut self,
    offset: winit::dpi::PhysicalPosition<f64>,
    size: winit::dpi::PhysicalSize<u32>,
  );
  fn redraw(&mut self, glyph_brush: &mut wgpu_glyph::GlyphBrush<()>);
  //fn get_rects(&self) -> &[&Rectangle];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
  position: [f32; 2],
  color: [f32; 3],
}

pub struct Rectangle {
  render_pipeline: wgpu::RenderPipeline,
  vertex_buffer: wgpu::Buffer,
}

impl Rectangle {
  pub fn new(
    device: &wgpu::Device,
    position: PhysicalPosition<f32>,
    end_position: PhysicalPosition<f32>,
    color: [f32; 3],
  ) -> Self {
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(Cow::from(include_str!(
        "./rectangle_shader.wgsl"
      ))),
      flags: wgpu::ShaderFlags::VALIDATION,
    });

    let render_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
      });

    let render_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader,
          entry_point: "vs_main",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float3],
          }],
        },
        fragment: Some(wgpu::FragmentState {
          module: &shader,
          entry_point: "fs_main",
          targets: &[RENDER_FORMAT.into()],
        }),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleStrip,
          ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
      });

    let vertices = &[
      Vertex {
        position: [position.x, position.y],
        color,
      }, // top left
      Vertex {
        position: [position.x + end_position.x, position.y],
        color,
      }, // top right
      Vertex {
        position: [position.x, position.y + end_position.y],
        color,
      }, // bottom left
      Vertex {
        position: [position.x + end_position.x, position.y + end_position.y],
        color,
      }, // bottom right
    ];

    let vertex_buffer =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsage::VERTEX,
      });

    Self {
      render_pipeline,
      vertex_buffer,
    }
  }

  pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    render_pass.draw(0..4, 0..1);
  }
}

fn calc_size(
  screen_size: PhysicalSize<u32>,
  position: PhysicalPosition<u32>,
  size: PhysicalSize<u32>,
) -> (PhysicalPosition<f32>, PhysicalPosition<f32>) {
  (
    PhysicalPosition {
      x: (((position.x as f32) / (screen_size.width as f32)) * 2.0) - 1.0,
      y: (((position.y as f32) / (screen_size.height as f32)) * 2.0) - 1.0,
    },
    PhysicalPosition {
      x: ((size.width as f32) / (screen_size.width as f32)) * 2.0,
      y: ((size.height as f32) / (screen_size.height as f32)) * 2.0,
    },
  )
}
