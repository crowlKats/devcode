use crate::renderer::code_view::CodeView;
use crate::renderer::input::line_length;
use crate::renderer::rectangle::Rectangle;
use crate::renderer::Dimensions;
use std::path::PathBuf;
use wgpu::util::StagingBelt;
use wgpu::{CommandEncoder, TextureView};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrush, HorizontalAlign, Layout, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

const TAB_HEIGHT: f32 = 50.0;
const TAB_PADDING: f32 = 15.0;

pub struct CodeViewTabs {
  font: FontArc,
  font_height: f32,
  pub code_views: Vec<(String, Rectangle, CodeView)>,
  active: usize,
  rect: Rectangle,
  dimensions: Dimensions,
}

impl CodeViewTabs {
  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    font: FontArc,
    font_height: f32,
    dimensions: Dimensions,
  ) -> Self {
    let rect = Rectangle::new(
      device,
      screen_size,
      Dimensions {
        height: TAB_HEIGHT,
        y: dimensions.height - TAB_HEIGHT,
        ..dimensions
      },
      [0.12, 0.2, 0.89],
      None,
    );

    Self {
      font,
      font_height,
      active: 0,
      code_views: vec![],
      rect,
      dimensions,
    }
  }

  pub fn add(
    &mut self,
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    filepath: PathBuf,
  ) -> Result<(), anyhow::Error> {
    if !filepath.exists() {
      anyhow::bail!("path doesn't exist");
    }
    if !filepath.is_file() {
      anyhow::bail!("path isn't a file");
    }
    let text = std::fs::read_to_string(&filepath)?;

    let filename = filepath.file_name().unwrap().to_str().unwrap();
    let name_width = line_length(filename, self.font.clone(), self.font_height);

    let rect = Rectangle::new(
      device,
      screen_size,
      Dimensions {
        height: TAB_HEIGHT,
        y: self.dimensions.height - TAB_HEIGHT,
        width: TAB_PADDING + name_width + TAB_PADDING,
        ..self.dimensions
      },
      [0.04, 0.12, 0.81],
      None,
    );

    let code_view = CodeView::new(
      &device,
      screen_size,
      self.font.clone(),
      self.font_height,
      Dimensions {
        y: self.dimensions.y,
        x: self.dimensions.x,
        width: self.dimensions.width,
        height: self.dimensions.height - TAB_HEIGHT,
      },
      text,
    );

    self
      .code_views
      .push((filename.to_string(), rect, code_view));
    Ok(())
  }

  fn get_active(&mut self) -> &mut CodeView {
    &mut self.code_views[self.active].2
  }
}

impl super::RenderElement for CodeViewTabs {
  fn resize(&mut self, screen_size: PhysicalSize<f32>) {
    self.rect.resize(
      screen_size.cast(),
      Dimensions {
        width: screen_size.width,
        ..self.rect.dimensions
      },
    );
  }

  fn scroll(&mut self, offset: PhysicalPosition<f64>, size: PhysicalSize<f32>) {
    self.get_active().scroll(offset, size);
  }

  fn click(&mut self, position: PhysicalPosition<f64>) {
    if let Some(pos) = self.rect.dimensions.contains(position.cast()) {
      for (i, (_, rect, _)) in self.code_views.iter().enumerate() {
        if rect.dimensions.contains(pos).is_some() {
          self.active = i;
          break;
        }
      }
    } else {
      self.get_active().click(position);
    }
  }

  fn redraw(
    &mut self,
    glyph_brush: &mut GlyphBrush<()>,
    device: &wgpu::Device,
    staging_belt: &mut StagingBelt,
    encoder: &mut CommandEncoder,
    target: &TextureView,
    size: PhysicalSize<u32>,
  ) {
    for (name, rect, _) in &self.code_views {
      glyph_brush.queue(Section {
        screen_position: (
          (rect.dimensions.x + TAB_PADDING),
          (TAB_HEIGHT - self.font_height) / 2.0,
        ),
        text: vec![Text::new(&name)
          .with_color([0.9, 0.9, 0.9, 1.0])
          .with_scale(self.font_height)],
        layout: Layout::default_wrap().h_align(HorizontalAlign::Left),
        ..Section::default()
      });
    }

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

    self.get_active().redraw(
      glyph_brush,
      device,
      staging_belt,
      encoder,
      target,
      size,
    );
  }

  fn get_rects(&self) -> Vec<&Rectangle> {
    let mut vec = vec![&self.rect];
    vec.extend(self.code_views.iter().map(|(_, rect, _)| rect));
    vec.extend(self.code_views[self.active].2.get_rects());
    vec
  }

  fn get_elements(&mut self) -> Vec<&mut dyn super::RenderElement> {
    vec![self.get_active()]
  }

  fn get_dimensions(&self) -> Dimensions {
    self.dimensions
  }
}

impl super::input::TextInput for CodeViewTabs {
  fn input_special(&mut self, size: PhysicalSize<u32>, key: VirtualKeyCode) {
    self.get_active().input_special(size, key);
  }

  fn input_char(&mut self, size: PhysicalSize<u32>, ch: char) {
    self.get_active().input_char(size, ch);
  }
}
