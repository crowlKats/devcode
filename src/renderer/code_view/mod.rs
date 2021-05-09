use crate::renderer::rectangle::Rectangle;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu_glyph::ab_glyph::FontArc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

mod code;
mod gutter;

pub struct CodeView {
  #[allow(dead_code)]
  text: Rc<RefCell<Vec<String>>>,
  gutter: gutter::Gutter,
  code: code::Code,
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

    let code = code::Code::new(
      device,
      screen_size,
      font,
      font_height,
      PhysicalPosition {
        x: position.x + gutter.size.width,
        y: position.y,
      },
      PhysicalSize {
        width: size.width - gutter.size.width,
        height: size.height,
      },
      Rc::clone(&text),
    );

    Self {
      code,
      gutter,
      text,
      position,
      size,
    }
  }
}

impl super::input::TextInput for CodeView {
  fn input_special(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    self.code.input_special(size, key);
  }

  fn input_char(&mut self, size: PhysicalSize<u32>, ch: char) {
    self.code.input_char(size, ch);
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

  fn get_pos_size(&self) -> (PhysicalPosition<u32>, PhysicalSize<u32>) {
    (self.position, self.size)
  }
}
