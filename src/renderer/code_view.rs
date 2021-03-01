use wgpu_glyph::ab_glyph::Rect;
use wgpu_glyph::{HorizontalAlign, Layout, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct CodeView {
  text: String,
  scroll_offset: winit::dpi::PhysicalPosition<f64>,
  font_size: Rect,
  rect: super::Rectangle,
}

impl CodeView {
  fn get_line_number_width(count: usize, font_width: f32) -> f32 {
    let line_count_digits_len = (count as f32).log10().floor() + 1.0;
    line_count_digits_len * font_width
  }

  pub fn new(
    text: String,
    font_size: Rect,
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
  ) -> Self {
    let line_numbers_width =
      CodeView::get_line_number_width(text.lines().count(), font_size.width());
    let (pos, end_pos) = super::calc_size(
      screen_size,
      PhysicalPosition { x: 0, y: 0 },
      PhysicalSize {
        width: line_numbers_width as u32 + 10,
        height: screen_size.height,
      },
    );
    let rect = super::Rectangle::new(device, pos, end_pos, [0.05, 0.05, 0.05]);

    Self {
      text,
      scroll_offset: winit::dpi::PhysicalPosition { x: 0f64, y: 0f64 },
      font_size,
      rect,
    }
  }

  pub fn rpass<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
    self.rect.render(render_pass);
  }
}

impl super::RenderElement for CodeView {
  fn resize(&mut self, _size: PhysicalSize<u32>) {
    unimplemented!()
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
    let line_numbers_width =
      CodeView::get_line_number_width(line_count, self.font_size.width());
    self.scroll_offset.x = (self.scroll_offset.x - offset.x)
      .max((line_numbers_width as f64 + 20.0) + (size.width as f64 - max_width))
      .min(0.0);
    self.scroll_offset.y = (self.scroll_offset.y + offset.y)
      .min(0.0)
      .max(-((line_count - 3) as f32 * self.font_size.height()) as f64);
  }

  fn redraw(&mut self, glyph_brush: &mut wgpu_glyph::GlyphBrush<()>) {
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

    let line_numbers_width =
      CodeView::get_line_number_width(line_count, self.font_size.width());
    glyph_brush.queue(Section {
      screen_position: (line_numbers_width, self.scroll_offset.y as f32),
      text: vec![Text::new(&line_numbers)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_size.height())],
      layout: Layout::default_wrap().h_align(HorizontalAlign::Right),
      ..Section::default()
    });

    glyph_brush.queue(Section {
      screen_position: (
        line_numbers_width + 20.0 + self.scroll_offset.x as f32,
        self.scroll_offset.y as f32,
      ),
      text: vec![Text::new(&self.text)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_size.height())],
      ..Section::default()
    });
  }
}
