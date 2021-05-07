use crate::renderer::position_in_obj;
use crate::renderer::rectangle::Rectangle;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu::util::StagingBelt;
use wgpu::{CommandEncoder, Device, TextureView};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::GlyphBrush;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

mod code_view;
mod gutter;

pub struct CodeView {
  #[allow(dead_code)]
  text: Rc<RefCell<Vec<String>>>,
  gutter: gutter::Gutter,
  code_view: code_view::CodeView,
  pub position: PhysicalPosition<u32>,
  pub size: PhysicalSize<u32>,
}

impl CodeView {
  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    font: FontArc,
    font_height: f32,
    position: PhysicalPosition<u32>,
    size: PhysicalSize<u32>,
    text: String,
  ) -> Self {
    let mut split_text =
      text.lines().map(|s| s.to_string()).collect::<Vec<String>>();
    if text.ends_with('\n') {
      split_text.push(String::from(""));
    }

    let text = Rc::new(RefCell::new(split_text));

    let gutter = gutter::Gutter::new(
      device,
      font.clone(),
      font_height,
      position,
      size,
      Rc::clone(&text),
    );

    let code_view = code_view::CodeView::new(
      device,
      screen_size,
      font.clone(),
      font_height,
      PhysicalPosition {
        x: position.x + gutter.size.width,
        y: position.y,
      },
      size,
      Rc::clone(&text),
    );

    Self {
      code_view,
      gutter,
      text,
      position,
      size,
    }
  }
}

impl super::input::TextInput for CodeView {
  fn input_special(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    self.code_view.input_special(size, key);
  }

  fn input_char(&mut self, size: PhysicalSize<u32>, ch: char) {
    self.code_view.input_char(size, ch);
  }
}

impl super::RenderElement for CodeView {
  fn resize(&mut self, size: PhysicalSize<u32>) {
    self.gutter.resize(size);
    self.code_view.resize(size);
  }

  fn scroll(&mut self, offset: PhysicalPosition<f64>, size: PhysicalSize<u32>) {
    self.gutter.scroll(offset, size);
    self.code_view.scroll(offset, size);
  }

  fn click(&mut self, position: PhysicalPosition<f64>) {
    let (pos, size) = self.gutter.get_pos_size();
    if let Some(pos) = position_in_obj(position.cast(), pos, size) {
      self.gutter.click(pos.cast());
    } else {
      let (pos, size) = self.code_view.get_pos_size();
      if let Some(pos) = position_in_obj(position.cast(), pos, size) {
        self.code_view.click(pos.cast());
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
    self.gutter.redraw(
      glyph_brush,
      device,
      staging_belt,
      encoder,
      target,
      size,
    );
    self.code_view.redraw(
      glyph_brush,
      device,
      staging_belt,
      encoder,
      target,
      size,
    );
  }

  fn get_rects(&self) -> Vec<&Rectangle> {
    let mut vec = vec![];
    vec.extend(self.gutter.get_rects());
    vec.extend(self.code_view.get_rects());
    vec
  }

  fn get_pos_size(&self) -> (PhysicalPosition<u32>, PhysicalSize<u32>) {
    (self.position, self.size)
  }
}
