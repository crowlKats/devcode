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
  active: Option<usize>,
  tabs_container: Rectangle,
  dimensions: Dimensions,
}

impl CodeViewTabs {
  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<f32>,
    font: FontArc,
    font_height: f32,
    dimensions: Dimensions,
  ) -> Self {
    let rect = Rectangle::new(
      device,
      screen_size,
      Dimensions {
        height: TAB_HEIGHT,
        ..dimensions
      },
      [0.12, 0.2, 0.89],
      None,
    );

    Self {
      font,
      font_height,
      active: None,
      code_views: vec![],
      tabs_container: rect,
      dimensions,
    }
  }

  pub fn add(
    &mut self,
    device: &wgpu::Device,
    screen_size: PhysicalSize<f32>,
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
        width: TAB_PADDING + name_width + TAB_PADDING,
        ..self.tabs_container.dimensions
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
        y: self.dimensions.y + TAB_HEIGHT,
        height: self.dimensions.height - TAB_HEIGHT,
        ..self.dimensions
      },
      text,
    );

    self
      .code_views
      .push((filename.to_string(), rect, code_view));
    self.active = Some(self.code_views.len() - 1);
    Ok(())
  }

  fn get_active(&mut self) -> Option<&mut CodeView> {
    if let Some(i) = self.active {
      Some(&mut self.code_views[i].2)
    } else {
      None
    }
  }
}

impl super::RenderElement for CodeViewTabs {
  fn resize(&mut self, screen_size: PhysicalSize<f32>) {
    self.tabs_container.resize(
      screen_size.cast(),
      Dimensions {
        width: screen_size.width,
        ..self.tabs_container.dimensions
      },
    );
  }

  fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    screen_size: PhysicalSize<f32>,
  ) {
    if let Some(active) = self.get_active() {
      active.scroll(offset, screen_size);
    }
  }

  fn click(
    &mut self,
    position: PhysicalPosition<f64>,
    screen_size: PhysicalSize<f32>,
  ) {
    if let Some(pos) = self.tabs_container.dimensions.contains(position.cast())
    {
      for (i, (_, rect, _)) in self.code_views.iter().enumerate() {
        if rect.dimensions.contains(pos).is_some() {
          self.active = Some(i);
          break;
        }
      }
    } else if let Some(active) = self.get_active() {
      active.click(position, screen_size);
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

    if let Some(active) = self.get_active() {
      active.redraw(glyph_brush, device, staging_belt, encoder, target, size);
    }
  }

  fn get_rects(&self) -> Vec<&Rectangle> {
    let mut vec = vec![&self.tabs_container];
    vec.extend(self.code_views.iter().map(|(_, rect, _)| rect));
    if let Some(i) = self.active {
      vec.extend(self.code_views[i].2.get_rects());
    }
    vec
  }

  fn get_elements(&mut self) -> Vec<&mut dyn super::RenderElement> {
    if let Some(active) = self.get_active() {
      vec![active]
    } else {
      vec![]
    }
  }

  fn get_dimensions(&self) -> Dimensions {
    self.dimensions
  }
}

impl super::input::TextInput for CodeViewTabs {
  fn input_special(
    &mut self,
    screen_size: PhysicalSize<f32>,
    key: VirtualKeyCode,
  ) {
    if let Some(active) = self.get_active() {
      active.input_special(screen_size, key);
    }
  }

  fn input_char(&mut self, screen_size: PhysicalSize<f32>, ch: char) {
    if let Some(active) = self.get_active() {
      active.input_char(screen_size, ch);
    }
  }
}
