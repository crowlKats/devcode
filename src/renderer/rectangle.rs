use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use winit::dpi::{PhysicalPosition, PhysicalSize};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
  position: [f32; 2],
  color: [f32; 3],
}

#[derive(Debug)]
pub struct Region {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

#[derive(Debug)]
pub struct Rectangle {
  pub vertex_buffer: wgpu::Buffer,
  vertices: [Vertex; 4],
  color: [f32; 3],
  pub region: Option<Region>,
  pub size: PhysicalSize<u32>,
  pub position: PhysicalPosition<f32>,
}

impl Rectangle {
  fn create_vertices(
    position: PhysicalPosition<f32>,
    end_position: PhysicalPosition<f32>,
    color: [f32; 3],
  ) -> [Vertex; 4] {
    [
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
    ]
  }

  pub fn pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: Some("Rectangle Shader Module"),
      source: wgpu::ShaderSource::Wgsl(Cow::from(include_str!(
        "./rectangle_shader.wgsl"
      ))),
      flags: wgpu::ShaderFlags::VALIDATION,
    });

    let render_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Rectangle Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
      });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Rectangle Render Pipeline"),
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
        targets: &[super::RENDER_FORMAT.into()],
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: Default::default(),
    })
  }

  fn calc_size(
    screen_size: PhysicalSize<u32>,
    position: PhysicalPosition<f32>,
    size: PhysicalSize<u32>,
  ) -> (PhysicalPosition<f32>, PhysicalPosition<f32>) {
    (
      PhysicalPosition {
        x: ((position.x / (screen_size.width as f32)) * 2.0) - 1.0,
        y: ((position.y / (screen_size.height as f32)) * 2.0) - 1.0,
      },
      PhysicalPosition {
        x: ((size.width as f32) / (screen_size.width as f32)) * 2.0,
        y: ((size.height as f32) / (screen_size.height as f32)) * 2.0,
      },
    )
  }

  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    position: PhysicalPosition<f32>,
    size: PhysicalSize<u32>,
    color: [f32; 3],
    region: Option<Region>,
  ) -> Self {
    let (pos, end_pos) = Self::calc_size(screen_size, position, size);
    let vertices = Self::create_vertices(pos, end_pos, color);

    let vertex_buffer =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Rectangle Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
      });

    Self {
      vertex_buffer,
      vertices,
      color,
      region,
      size,
      position,
    }
  }

  pub fn resize(
    &mut self,
    screen_size: PhysicalSize<u32>,
    position: PhysicalPosition<f32>,
    size: PhysicalSize<u32>,
  ) {
    self.position = position;
    self.size = size;
    let (pos, end_pos) = Self::calc_size(screen_size, position, size);
    self.vertices = Self::create_vertices(pos, end_pos, self.color);
  }

  pub fn write_buffer(&self, queue: &wgpu::Queue) {
    queue.write_buffer(
      &self.vertex_buffer,
      0,
      bytemuck::cast_slice(&self.vertices),
    );
  }
}
