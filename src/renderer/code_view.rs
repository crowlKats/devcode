use wgpu_glyph::{Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct CodeView {
  text: String,
  scroll_offset: winit::dpi::PhysicalPosition<f64>,
  font_height: f32,
}

impl CodeView {
  pub fn new(text: String) -> Self {
    CodeView {
      text,
      scroll_offset: winit::dpi::PhysicalPosition { x: 0f64, y: 0f64 },
      font_height: 40.0,
    }
  }
}

impl super::RenderElement for CodeView {
  fn resize(&mut self, _size: PhysicalSize<u32>) {
    unimplemented!()
  }

  fn scroll(&mut self, offset: PhysicalPosition<f64>) {
    let line_count = self.text.lines().count() as f32;

    self.scroll_offset.x = (self.scroll_offset.x + offset.x).min(0f64);
    self.scroll_offset.y = (self.scroll_offset.y + offset.y)
      .min(0f64)
      .max(-((line_count - 3.0) * self.font_height) as f64);
  }

  fn redraw(&mut self, glyph_brush: &mut wgpu_glyph::GlyphBrush<()>) {
    glyph_brush.queue(Section {
      screen_position: (20.0, self.scroll_offset.y as f32),
      text: vec![Text::new(&self.text)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_height)],
      ..Section::default()
    });
  }
}
