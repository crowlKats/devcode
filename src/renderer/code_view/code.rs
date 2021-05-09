use super::super::input::{max_line_length, Cursor};
use super::super::rectangle::{Rectangle, Region};
use crate::renderer::Dimensions;
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphPositioner, Layout, Section, SectionGeometry, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

pub struct Code {
  font: FontArc,
  font_height: f32,
  text: Rc<RefCell<Vec<String>>>,
  scroll_offset: PhysicalPosition<f64>,
  cursor: Cursor,
  max_line_length: f32,
  pub dimensions: Dimensions,
}

impl Code {
  fn generate_glyph_text<'r>(
    &self,
    text: &'r Ref<'_, [String]>,
  ) -> Vec<Text<'r>> {
    text
      .iter()
      .flat_map(|s| {
        std::iter::once(
          Text::new(s)
            .with_color([0.9, 0.9, 0.9, 1.0])
            .with_scale(self.font_height),
        )
        .chain(std::iter::once(
          Text::new("\n").with_scale(self.font_height),
        ))
      })
      .collect()
  }

  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    font: FontArc,
    font_height: f32,
    dimensions: Dimensions,
    text: Rc<RefCell<Vec<String>>>,
  ) -> Self {
    let cursor = Cursor::new(
      device,
      screen_size,
      Dimensions {
        x: dimensions.x,
        y: screen_size.height as f32 - font_height,
        width: 4.0,
        height: font_height,
      },
      [0.7, 0.0, 0.0],
      Some(Region {
        x: 0,
        y: 0,
        width: screen_size.width,
        height: screen_size.height,
      }),
    );

    let max_line_length =
      max_line_length(&text.borrow(), font.clone(), font_height);

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
  fn input_special(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    super::super::input::input_special(
      size,
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

  fn input_char(&mut self, size: PhysicalSize<u32>, ch: char) {
    self.max_line_length = super::super::input::input_char(
      size,
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
    self.cursor.rect.resize(
      screen_size.cast(),
      Dimensions {
        y: screen_size.height
          - self.font_height
          - (self.cursor.row as f32 * self.font_height),
        ..self.cursor.rect.dimensions
      },
    );

    self.cursor.rect.region = Some(Region {
      x: self.dimensions.x as u32,
      y: 0,
      width: (screen_size.width - self.dimensions.x) as u32,
      height: screen_size.height as u32,
    });

    self.dimensions.width = screen_size.width;
    self.dimensions.height = screen_size.height;
  }

  fn scroll(&mut self, offset: PhysicalPosition<f64>, size: PhysicalSize<f32>) {
    if offset.x.abs() > offset.y.abs() {
      self.scroll_offset.x = (self.scroll_offset.x - offset.x)
        .max(size.width as f64 - self.max_line_length as f64) // TODO
        .min(0.0);
    } else {
      self.scroll_offset.y = (self.scroll_offset.y + offset.y).min(0.0).max(
        -((self.text.borrow().len() - 3) as f32 * self.font_height) as f64,
      );
    }

    self.cursor.rect.resize(
      size.cast(),
      Dimensions {
        x: self.dimensions.x
          + self.scroll_offset.x as f32
          + self.cursor.x_offset,
        y: size.height as f32
          - self.font_height
          - self.scroll_offset.y as f32
          - (self.cursor.row as f32 * self.font_height),
        ..self.cursor.rect.dimensions
      },
    );
  }

  fn click(&mut self, position: PhysicalPosition<f64>) {
    let line = ((position.y - self.scroll_offset.y) / self.font_height as f64)
      .floor() as usize;
    let vec = Ref::map(self.text.borrow(), |v| v[line..line + 1].as_ref());
    let text = self.generate_glyph_text(&vec)[0];
    let layout = Layout::default_wrap();

    let section_glyphs = &layout.calculate_glyphs(
      &[self.font.clone()],
      &SectionGeometry {
        ..Default::default()
      },
      &[text],
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
      .min(self.text.borrow().len());

    let vec =
      Ref::map(self.text.borrow(), |v| v[upper_bound..lower_bound].as_ref());
    glyph_brush.queue(Section {
      screen_position: (
        self.dimensions.x + self.scroll_offset.x as f32,
        -((-self.scroll_offset.y as f32) % self.font_height),
      ),
      text: self.generate_glyph_text(&vec),
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
          x: self.dimensions.x as u32,
          y: 0,
          width: size.width - self.dimensions.x as u32,
          height: size.height,
        },
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
