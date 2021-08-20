use crate::renderer::rectangle::Rectangle;
use crate::renderer::Dimensions;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu_glyph::ab_glyph::FontArc;
use winit::dpi::PhysicalSize;
use winit::event::VirtualKeyCode;

mod code;
mod gutter;

pub struct CodeView {
  #[allow(dead_code)]
  text: Rc<RefCell<ropey::Rope>>,
  gutter: gutter::Gutter,
  code: code::Code,
  pub dimensions: Dimensions,
}

impl CodeView {
  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<f32>,
    font: FontArc,
    font_height: f32,
    dimensions: Dimensions,
    text: ropey::Rope,
  ) -> Self {
    let text = Rc::new(RefCell::new(text));

    let gutter = gutter::Gutter::new(
      device,
      font.clone(),
      font_height,
      screen_size,
      dimensions,
      Rc::clone(&text),
    );

    let code = code::Code::new(
      device,
      screen_size,
      font,
      font_height,
      Dimensions {
        x: dimensions.x + gutter.dimensions.width,
        width: dimensions.width - gutter.dimensions.width,
        ..dimensions
      },
      Rc::clone(&text),
    );

    Self {
      text,
      gutter,
      code,
      dimensions,
    }
  }
}

impl super::input::TextInput for CodeView {
  fn input_special(
    &mut self,
    screen_size: PhysicalSize<f32>,
    key: VirtualKeyCode,
  ) {
    self.code.input_special(screen_size, key);
  }

  fn input_char(&mut self, screen_size: PhysicalSize<f32>, ch: char) {
    self.code.input_char(screen_size, ch);
  }
}

impl super::RenderElement for CodeView {
  fn get_rects(&self) -> Vec<&Rectangle> {
    let mut vec = vec![];
    vec.extend(self.gutter.get_rects());
    vec.extend(self.code.get_rects());
    vec
  }

  fn get_elements(&mut self) -> Vec<&mut dyn super::RenderElement> {
    vec![&mut self.gutter, &mut self.code]
  }

  fn get_dimensions(&self) -> Dimensions {
    self.dimensions
  }
}
