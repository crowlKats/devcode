use super::input::{max_line_length, Cursor};
use super::rectangle::{Rectangle, Region};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{
  GlyphPositioner, HorizontalAlign, Layout, Section, SectionGeometry, Text,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

pub struct CodeView {
  font: FontArc,
  font_height: f32,
  text: Vec<String>,
  scroll_offset: PhysicalPosition<f64>,
  rect: Rectangle,
  cursor: Cursor,
  line_numbers_width: f32,
  line_numbers_width_padded: f32,
  max_line_length: f32,
  pub position: PhysicalPosition<u32>,
  pub size: PhysicalSize<u32>,
}

impl CodeView {
  fn generate_glyph_text(&self, range: std::ops::Range<usize>) -> Vec<Text> {
    self.text[range]
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
    position: PhysicalPosition<u32>,
    size: PhysicalSize<u32>,
    text: String,
  ) -> Self {
    let mut split_text =
      text.lines().map(|s| s.to_string()).collect::<Vec<String>>();
    if text.ends_with('\n') {
      split_text.push(String::from(""));
    }

    let line_numbers = split_text
      .iter()
      .enumerate()
      .map(|(i, _)| i.to_string())
      .collect::<Vec<String>>();
    let line_numbers_width =
      max_line_length(&line_numbers, font.clone(), font_height);

    let line_numbers_width_padded = line_numbers_width + 20.0;

    let rect = Rectangle::new(
      device,
      screen_size,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalSize {
        width: position.x + (line_numbers_width_padded as u32 - 10),
        height: screen_size.height,
      },
      [0.05, 0.05, 0.05],
      None,
    );

    let cursor = Cursor::new(
      device,
      screen_size,
      PhysicalPosition {
        x: position.x as f32 + line_numbers_width_padded,
        y: screen_size.height as f32 - font_height,
      },
      PhysicalSize {
        width: 4,
        height: font_height as u32,
      },
      [0.7, 0.0, 0.0],
      Some(Region {
        x: line_numbers_width_padded as u32,
        y: 0,
        width: screen_size.width - (line_numbers_width_padded as u32),
        height: screen_size.height,
      }),
    );

    let max_line_length =
      max_line_length(&split_text, font.clone(), font_height);

    Self {
      font,
      font_height,
      text: split_text,
      scroll_offset: PhysicalPosition { x: 0.0, y: 0.0 },
      rect,
      cursor,
      line_numbers_width,
      line_numbers_width_padded,
      max_line_length,
      position,
      size,
    }
  }
}

impl super::input::TextInput for CodeView {
  fn input_special(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    super::input::input_special(
      size,
      key,
      &mut self.text,
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition {
        x: self.position.x as f32 + self.line_numbers_width_padded,
        y: 0.0,
      },
      self.scroll_offset.cast(),
    );
  }

  fn input_char(&mut self, size: PhysicalSize<u32>, ch: char) {
    self.max_line_length = super::input::input_char(
      size,
      ch,
      &mut self.text,
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition {
        x: self.position.x as f32 + self.line_numbers_width_padded,
        y: 0.0,
      },
      self.scroll_offset.cast(),
    );
  }
}

impl super::RenderElement for CodeView {
  fn resize(&mut self, screen_size: PhysicalSize<u32>) {
    self.rect.resize(
      screen_size,
      PhysicalPosition {
        x: self.position.x as f32,
        y: 0.0,
      },
      PhysicalSize {
        width: self.line_numbers_width_padded as u32 - 10,
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
      x: self.position.x + self.line_numbers_width_padded as u32,
      y: 0,
      width: screen_size.width
        - (self.position.x + self.line_numbers_width_padded as u32),
      height: screen_size.height,
    });

    self.size = screen_size;
  }

  fn scroll(&mut self, offset: PhysicalPosition<f64>, size: PhysicalSize<u32>) {
    if offset.x.abs() > offset.y.abs() {
      self.scroll_offset.x = (self.scroll_offset.x - offset.x)
        .max(
          (size.width as f64 - self.line_numbers_width_padded as f64)
            - self.max_line_length as f64,
        ) // TODO
        .min(0.0);
    } else {
      self.scroll_offset.y = (self.scroll_offset.y + offset.y)
        .min(0.0)
        .max(-((self.text.len() - 3) as f32 * self.font_height) as f64);
    }

    self.cursor.rect.resize(
      size,
      PhysicalPosition {
        x: self.position.x as f32
          + self.scroll_offset.x as f32
          + self.line_numbers_width_padded
          + self.cursor.x_offset,
        y: size.height as f32
          - self.font_height
          - self.scroll_offset.y as f32
          - (self.cursor.row as f32 * self.font_height),
      },
      self.cursor.rect.size,
    );
  }

  fn click(&mut self, position: PhysicalPosition<f64>) {
    let line = ((position.y - self.scroll_offset.y) / self.font_height as f64)
      .floor() as usize;
    let text = self.generate_glyph_text(line..line + 1)[0];
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
      if (position.x as f32 - self.line_numbers_width_padded)
        < section_glyph.glyph.position.x
      {
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
      + ((self.size.height as f64) as f32 / self.font_height).ceil() as usize)
      .min(self.text.len());

    let mut line_count = upper_bound;
    let mut line_numbers = String::new();
    for _ in &self.text[upper_bound..lower_bound] {
      line_count += 1;
      line_numbers += &format!("{}\n", line_count);
    }

    let y_scroll = -((-self.scroll_offset.y as f32) % self.font_height);

    glyph_brush.queue(Section {
      screen_position: (
        self.position.x as f32 + self.line_numbers_width,
        y_scroll,
      ),
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

    let codeview_offset =
      self.position.x as f32 + self.line_numbers_width_padded;
    glyph_brush.queue(Section {
      screen_position: (
        codeview_offset + self.scroll_offset.x as f32,
        y_scroll,
      ),
      text: self.generate_glyph_text(upper_bound..lower_bound),
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

  fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.cursor.rect, &self.rect]
  }

  fn get_pos_size(&self) -> (PhysicalPosition<u32>, PhysicalSize<u32>) {
    (self.position, self.size)
  }
}
