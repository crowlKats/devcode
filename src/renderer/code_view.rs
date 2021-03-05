use crate::renderer::rectangle::{Rectangle, Region};
use std::collections::HashMap;
use wgpu_glyph::ab_glyph::{Font, FontArc};
use wgpu_glyph::{
  GlyphPositioner, HorizontalAlign, Layout, Section, SectionGeometry, Text,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

#[derive(Debug)]
struct Cursor {
  rect: Rectangle,
  row: usize,
  column: usize,
  x_offset: f32,
}

pub struct CodeView {
  font: FontArc,
  text: Vec<String>,
  scroll_offset: winit::dpi::PhysicalPosition<f64>,
  font_height: f32,
  rect: Rectangle,
  cursor: Cursor,
  line_numbers_width: f32,
  max_line_length: f32,
}

impl CodeView {
  pub fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.cursor.rect, &self.rect]
  }

  fn generate_glyph_text(&self) -> Vec<Text> {
    self
      .text
      .iter()
      .map(|s| {
        Text::new(s)
          .with_color([0.9, 0.9, 0.9, 1.0])
          .with_scale(self.font_height)
      })
      .collect()
  }

  fn cursor_x_position(&self, row: usize, column: usize) -> Option<f32> {
    let text = Text::new(&self.text[row]).with_scale(self.font_height);
    let layout = Layout::default_wrap();

    let section_glyphs = layout.calculate_glyphs(
      &[self.font.clone()],
      &SectionGeometry {
        screen_position: (
          self.scroll_offset.x as f32,
          self.scroll_offset.y as f32,
        ),
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
            + self.font.glyph_bounds(&section_glyph.glyph).width(),
        )
      } else {
        None
      }
    } else {
      None
    }
  }

  fn max_line_length(lines: &[String], font: FontArc, font_height: f32) -> f32 {
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

  pub fn new(
    font: FontArc,
    text: String,
    font_height: f32,
    font_width_map: HashMap<char, f32>,
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
  ) -> Self {
    let mut split_text =
      text.lines().map(|s| s.to_string()).collect::<Vec<String>>();
    if text.ends_with('\n') {
      split_text.push(String::from(""));
    }

    // TODO: use Layout::calculate_glyphs
    let line_numbers_width = {
      let mut max_line_width = 0.0;
      for (i, _) in split_text.iter().enumerate() {
        let line_width = i
          .to_string()
          .chars()
          .fold(0.0, |acc, c| acc + font_width_map.get(&c).unwrap());
        if line_width > max_line_width {
          max_line_width = line_width;
        }
      }
      max_line_width
    };

    let rect = Rectangle::new(
      device,
      screen_size,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalSize {
        width: line_numbers_width as u32 + 10,
        height: screen_size.height,
      },
      [0.05, 0.05, 0.05],
      None,
    );

    let cursor = Rectangle::new(
      device,
      screen_size,
      PhysicalPosition {
        x: line_numbers_width + 20.0,
        y: screen_size.height as f32 - font_height,
      },
      PhysicalSize {
        width: 6,
        height: font_height as u32,
      },
      [0.7, 0.0, 0.0],
      Some(Region {
        x: line_numbers_width as u32 + 20,
        y: 0,
        width: screen_size.width - (line_numbers_width as u32 + 20),
        height: screen_size.height,
      }),
    );

    let max_line_length =
      Self::max_line_length(&split_text, font.clone(), font_height);

    Self {
      font,
      text: split_text,
      scroll_offset: winit::dpi::PhysicalPosition { x: 0.0, y: 0.0 },
      font_height,
      rect,
      cursor: Cursor {
        rect: cursor,
        row: 0,
        column: 0,
        x_offset: 0.0,
      },
      line_numbers_width,
      max_line_length,
    }
  }

  pub fn input_special(
    &mut self,
    size: PhysicalSize<u32>,
    key: VirtualKeyCode,
  ) {
    match key {
      VirtualKeyCode::Up => {
        if self.cursor.row != 0 {
          self.cursor.row -= 1;
          if let Some(offset) =
            self.cursor_x_position(self.cursor.row, self.cursor.column)
          {
            self.cursor.x_offset = offset;
          } else {
            self.cursor.column = self.text[self.cursor.row].len();
            self.cursor.x_offset = self
              .cursor_x_position(self.cursor.row, self.cursor.column)
              .unwrap_or(0.0);
          }
        } else {
          self.cursor.x_offset = 0.0;
          self.cursor.column = 0;
        }
      }
      VirtualKeyCode::Left => {
        if self.cursor.column != 0 {
          self.cursor.column -= 1;
          self.cursor.x_offset = self
            .cursor_x_position(self.cursor.row, self.cursor.column)
            .unwrap();
        } else if self.cursor.row != 0 {
          self.cursor.row -= 1;
          self.cursor.column = self.text[self.cursor.row].len();
          self.cursor.x_offset = self
            .cursor_x_position(self.cursor.row, self.cursor.column)
            .unwrap_or(0.0);
        }
      }
      VirtualKeyCode::Down => {
        // TODO: handle last line
        if self.cursor.row != self.text.len() {
          self.cursor.row += 1;
          if let Some(offset) =
            self.cursor_x_position(self.cursor.row, self.cursor.column)
          {
            self.cursor.x_offset = offset;
          } else {
            self.cursor.column = self.text[self.cursor.row].len();
            self.cursor.x_offset = self
              .cursor_x_position(self.cursor.row, self.cursor.column)
              .unwrap_or(0.0);
          }
        } else {
          self.cursor.column = self.text[self.cursor.row].len();
          self.cursor.x_offset = self
            .cursor_x_position(self.cursor.row, self.cursor.column)
            .unwrap();
        }
      }
      VirtualKeyCode::Right => {
        self.cursor.column += 1;
        if let Some(offset) =
          self.cursor_x_position(self.cursor.row, self.cursor.column)
        {
          self.cursor.x_offset = offset;
        } else {
          self.cursor.x_offset = 0.0;
          self.cursor.column = 0;
          self.cursor.row += 1;
        }
      }
      _ => return,
    }

    self.cursor.rect.resize(
      size,
      PhysicalPosition {
        x: self.scroll_offset.x as f32
          + self.line_numbers_width
          + 20.0
          + self.cursor.x_offset,
        y: size.height as f32
          - self.scroll_offset.y as f32
          - self.font_height
          - (self.cursor.row as f32 * self.font_height),
      },
      self.cursor.rect.size,
    );
  }

  pub fn input_char(&mut self, size: PhysicalSize<u32>, ch: char) {
    match ch {
      '\u{7f}' => {
        self.input_special(size, VirtualKeyCode::Left);
        if self.cursor.column != 0 {
          self.text[self.cursor.row].remove(self.cursor.column);
        } else if self.cursor.row != 0 {
          let removed = self.text.remove(self.cursor.row);
          self.text[self.cursor.row - 1] += &removed;
        }
      }
      '\r' => {
        let after_enter =
          self.text[self.cursor.row].split_off(self.cursor.column);
        self.text.insert(self.cursor.row + 1, after_enter);
        self.input_special(size, VirtualKeyCode::Right);
      }
      _ => {
        println!(
          "{} {} {} ({})",
          self.text[self.cursor.row].len(),
          self.cursor.column,
          ch,
          ch.escape_unicode(),
        );
        assert!(self.text[self.cursor.row].is_char_boundary(self.cursor.column));
        println!("foo");
        self.text[self.cursor.row].insert(self.cursor.column, ch);
        self.input_special(size, VirtualKeyCode::Right);
      }
    }
    self.max_line_length =
      Self::max_line_length(&self.text, self.font.clone(), self.font_height);
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

    self.cursor.rect.resize(
      screen_size,
      PhysicalPosition {
        x: self.cursor.rect.position.x,
        y: screen_size.height as f32
          - self.font_height
          - (self.cursor.row as f32 * self.font_height),
      },
      self.cursor.rect.size,
    );

    self.cursor.rect.region = Some(Region {
      x: self.line_numbers_width as u32 + 20,
      y: 0,
      width: screen_size.width - (self.line_numbers_width as u32 + 20),
      height: screen_size.height,
    });
  }

  fn scroll(&mut self, offset: PhysicalPosition<f64>, size: PhysicalSize<u32>) {
    self.scroll_offset.x = (self.scroll_offset.x - offset.x)
      .max(
        (size.width as f64 - (self.line_numbers_width as f64 + 20.0))
          - self.max_line_length as f64,
      )
      .min(0.0);
    self.scroll_offset.y = (self.scroll_offset.y + offset.y)
      .min(0.0)
      .max(-((self.text.len() - 3) as f32 * self.font_height) as f64);

    self.cursor.rect.resize(
      size,
      PhysicalPosition {
        x: self.scroll_offset.x as f32
          + self.line_numbers_width
          + 20.0
          + self.cursor.x_offset,
        y: size.height as f32
          - self.font_height
          - self.scroll_offset.y as f32
          - (self.cursor.row as f32 * self.font_height),
      },
      self.cursor.rect.size,
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
    for _ in &self.text {
      line_count += 1;
      line_numbers += &format!("{}\n", line_count);
    }

    glyph_brush.queue(Section {
      screen_position: (self.line_numbers_width, self.scroll_offset.y as f32),
      text: vec![Text::new(&line_numbers)
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_height)],
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
      text: self
        .generate_glyph_text()
        .iter()
        .flat_map(|s| {
          std::iter::once(*s).chain(std::iter::once(
            Text::new("\n").with_scale(self.font_height),
          ))
        })
        .collect(),
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
