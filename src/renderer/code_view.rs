use crate::renderer::rectangle::Rectangle;
use std::collections::HashMap;
use wgpu_glyph::{HorizontalAlign, Layout, Region, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

pub struct CodeView {
  text: Vec<String>,
  scroll_offset: winit::dpi::PhysicalPosition<f64>,
  font_height: f32,
  font_width_map: HashMap<char, f32>,
  pub rect: Rectangle,
  pub cursor: Rectangle,
  cursor_row: u32,
  cursor_column: u32,
  cursor_x_offset: f32,
  line_numbers_width: f32,
}

impl CodeView {
  pub fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.cursor, &self.rect]
  }

  pub fn new(
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
    );

    let mut cursor = Rectangle::new(
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
    );
    cursor.region = Some(Region {
      x: line_numbers_width as u32 + 20,
      y: 0,
      width: screen_size.width - (line_numbers_width as u32 + 20),
      height: screen_size.height,
    });

    Self {
      text: split_text,
      scroll_offset: winit::dpi::PhysicalPosition { x: 0.0, y: 0.0 },
      font_height,
      font_width_map,
      rect,
      cursor,
      cursor_row: 0,
      cursor_column: 0,
      cursor_x_offset: 0.0,
      line_numbers_width,
    }
  }

  fn get_char(&self, row: u32, column: u32) -> Option<char> {
    self.text[row as usize].chars().nth(column as usize)
  }

  fn get_char_width(&self, row: u32, column: u32) -> Option<f32> {
    self
      .get_char(row, column)
      .map(|c| *self.font_width_map.get(&c).unwrap())
  }

  pub fn input(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    match key {
      VirtualKeyCode::Up => {
        if self.cursor_row != 0 {
          self.cursor_row -= 1;
          self.cursor_x_offset = 0.0;
          if self.get_char(self.cursor_row, self.cursor_column).is_some() {
            for i in 0..self.cursor_column {
              self.cursor_x_offset +=
                self.get_char_width(self.cursor_row, i).unwrap();
            }
          } else {
            let mut count = 0;
            for (i, _) in
              self.text[self.cursor_row as usize].chars().enumerate()
            {
              count += 1;
              self.cursor_x_offset +=
                self.get_char_width(self.cursor_row, i as u32).unwrap();
            }
            self.cursor_column = count;
          }
        } else {
          self.cursor_x_offset = 0.0;
          self.cursor_column = 0;
        }
      }
      VirtualKeyCode::Left => {
        if self.cursor_column != 0 {
          self.cursor_column -= 1;
          self.cursor_x_offset -= self
            .get_char_width(self.cursor_row, self.cursor_column)
            .unwrap();
        } else if self.cursor_row != 0 {
          self.cursor_row -= 1;
          self.cursor_x_offset = 0.0;
          let mut count = 0;
          for (i, _) in self.text[self.cursor_row as usize].chars().enumerate()
          {
            count += 1;
            self.cursor_x_offset +=
              self.get_char_width(self.cursor_row, i as u32).unwrap();
          }
          self.cursor_column = count;
        }
      }
      VirtualKeyCode::Down => {
        if self.cursor_row != self.text.len() as u32 {
          self.cursor_row += 1;
          self.cursor_x_offset = 0.0;
          if self.get_char(self.cursor_row, self.cursor_column).is_some() {
            for i in 0..self.cursor_column {
              self.cursor_x_offset +=
                self.get_char_width(self.cursor_row, i).unwrap();
            }
          } else {
            let mut count = 0;
            for (i, _) in
              self.text[self.cursor_row as usize].chars().enumerate()
            {
              count += 1;
              self.cursor_x_offset +=
                self.get_char_width(self.cursor_row, i as u32).unwrap();
            }
            self.cursor_column = count;
          }
        } else {
          self.cursor_x_offset = 0.0;
          let mut count = 0;
          for (i, _) in self.text[self.cursor_row as usize].chars().enumerate()
          {
            count += 1;
            self.cursor_x_offset +=
              self.get_char_width(self.cursor_row, i as u32).unwrap();
          }
          self.cursor_column = count;
        }
      }
      VirtualKeyCode::Right => {
        if let Some(width) =
          self.get_char_width(self.cursor_row, self.cursor_column)
        {
          self.cursor_x_offset += width;
          self.cursor_column += 1;
        } else {
          self.cursor_x_offset = 0.0;
          self.cursor_column = 0;
          self.cursor_row += 1;
        }
      }
      _ => {}
    }

    self.cursor.resize(
      size,
      PhysicalPosition {
        x: self.scroll_offset.x as f32
          + self.line_numbers_width
          + 20.0
          + self.cursor_x_offset,
        y: size.height as f32
          - self.scroll_offset.y as f32
          - self.font_height
          - (self.cursor_row as f32 * self.font_height),
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
        x: self.cursor.position.x,
        y: screen_size.height as f32
          - self.font_height
          - (self.cursor_row as f32 * self.font_height),
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
    let mut max_line_width = 0.0;
    for line in &self.text {
      line_count += 1;
      let line_width = line
        .chars()
        .fold(0.0, |acc, c| acc + self.font_width_map.get(&c).unwrap());
      if line_width > max_line_width {
        max_line_width = line_width;
      }
    }

    self.scroll_offset.x = (self.scroll_offset.x - offset.x)
      .max(
        (self.line_numbers_width as f64 + 20.0)
          + (size.width as f64 - max_line_width as f64),
      )
      .min(0.0);
    self.scroll_offset.y = (self.scroll_offset.y + offset.y)
      .min(0.0)
      .max(-((line_count - 3) as f32 * self.font_height) as f64);

    self.cursor.resize(
      size,
      PhysicalPosition {
        x: self.scroll_offset.x as f32
          + self.line_numbers_width
          + 20.0
          + self.cursor_x_offset,
        y: size.height as f32
          - self.font_height
          - self.scroll_offset.y as f32
          - (self.cursor_row as f32 * self.font_height),
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
      text: vec![Text::new(&self.text.join("\n"))
        .with_color([0.9, 0.9, 0.9, 1.0])
        .with_scale(self.font_height)],
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
