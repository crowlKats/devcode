use crate::renderer::rectangle::Rectangle;
use wgpu_glyph::ab_glyph::Rect;
use wgpu_glyph::{HorizontalAlign, Layout, Region, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct CodeView {
  text: String,
  scroll_offset: winit::dpi::PhysicalPosition<f64>,
  font_size: Rect,
  pub rect: Rectangle,
  pub cursor: Rectangle,
  pub cursor_line: u32,
  pub cursor_column: u32,
  line_numbers_width: f32,
}

impl CodeView {
  pub fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.cursor, &self.rect]
  }

  pub fn new(
    text: String,
    font_size: Rect,
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
  ) -> Self {
    let line_count_digits_len =
      (text.lines().count() as f32).log10().floor() + 1.0;
    let line_numbers_width = line_count_digits_len * font_size.width();

    let rect = Rectangle::new(
      device,
      screen_size,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalSize {
        width: line_numbers_width as u32 + 10,
        height: screen_size.height,
      },
      [0.05, 0.05, 0.05],
    );

    let mut cursor = Rectangle::new(
      device,
      screen_size,
      PhysicalPosition {
        x: line_numbers_width + 20.0,
        y: screen_size.height as f32 - font_size.height(),
      },
      PhysicalSize {
        width: 6,
        height: font_size.height() as u32,
      },
      [0.7, 0.0, 0.0],
    );
    cursor.region = Some(Region {
      x: line_numbers_width as u32 + 20,
      y: 0,
      width: screen_size.width - (line_numbers_width as u32 + 20),
      height: screen_size.height,
    });

    Self {
      text,
      scroll_offset: winit::dpi::PhysicalPosition { x: 0.0, y: 0.0 },
      font_size,
      rect,
      cursor,
      cursor_line: 0,
      cursor_column: 0,
      line_numbers_width,
    }
  }

  pub fn input(&mut self, size: PhysicalSize<u32>) {
    self.cursor.resize(
      size,
      PhysicalPosition {
        x: self.line_numbers_width
          + 20.0
          + (self.cursor_column as f32 * self.font_size.width()),
        y: size.height as f32
          - self.font_size.height()
          - (self.cursor_line as f32 * self.font_size.height()),
      },
      self.cursor.size,
    );
  }
}

impl super::RenderElement for CodeView {
  fn resize(&mut self, screen_size: PhysicalSize<u32>) {
    self.rect.resize(
      screen_size,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalSize {
        width: self.line_numbers_width as u32 + 10,
        height: screen_size.height,
      },
    );

    self.cursor.resize(
      screen_size,
      PhysicalPosition {
        x: self.line_numbers_width
          + 20.0
          + (self.cursor_column as f32 * self.font_size.width()),
        y: screen_size.height as f32
          - self.font_size.height()
          - (self.cursor_line as f32 * self.font_size.height()),
      },
      self.cursor.size,
    );

    self.cursor.region = Some(Region {
      x: self.line_numbers_width as u32 + 20,
      y: 0,
      width: screen_size.width - (self.line_numbers_width as u32 + 20),
      height: screen_size.height,
    });
  }

  fn scroll(&mut self, offset: PhysicalPosition<f64>, size: PhysicalSize<u32>) {
    let mut line_count = 0;
    let mut max_line_length = 0;
    for line in self.text.lines() {
      line_count += 1;
      if line.len() > max_line_length {
        max_line_length = line.len();
      }
    }
    if self.text.ends_with('\n') {
      line_count += 1;
    }

    let max_width = max_line_length as f64 * self.font_size.width() as f64;

    self.scroll_offset.x = (self.scroll_offset.x - offset.x)
      .max(
        (self.line_numbers_width as f64 + 20.0)
          + (size.width as f64 - max_width),
      )
      .min(0.0);
    self.scroll_offset.y = (self.scroll_offset.y + offset.y)
      .min(0.0)
      .max(-((line_count - 3) as f32 * self.font_size.height()) as f64);

    self.cursor.resize(
      size,
      PhysicalPosition {
        x: (self.line_numbers_width + 20.0)
          + self.scroll_offset.x as f32
          + (self.cursor_column as f32 * self.font_size.width()),
        y: size.height as f32
          - self.font_size.height()
          - self.scroll_offset.y as f32
          - (self.cursor_line as f32 * self.font_size.height()),
      },
      self.cursor.size,
    );
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
    let mut line_count = 0;
    let mut line_numbers = String::new();
    for _ in self.text.lines() {
      line_count += 1;
      line_numbers += &format!("{}\n", line_count);
    }
    if self.text.ends_with('\n') {
      line_count += 1;
      line_numbers += &format!("{}\n", line_count);
    }

    glyph_brush.queue(Section {
      screen_position: (self.line_numbers_width, self.scroll_offset.y as f32),
      text: vec![Text::new(&line_numbers)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_size.height())],
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

    let codeview_offset = self.line_numbers_width + 20.0;
    glyph_brush.queue(Section {
      screen_position: (
        codeview_offset + self.scroll_offset.x as f32,
        self.scroll_offset.y as f32,
      ),
      text: vec![Text::new(&self.text)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_size.height())],
      ..Section::default()
    });

    glyph_brush
      .draw_queued_with_transform_and_scissoring(
        device,
        staging_belt,
        encoder,
        target,
        wgpu_glyph::orthographic_projection(size.width, size.height),
        wgpu_glyph::Region {
          x: codeview_offset as u32,
          y: 0,
          width: size.width - codeview_offset as u32,
          height: size.height,
        },
      )
      .unwrap();
  }
}
