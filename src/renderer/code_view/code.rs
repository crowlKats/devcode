use super::super::input::{max_line_length, Cursor};
use super::super::rectangle::Rectangle;
use crate::renderer::Dimensions;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphPositioner, Layout, Section, SectionGeometry, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

pub struct Code {
  font: FontArc,
  font_height: f32,
  text: Rc<RefCell<ropey::Rope>>,
  scroll_offset: PhysicalPosition<f64>,
  cursor: Cursor,
  max_line_length: f32,
  pub dimensions: Dimensions,
}

impl Code {
  fn generate_glyph_text<'r>(
    &self,
    text: impl Iterator<Item = ropey::RopeSlice<'r>>,
  ) -> Vec<Text<'r>> {
    text
      .flat_map(|s| {
        s.chunks().map(|c| {
          Text::new(c)
            .with_color([0.9, 0.9, 0.9, 1.0])
            .with_scale(self.font_height)
        })
      })
      .collect()
  }

  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<f32>,
    font: FontArc,
    font_height: f32,
    dimensions: Dimensions,
    text: Rc<RefCell<ropey::Rope>>,
  ) -> Self {
    let cursor = Cursor::new(
      device,
      screen_size,
      Dimensions {
        width: 4.0,
        height: font_height,
        ..dimensions
      },
      [0.7, 0.0, 0.0],
      Some(dimensions.into()),
    );

    let max_line_length = max_line_length(
      text.borrow().lines().map(|s| s.to_string()),
      font.clone(),
      font_height,
    );

    Self {
      font,
      font_height,
      text,
      scroll_offset: PhysicalPosition { x: 0.0, y: 0.0 },
      cursor,
      max_line_length,
      dimensions,
    }
  }
}

impl super::super::input::TextInput for Code {
  fn input_special(
    &mut self,
    screen_size: PhysicalSize<f32>,
    key: VirtualKeyCode,
  ) {
    super::super::input::input_special(
      screen_size,
      key,
      &mut self.text.borrow_mut(),
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition {
        x: self.dimensions.x,
        y: 0.0,
      },
      self.scroll_offset.cast(),
    );
  }

  fn input_char(&mut self, screen_size: PhysicalSize<f32>, ch: char) {
    self.max_line_length = super::super::input::input_char(
      screen_size,
      ch,
      &mut self.text.borrow_mut(),
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition {
        x: self.dimensions.x,
        y: 0.0,
      },
      self.scroll_offset.cast(),
    );
  }
}

impl super::super::RenderElement for Code {
  fn resize(&mut self, screen_size: PhysicalSize<f32>) {
    self.dimensions.width = screen_size.width - self.dimensions.x;

    self.cursor.rect.resize(
      screen_size.cast(),
      Dimensions {
        y: self.font_height - (self.cursor.row as f32 * self.font_height),
        ..self.cursor.rect.dimensions
      },
    );

    self.cursor.rect.region = Some(self.dimensions.into());
  }

  fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    screen_size: PhysicalSize<f32>,
  ) {
    if offset.x.abs() > offset.y.abs() {
      self.scroll_offset.x = (self.scroll_offset.x - offset.x)
        .max((screen_size.width - self.max_line_length) as f64) // TODO
        .min(0.0);
    } else {
      self.scroll_offset.y = (self.scroll_offset.y + offset.y).min(0.0).max(
        -((self.text.borrow().len_lines() - 3) as f32 * self.font_height)
          as f64,
      );
    }

    self.cursor.rect.resize(
      screen_size,
      Dimensions {
        x: self.dimensions.x
          + self.scroll_offset.x as f32
          + self.cursor.x_offset,
        y: self.dimensions.y
          + self.scroll_offset.y as f32
          + (self.cursor.row as f32 * self.font_height),
        ..self.cursor.rect.dimensions
      },
    );
  }

  fn click(
    &mut self,
    position: PhysicalPosition<f64>,
    _screen_size: PhysicalSize<f32>,
  ) {
    let line = ((position.y - self.scroll_offset.y) / self.font_height as f64)
      .floor() as usize;
    let text = self.text.borrow();
    let lines = self.generate_glyph_text(text.lines_at(line).take(1));
    let layout = Layout::default_wrap();

    let section_glyphs = &layout.calculate_glyphs(
      &[self.font.clone()],
      &SectionGeometry {
        ..Default::default()
      },
      lines.as_slice(),
    );

    let mut c = 0;
    for section_glyph in section_glyphs {
      c += 1;
      self.cursor.x_offset = section_glyph.glyph.position.x;
      if (position.x as f32) < section_glyph.glyph.position.x {
        c -= 1;
        break;
      }
    }

    self.cursor.row = line;
    self.cursor.column = c;
  }

  fn redraw(
    &mut self,
    glyph_brush: &mut wgpu_glyph::GlyphBrush<()>,
    device: &wgpu::Device,
    staging_belt: &mut wgpu::util::StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    size: PhysicalSize<u32>,
  ) {
    let upper_bound =
      ((-self.scroll_offset.y) / self.font_height as f64).floor() as usize;
    let lower_bound = (upper_bound
      + (self.dimensions.height / self.font_height).ceil() as usize)
      .min(self.text.borrow().len_lines());

    let text = self.text.borrow();
    let lines = text.lines_at(upper_bound).take(lower_bound - upper_bound);
    glyph_brush.queue(Section {
      screen_position: (
        self.dimensions.x + self.scroll_offset.x as f32,
        -(((-self.scroll_offset.y as f32) % self.font_height)
          - self.dimensions.y),
      ),
      text: self.generate_glyph_text(lines),
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
    vec![&self.cursor.rect]
  }

  fn get_elements(&mut self) -> Vec<&mut dyn super::super::RenderElement> {
    vec![]
  }

  fn get_dimensions(&self) -> Dimensions {
    self.dimensions
  }
}
