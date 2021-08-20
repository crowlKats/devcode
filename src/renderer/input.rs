use crate::renderer::rectangle::{Rectangle, Region};
use crate::renderer::Dimensions;
use wgpu_glyph::ab_glyph::{Font, FontArc};
use wgpu_glyph::{GlyphPositioner, Layout, SectionGeometry, Text};
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
    screen_size: PhysicalSize<f32>,
    dimensions: Dimensions,
    color: [f32; 3],
    region: Option<Region>,
  ) -> Self {
    Self {
      rect: Rectangle::new(device, screen_size, dimensions, color, region),
      row: 0,
      column: 0,
      x_offset: 0.0,
    }
  }
}

pub trait TextInput {
  fn input_special(
    &mut self,
    screen_size: PhysicalSize<f32>,
    key: VirtualKeyCode,
  );
  fn input_char(&mut self, screen_size: PhysicalSize<f32>, ch: char);
}

// TODO: implement TextArea

pub fn line_length(line: &str, font: FontArc, font_height: f32) -> f32 {
  let layout = Layout::default_wrap();
  let text = Text::new(line).with_scale(font_height);
  let section_glyphs = layout.calculate_glyphs(
    &[font.clone()],
    &SectionGeometry {
      ..Default::default()
    },
    &[text],
  );

  if let Some(section_glyph) = section_glyphs.last() {
    section_glyph.glyph.position.x
      + font.glyph_bounds(&section_glyph.glyph).width()
  } else {
    0.0
  }
}

pub fn max_line_length(
  lines: impl Iterator<Item = String>,
  font: FontArc,
  font_height: f32,
) -> f32 {
  let mut max_line_width = 0.0;
  for line in lines {
    let width = line_length(&line, font.clone(), font_height);

    if width > max_line_width {
      max_line_width = width;
    }
  }

  max_line_width
}

pub fn cursor_x_position(
  row: usize,
  column: usize,
  text: &ropey::Rope,
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
) -> Option<f32> {
  let line = text.line(row).to_string();
  let text = Text::new(&line).with_scale(font_height);
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
    section_glyphs.get(column - 1).map(|section_glyph| {
      section_glyph.glyph.position.x
        + font.glyph_bounds(&section_glyph.glyph).width()
    })
  } else {
    None
  }
}

#[allow(clippy::too_many_arguments)]
pub fn input_special(
  screen_size: PhysicalSize<f32>,
  key: VirtualKeyCode,
  rope: &mut ropey::Rope,
  cursor: &mut Cursor,
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
  scroll_offset: PhysicalPosition<f32>,
) {
  let cursor_x_pos = |row: usize, column: usize| {
    cursor_x_position(
      row,
      column,
      rope,
      font.clone(),
      font_height,
      scroll_offset,
    )
  };

  match key {
    VirtualKeyCode::Up => {
      if cursor.row != 0 {
        cursor.row -= 1;
        if let Some(offset) = cursor_x_pos(cursor.row, cursor.column) {
          cursor.x_offset = offset;
        } else {
          cursor.column = rope.line(cursor.row).len_chars();
          cursor.x_offset =
            cursor_x_pos(cursor.row, cursor.column).unwrap_or_default();
        }
      } else {
        cursor.x_offset = 0.0;
        cursor.column = 0;
      }
    }
    VirtualKeyCode::Left => {
      match (cursor.row, cursor.column) {
        (0, 0) => {}
        (_, 0) => {
          // TODO: https://github.com/cessen/ropey/issues/44
          cursor.row -= 1;
          cursor.column = rope.line(cursor.row).len_chars() - 1;
          cursor.x_offset =
            cursor_x_pos(cursor.row, cursor.column).unwrap_or_default();
        }
        (_, _) => {
          cursor.column -= 1;
          cursor.x_offset = cursor_x_pos(cursor.row, cursor.column).unwrap();
        }
      }
    }
    VirtualKeyCode::Down => {
      if cursor.row != (rope.len_lines() - 1) {
        cursor.row += 1;
        if let Some(offset) = cursor_x_pos(cursor.row, cursor.column) {
          cursor.x_offset = offset;
        } else {
          cursor.column = rope.line(cursor.row).len_chars();
          cursor.x_offset =
            cursor_x_pos(cursor.row, cursor.column).unwrap_or_default();
        }
      } else {
        cursor.column = rope.line(cursor.row).len_chars();
        cursor.x_offset =
          cursor_x_pos(cursor.row, cursor.column).unwrap_or_default();
      }
    }
    VirtualKeyCode::Right => {
      if cursor.row != (rope.len_lines() - 1) {
        if let Some(offset) = cursor_x_pos(cursor.row, cursor.column + 1) {
          cursor.column += 1;
          cursor.x_offset = offset;
        } else {
          cursor.x_offset = 0.0;
          cursor.column = 0;
          cursor.row += 1;
        }
      } else if let Some(offset) = cursor_x_pos(cursor.row, cursor.column + 1) {
        cursor.column += 1;
        cursor.x_offset = offset;
      }
    }
    _ => return,
  }

  cursor.rect.resize(
    screen_size,
    Dimensions {
      x: offset.x + scroll_offset.x + cursor.x_offset,
      y: scroll_offset.y + font_height + (cursor.row as f32 * font_height),
      ..cursor.rect.dimensions
    },
  );
}

#[allow(clippy::too_many_arguments)]
pub fn input_char(
  screen_size: PhysicalSize<f32>,
  ch: char,
  rope: &mut ropey::Rope,
  cursor: &mut Cursor,
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
  scroll_offset: PhysicalPosition<f32>,
) -> f32 {
  let input_spc =
    |key: VirtualKeyCode, text: &mut ropey::Rope, cursor: &mut Cursor| {
      input_special(
        screen_size,
        key,
        text,
        cursor,
        font.clone(),
        font_height,
        offset,
        scroll_offset,
      );
    };

  match ch {
    // backspace
    '\u{7f}' => match (cursor.row, cursor.column) {
      (0, 0) => {}
      (row, 0) => {
        // TODO: https://github.com/cessen/ropey/issues/44
        let ln = rope.line_to_char(row);
        input_spc(VirtualKeyCode::Left, rope, cursor);
        rope.remove((ln - 1)..ln);
      }
      (row, column) => {
        let index = rope.line_to_char(row) + column;
        rope.remove((index - 1)..index);
        input_spc(VirtualKeyCode::Left, rope, cursor);
      }
    },
    // enter
    '\r' => {
      rope.insert_char(rope.line_to_char(cursor.row) + cursor.column, '\n');
      input_spc(VirtualKeyCode::Right, rope, cursor);
    }
    // esc
    '\u{1b}' => {
      println!("{:?}", (cursor.row, cursor.column));
    }
    _ => {
      rope.insert_char(rope.line_to_char(cursor.row) + cursor.column, ch);
      input_spc(VirtualKeyCode::Right, rope, cursor);
    }
  }

  max_line_length(rope.lines().map(|l| l.to_string()), font, font_height)
}
