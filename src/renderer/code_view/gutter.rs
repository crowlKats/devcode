use crate::renderer::input::max_line_length;
use crate::renderer::rectangle::Rectangle;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu::util::StagingBelt;
use wgpu::{CommandEncoder, Device, TextureView};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrush, HorizontalAlign, Layout, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct Gutter {
  text: Rc<RefCell<Vec<String>>>,
  rect: Rectangle,
  position: PhysicalPosition<u32>,
  pub size: PhysicalSize<u32>,
  scroll_offset_y: f64,
  font_height: f32,
}

impl Gutter {
  pub fn new(
    device: &wgpu::Device,
    font: FontArc,
    font_height: f32,
    position: PhysicalPosition<u32>,
    size: PhysicalSize<u32>,
    text: Rc<RefCell<Vec<String>>>,
  ) -> Self {
    let line_numbers = text
      .borrow()
      .iter()
      .enumerate()
      .map(|(i, _)| i.to_string())
      .collect::<Vec<String>>();
    let line_numbers_width = max_line_length(&line_numbers, font, font_height);

    let rect = Rectangle::new(
      device,
      size,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalSize {
        width: position.x + (line_numbers_width as u32 + 10),
        height: size.height,
      },
      [0.05, 0.05, 0.05],
      None,
    );

    let size = PhysicalSize {
      width: line_numbers_width as u32 + 20,
      height: size.height,
    };

    Self {
      text,
      position,
      size,
      rect,
      font_height,
      scroll_offset_y: 0.0,
    }
  }
}

impl super::super::RenderElement for Gutter {
  fn resize(&mut self, screen_size: PhysicalSize<u32>) {
    self.rect.resize(
      screen_size,
      PhysicalPosition {
        x: self.position.x as f32,
        y: 0.0,
      },
      PhysicalSize {
        width: self.size.width,
        height: screen_size.height,
      },
    );
  }

  fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    _size: PhysicalSize<u32>,
  ) {
    self.scroll_offset_y = (self.scroll_offset_y + offset.y)
      .min(0.0)
      .max(-((self.text.borrow().len() - 3) as f32 * self.font_height) as f64);
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
    let upper_bound =
      ((-self.scroll_offset_y) / self.font_height as f64).floor() as usize;
    let lower_bound = (upper_bound
      + ((self.size.height as f64) as f32 / self.font_height).ceil() as usize)
      .min(self.text.borrow().len());

    let mut line_count = upper_bound;
    let mut line_numbers = String::new();
    for _ in &self.text.borrow()[upper_bound..lower_bound] {
      line_count += 1;
      line_numbers += &format!("{}\n", line_count);
    }

    glyph_brush.queue(Section {
      screen_position: (
        (self.position.x + (self.size.width - 20)) as f32,
        -((-self.scroll_offset_y as f32) % self.font_height),
      ),
      text: vec![Text::new(&line_numbers)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_height)],
      layout: Layout::default_wrap().h_align(HorizontalAlign::Right),
      ..Section::default()
    });

    glyph_brush
      .draw_queued(
        device,
        staging_belt,
        encoder,
        target,
        size.width,
        size.height,
      )
      .unwrap();
  }

  fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.rect]
  }

  fn get_elements(&mut self) -> Vec<&mut dyn super::super::RenderElement> {
    vec![]
  }

  fn get_pos_size(&self) -> (PhysicalPosition<u32>, PhysicalSize<u32>) {
    (self.position, self.size)
  }
}
