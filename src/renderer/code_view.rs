use crate::renderer::rectangle::Rectangle;
use std::collections::HashMap;
use wgpu_glyph::ab_glyph::{Font, FontArc};
use wgpu_glyph::{
  GlyphPositioner, HorizontalAlign, Layout, Region, Section, SectionGeometry,
  Text,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

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
  font_width_map: HashMap<char, f32>,
  rect: Rectangle,
  cursor: Cursor,
  line_numbers_width: f32,
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

    let x = layout.calculate_glyphs(
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

    //println!("{} {:#?}", column, x.get(column));

    if let Some(g) = x.get(column) {
      Some(g.glyph.position.x)
    } else if column != 0 {
      if let Some(g) = x.get(column - 1) {
        Some(g.glyph.position.x + self.font.glyph_bounds(&g.glyph).width())
      } else {
        None
      }
    } else {
      None
    }
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
      font,
      text: split_text,
      scroll_offset: winit::dpi::PhysicalPosition { x: 0.0, y: 0.0 },
      font_height,
      font_width_map,
      rect,
      cursor: Cursor {
        rect: cursor,
        row: 0,
        column: 0,
        x_offset: 0.0,
      },
      line_numbers_width,
    }
  }

  pub fn input(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    let mut handle_left = || {
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
    };

    match key {
      VirtualKeyCode::Up => {
        if self.cursor.row != 0 {
          self.cursor.row -= 1;
          if let Some(x) =
            self.cursor_x_position(self.cursor.row, self.cursor.column)
          {
            self.cursor.x_offset = x;
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
      VirtualKeyCode::Left => handle_left(),
      VirtualKeyCode::Down => {
        // TODO: handle last line
        if self.cursor.row != self.text.len() {
          self.cursor.row += 1;
          if let Some(x) =
            self.cursor_x_position(self.cursor.row, self.cursor.column)
          {
            self.cursor.x_offset = x;
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
        if let Some(width) =
          self.cursor_x_position(self.cursor.row, self.cursor.column)
        {
          self.cursor.x_offset = width;
        } else {
          self.cursor.x_offset = 0.0;
          self.cursor.column = 0;
          self.cursor.row += 1;
        }
      }
      VirtualKeyCode::Back => {
        handle_left();

        self.text[self.cursor.row].remove(self.cursor.column);
      }
      _ => {}
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
