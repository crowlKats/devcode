mod code_view;
mod code_view_tabs;
mod fs_tree;
pub mod input;
mod rectangle;

use futures::task::SpawnExt;
use std::path::PathBuf;
use wgpu::util::StagingBelt;
use wgpu::{CommandEncoder, Device, TextureView};
use wgpu_glyph::ab_glyph::Font;
use wgpu_glyph::GlyphBrush;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::ElementState;

const RENDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub struct Renderer {
  pub window: winit::window::Window,
  pub size: PhysicalSize<u32>,
  surface: wgpu::Surface,
  device: wgpu::Device,
  queue: wgpu::Queue,
  swap_chain: wgpu::SwapChain,
  staging_belt: wgpu::util::StagingBelt,
  local_spawner: futures::executor::LocalSpawner,
  local_pool: futures::executor::LocalPool,
  glyph_brush: wgpu_glyph::GlyphBrush<()>,
  rectangle_render_pipeline: wgpu::RenderPipeline,
  fs_tree: fs_tree::FsTree,
  pub font_height: f32,
  pub code_views: code_view_tabs::CodeViewTabs,
}

impl Renderer {
  pub async fn new(
    event_loop: &winit::event_loop::EventLoop<()>,
    font: wgpu_glyph::ab_glyph::FontArc,
    filepath: PathBuf,
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

    let px_per_em = (10.0 / 72.0) * (96.0 * window.scale_factor() as f32);
    let units_per_em = font.units_per_em().unwrap();
    let height = font.height_unscaled();
    let scale = (px_per_em / units_per_em) * height;

    let font_height = font
      .glyph_bounds(&font.glyph_id('0').with_scale(scale))
      .height();

    let glyph_brush = wgpu_glyph::GlyphBrushBuilder::using_font(font.clone())
      .build(&device, RENDER_FORMAT);

    // 20% for window for file tree
    let tree_width = (size.width as f32 / 100.0) * 20.0;

    let mut code_views = code_view_tabs::CodeViewTabs::new(
      &device,
      size.cast(),
      font,
      font_height,
      Dimensions {
        x: tree_width,
        y: 0.0,
        width: size.width as f32 - tree_width,
        height: size.height as f32,
      },
    );
    code_views.add(&device, size.cast(), filepath)?;

    let path = std::path::Path::new("./").canonicalize()?;
    let fs_tree = fs_tree::FsTree::new(
      &device,
      size.cast(),
      font_height,
      Dimensions {
        x: 0.0,
        y: 0.0,
        width: tree_width,
        height: size.height as f32,
      },
      path,
    );

    let rectangle_render_pipeline = rectangle::Rectangle::pipeline(&device);
    Ok(Self {
      window,
      size,
      surface,
      device,
      queue,
      swap_chain,
      staging_belt,
      local_spawner,
      local_pool,
      glyph_brush,
      rectangle_render_pipeline,
      fs_tree,
      font_height,
      code_views,
    })
  }

  pub fn resize(&mut self, size: PhysicalSize<f32>) {
    self.size = size.cast();

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

    for element in self.get_elements() {
      element.resize(size);
      element.scroll(PhysicalPosition { x: 0.0, y: 0.0 }, size);
    }
  }

  pub fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    mouse_pos: PhysicalPosition<f64>,
  ) {
    let self_size = self.size.cast();
    for element in self.get_elements() {
      if element
        .get_dimensions()
        .contains(mouse_pos.cast())
        .is_some()
      {
        element.scroll(offset, self_size);
        break;
      }
    }
  }

  pub fn click(
    &mut self,
    position: PhysicalPosition<f64>,
    state: ElementState,
  ) {
    if state == ElementState::Pressed {
      let size = self.size.cast();
      for element in self.get_elements() {
        if let Some(pos) = element.get_dimensions().contains(position.cast()) {
          element.click(pos.cast(), size);
          self.window.request_redraw();
          break;
        }
      }
    }
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
        color_attachments: &[wgpu::RenderPassColorAttachment {
          view: &frame.view,
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
      for rect in self.get_rects() {
        rect.write_buffer(&self.queue);
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

    self.code_views.redraw(
      &mut self.glyph_brush,
      &self.device,
      &mut self.staging_belt,
      &mut encoder,
      &frame.view,
      self.size,
    );

    self.fs_tree.redraw(
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

  fn get_rects(&self) -> Vec<&rectangle::Rectangle> {
    let mut vec = vec![];
    vec.extend(self.code_views.get_rects());
    vec.extend(self.fs_tree.get_rects());
    vec
  }

  fn get_elements(&mut self) -> Vec<&mut dyn RenderElement> {
    let mut vec: Vec<&mut dyn RenderElement> = vec![&mut self.fs_tree];
    vec.extend(self.code_views.get_elements());
    vec
  }
}

trait RenderElement {
  fn resize(&mut self, screen_size: PhysicalSize<f32>) {
    for element in self.get_elements() {
      element.resize(screen_size);
    }
  }

  fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    screen_size: PhysicalSize<f32>,
  ) {
    for element in self.get_elements() {
      element.scroll(offset, screen_size);
    }
  }

  fn click(
    &mut self,
    position: PhysicalPosition<f64>,
    screen_size: PhysicalSize<f32>,
  ) {
    for element in self.get_elements() {
      if let Some(pos) = element.get_dimensions().contains(position.cast()) {
        element.click(pos.cast(), screen_size);
        break;
      }
    }
  }

  fn redraw(
    &mut self,
    glyph_brush: &mut GlyphBrush<()>,
    device: &Device,
    staging_belt: &mut StagingBelt,
    encoder: &mut CommandEncoder,
    target: &TextureView,
    size: PhysicalSize<u32>,
  ) {
    for element in self.get_elements() {
      element.redraw(glyph_brush, device, staging_belt, encoder, target, size);
    }
  }

  fn get_rects(&self) -> Vec<&rectangle::Rectangle>;
  fn get_elements(&mut self) -> Vec<&mut dyn RenderElement>;
  fn get_dimensions(&self) -> Dimensions;
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Dimensions {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
}

impl Dimensions {
  fn contains(
    &self,
    position: PhysicalPosition<f32>,
  ) -> Option<PhysicalPosition<f32>> {
    if position.x >= self.x && position.y >= self.y {
      let end_pos = PhysicalPosition {
        x: self.x + self.width,
        y: self.y + self.height,
      };
      if position.x <= end_pos.x && position.y <= end_pos.y {
        Some(PhysicalPosition {
          x: position.x - self.x,
          y: position.y - self.y,
        })
      } else {
        None
      }
    } else {
      None
    }
  }
}

impl From<Dimensions> for wgpu_glyph::Region {
  fn from(item: Dimensions) -> Self {
    Self {
      x: item.x as u32,
      y: item.y as u32,
      width: item.width as u32,
      height: item.height as u32,
    }
  }
}

impl From<Dimensions> for rectangle::Region {
  fn from(item: Dimensions) -> Self {
    Self {
      x: item.x as u32,
      y: item.y as u32,
      width: item.width as u32,
      height: item.height as u32,
    }
  }
}
