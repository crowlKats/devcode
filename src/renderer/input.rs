use crate::renderer::rectangle::{Rectangle, Region};
use unicode_segmentation::UnicodeSegmentation;
use wgpu::util::StagingBelt;
use wgpu::{CommandEncoder, Device, TextureView};
use wgpu_glyph::ab_glyph::{Font, FontArc};
use wgpu_glyph::{GlyphBrush, GlyphPositioner, Layout, SectionGeometry, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

#[derive(Debug)]
pub struct Cursor {
  pub rect: Rectangle,
  pub row: usize,
  pub column: usize,
  pub x_offset: f32,
}

impl Cursor {
  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    position: PhysicalPosition<f32>,
    size: PhysicalSize<u32>,
    color: [f32; 3],
    region: Option<Region>,
  ) -> Self {
    let rect =
      Rectangle::new(device, screen_size, position, size, color, region);
    Self {
      rect,
      row: 0,
      column: 0,
      x_offset: 0.0,
    }
  }
}

pub trait TextInput {
  fn input_special(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode);
  fn input_char(&mut self, size: PhysicalSize<u32>, ch: char);
}

pub struct TextArea {
  cursor: Cursor,
  font: FontArc,
  font_height: f32,
  max_line_length: f32,
  text: Vec<String>,
  _multiline: Option<f32>,
}

impl TextArea {
  pub fn _new(
    font: FontArc,
    text: String,
    font_height: f32,
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    multiline: Option<f32>,
  ) -> Self {
    let mut split_text =
      text.lines().map(|s| s.to_string()).collect::<Vec<String>>();
    if multiline.is_some() && text.ends_with('\n') {
      split_text.push(String::from(""));
    }

    if multiline.is_none() {
      assert_eq!(split_text.len(), 1);
    }

    // TODO: bounding rect

    let cursor = Cursor::new(
      device,
      screen_size,
      PhysicalPosition {
        x: 0.0,
        y: screen_size.height as f32 - font_height,
      },
      PhysicalSize {
        width: 1,
        height: font_height as u32,
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
      max_line_length(&split_text, font.clone(), font_height);

    Self {
      text: split_text,
      cursor,
      font,
      font_height,
      max_line_length,
      _multiline: multiline,
    }
  }
}

pub fn max_line_length(
  lines: &[String],
  font: FontArc,
  font_height: f32,
) -> f32 {
  let mut max_line_width = 0.0;
  let layout = Layout::default_wrap();
  for line in lines {
    let text = Text::new(line).with_scale(font_height);
    let section_glyphs = layout.calculate_glyphs(
      &[font.clone()],
      &SectionGeometry {
        ..Default::default()
      },
      &[text],
    );

    if let Some(section_glyph) = section_glyphs.last() {
      let width = section_glyph.glyph.position.x
        + font.glyph_bounds(&section_glyph.glyph).width();

      if width > max_line_width {
        max_line_width = width;
      }
    }
  }

  max_line_width
}

fn cursor_x_position(
  row: usize,
  column: usize,
  text: &[String],
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
) -> Option<f32> {
  let text = Text::new(&text[row]).with_scale(font_height);
  let layout = Layout::default_wrap();

  let section_glyphs = layout.calculate_glyphs(
    &[font.clone()],
    &SectionGeometry {
      screen_position: (offset.x, offset.y),
      ..Default::default()
    },
    &[text],
  );

  if let Some(section_glyph) = section_glyphs.get(column) {
    Some(section_glyph.glyph.position.x)
  } else if column != 0 {
    if let Some(section_glyph) = section_glyphs.get(column - 1) {
      Some(
        section_glyph.glyph.position.x
          + font.glyph_bounds(&section_glyph.glyph).width(),
      )
    } else {
      None
    }
  } else {
    None
  }
}

#[allow(clippy::too_many_arguments)]
pub fn input_special(
  size: PhysicalSize<u32>,
  key: VirtualKeyCode,
  text: &mut Vec<String>,
  cursor: &mut Cursor,
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
  scroll_offset: PhysicalPosition<f32>,
) {
  let cursor_x_position2 = |row: usize, column: usize| {
    cursor_x_position(
      row,
      column,
      text,
      font.clone(),
      font_height,
      scroll_offset,
    )
  };

  match key {
    VirtualKeyCode::Up => {
      if cursor.row != 0 {
        cursor.row -= 1;
        if let Some(offset) = cursor_x_position2(cursor.row, cursor.column) {
          cursor.x_offset = offset;
        } else {
          cursor.column = text[cursor.row].len();
          cursor.x_offset =
            cursor_x_position2(cursor.row, cursor.column).unwrap_or(0.0);
        }
      } else {
        cursor.x_offset = 0.0;
        cursor.column = 0;
      }
    }
    VirtualKeyCode::Left => {
      if cursor.column != 0 {
        cursor.column -= 1;
        cursor.x_offset =
          cursor_x_position2(cursor.row, cursor.column).unwrap();
      } else if cursor.row != 0 {
        cursor.row -= 1;
        cursor.column = text[cursor.row].len();
        cursor.x_offset =
          cursor_x_position2(cursor.row, cursor.column).unwrap_or(0.0);
      }
    }
    VirtualKeyCode::Down => {
      // TODO: handle last line
      if cursor.row != text.len() {
        cursor.row += 1;
        if let Some(offset) = cursor_x_position2(cursor.row, cursor.column) {
          cursor.x_offset = offset;
        } else {
          cursor.column = text[cursor.row].len();
          cursor.x_offset =
            cursor_x_position2(cursor.row, cursor.column).unwrap_or(0.0);
        }
      } else {
        cursor.column = text[cursor.row].len();
        cursor.x_offset =
          cursor_x_position2(cursor.row, cursor.column).unwrap();
      }
    }
    VirtualKeyCode::Right => {
      cursor.column += 1;
      if let Some(offset) = cursor_x_position2(cursor.row, cursor.column) {
        cursor.x_offset = offset;
      } else {
        cursor.x_offset = 0.0;
        cursor.column = 0;
        cursor.row += 1;
      }
    }
    _ => return,
  }

  cursor.rect.resize(
    size,
    PhysicalPosition {
      x: offset.x + scroll_offset.x + cursor.x_offset,
      y: size.height as f32
        - scroll_offset.y
        - font_height
        - (cursor.row as f32 * font_height),
    },
    cursor.rect.size,
  );
}

#[allow(clippy::too_many_arguments)]
pub fn input_char(
  size: PhysicalSize<u32>,
  ch: char,
  text: &mut Vec<String>,
  cursor: &mut Cursor,
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
  scroll_offset: PhysicalPosition<f32>,
) -> f32 {
  // TODO: deduplicate long input_special calls
  match ch {
    // backspace
    '\u{7f}' => {
      if cursor.column != 0 {
        let mut graphemes_indices = text[cursor.row].grapheme_indices(true);
        let index = graphemes_indices.nth(cursor.column - 1).unwrap().0;
        text[cursor.row].remove(index);
        input_special(
          size,
          VirtualKeyCode::Left,
          text,
          cursor,
          font.clone(),
          font_height,
          offset,
          scroll_offset,
        );
      } else if cursor.row != 0 {
        let removed = text.remove(cursor.row);
        let old = text[cursor.row - 1].clone();
        let first_char = removed.chars().nth(0);

        if let Some(ch) = first_char {
          text[cursor.row - 1] += &ch.to_string();
          input_special(
            size,
            VirtualKeyCode::Left,
            text,
            cursor,
            font.clone(),
            font_height,
            offset,
            scroll_offset,
          );
        }

        input_special(
          size,
          VirtualKeyCode::Left,
          text,
          cursor,
          font.clone(),
          font_height,
          offset,
          scroll_offset,
        );

        text[cursor.row] = old + &removed;
      }
    }
    // enter
    '\r' => {
      let mut graphemes_indices = text[cursor.row].grapheme_indices(true);
      let index = graphemes_indices.nth(cursor.column).unwrap().0;
      let after_enter = text[cursor.row].split_off(index);
      text.insert(cursor.row + 1, after_enter);
      input_special(
        size,
        VirtualKeyCode::Right,
        text,
        cursor,
        font.clone(),
        font_height,
        offset,
        scroll_offset,
      );
    }
    _ => {
      let mut graphemes_indices = text[cursor.row].grapheme_indices(true);
      let index = graphemes_indices.nth(cursor.column).unwrap().0;
      text[cursor.row].insert(index, ch);
      input_special(
        size,
        VirtualKeyCode::Right,
        text,
        cursor,
        font.clone(),
        font_height,
        offset,
        scroll_offset,
      );
    }
  }

  max_line_length(&text, font, font_height)
}

impl TextInput for TextArea {
  fn input_special(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    input_special(
      size,
      key,
      &mut self.text,
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalPosition { x: 0.0, y: 0.0 },
    );
  }

  fn input_char(&mut self, size: PhysicalSize<u32>, ch: char) {
    self.max_line_length = input_char(
      size,
      ch,
      &mut self.text,
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalPosition { x: 0.0, y: 0.0 },
    );
  }
}

impl super::RenderElement for TextArea {
  fn resize(&mut self, _size: PhysicalSize<u32>) {
    unimplemented!() // TODO
  }

  fn scroll(
    &mut self,
    _offset: PhysicalPosition<f64>,
    _size: PhysicalSize<u32>,
  ) {
    unimplemented!() // TODO
  }

  fn redraw(
    &mut self,
    _glyph_brush: &mut GlyphBrush<()>,
    _device: &Device,
    _staging_belt: &mut StagingBelt,
    _encoder: &mut CommandEncoder,
    _target: &TextureView,
    _size: PhysicalSize<u32>,
  ) {
    unimplemented!() // TODO
  }

  fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.cursor.rect]
  }
}
