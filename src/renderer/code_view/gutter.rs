use crate::renderer::input::max_line_length;
use crate::renderer::rectangle::Rectangle;
use crate::renderer::Dimensions;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu::util::StagingBelt;
use wgpu::{CommandEncoder, Device, TextureView};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrush, HorizontalAlign, Layout, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};

const GUTTER_MARGIN: f32 = 10.0;
const GUTTER_PADDING: f32 = 10.0;

pub struct Gutter {
  text: Rc<RefCell<ropey::Rope>>,
  rect: Rectangle,
  pub dimensions: Dimensions,
  scroll_offset_y: f64,
  font_height: f32,
}

impl Gutter {
  pub fn new(
    device: &wgpu::Device,
    font: FontArc,
    font_height: f32,
    screen_size: PhysicalSize<f32>,
    dimensions: Dimensions,
    text: Rc<RefCell<ropey::Rope>>,
  ) -> Self {
    let line_numbers =
      (0..(text.borrow().len_lines() - 1)).map(|i| i.to_string());
    let line_numbers_width = max_line_length(line_numbers, font, font_height);

    let rect_size = line_numbers_width + GUTTER_PADDING;

    let rect = Rectangle::new(
      device,
      screen_size,
      Dimensions {
        width: rect_size,
        ..dimensions
      },
      [0.5, 0.05, 0.05],
      None,
    );

    Self {
      text,
      dimensions: Dimensions {
        width: rect_size + GUTTER_MARGIN,
        ..dimensions
      },
      rect,
      font_height,
      scroll_offset_y: 0.0,
    }
  }
}

impl super::super::RenderElement for Gutter {
  fn resize(&mut self, screen_size: PhysicalSize<f32>) {
    self.rect.resize(
      screen_size.cast(),
      Dimensions {
        width: self.dimensions.width - GUTTER_MARGIN,
        ..self.dimensions
      },
    );
  }

  fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    _screen_size: PhysicalSize<f32>,
  ) {
    self.scroll_offset_y = (self.scroll_offset_y + offset.y).min(0.0).max(
      -((self.text.borrow().len_lines() - 3) as f32 * self.font_height) as f64,
    );
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
      + (self.dimensions.height / self.font_height).ceil() as usize)
      .min(self.text.borrow().len_lines());

    let mut line_count = upper_bound;
    let mut line_numbers = String::new();
    for _ in self
      .text
      .borrow()
      .lines_at(upper_bound)
      .take(lower_bound - upper_bound)
    {
      // TODO
      line_count += 1;
      line_numbers += &format!("{}\n", line_count);
    }

    glyph_brush.queue(Section {
      screen_position: (
        (self.dimensions.x
          + (self.dimensions.width - (GUTTER_PADDING + GUTTER_MARGIN))),
        -(((-self.scroll_offset_y as f32) % self.font_height)
          - self.dimensions.y),
      ),
      text: vec![Text::new(&line_numbers)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_height)],
      layout: Layout::default_wrap().h_align(HorizontalAlign::Right),
      ..Section::default()
    });

    glyph_brush
      .draw_queued_with_transform_and_scissoring(
        device,
        staging_belt,
        encoder,
        target,
        wgpu_glyph::orthographic_projection(size.width, size.height),
        self.dimensions.into(),
      )
      .unwrap();
  }

  fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.rect]
  }

  fn get_elements(&mut self) -> Vec<&mut dyn super::super::RenderElement> {
    vec![]
  }

  fn get_dimensions(&self) -> Dimensions {
    self.dimensions
  }
}
