use super::super::input::{max_line_length, Cursor};
use super::super::rectangle::Rectangle;
use crate::renderer::Dimensions;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::fmt::Formatter;
use std::rc::Rc;
use tree_sitter_highlight::{HighlightEvent, Highlighter};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphPositioner, Layout, Section, SectionGeometry, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

pub struct Code {
  font: FontArc,
  font_height: f32,
  text: Rc<RefCell<ropey::Rope>>,
  scroll_offset: PhysicalPosition<f64>,
  cursor: Cursor,
  max_line_length: f32,
  pub dimensions: Dimensions,
  highlight_config: tree_sitter_highlight::HighlightConfiguration,
  /// Vec of tuples of char_start, chard_end and HighlightName
  highlights: Vec<(usize, usize, Option<HighlightNames>)>,
}

#[derive(Copy, Clone, Debug, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum HighlightNames {
  Constant,
  ConstantBuiltin,
  Type,
  TypeBuiltin,
  Constructor,
  Function,
  FunctionMethod,
  FunctionMacro,
  Property,
  Comment,
  PunctuationBracket,
  PunctuationDelimiter,
  VariableParameter,
  VariableBuiltin,
  Label,
  Keyword,
  String,
  Escape,
  Attribute,
  Operator,
}

impl HighlightNames {
  const VARIANTS: [HighlightNames; 20] = [
    HighlightNames::Constant,
    HighlightNames::ConstantBuiltin,
    HighlightNames::Type,
    HighlightNames::TypeBuiltin,
    HighlightNames::Constructor,
    HighlightNames::Function,
    HighlightNames::FunctionMethod,
    HighlightNames::FunctionMacro,
    HighlightNames::Property,
    HighlightNames::Comment,
    HighlightNames::PunctuationBracket,
    HighlightNames::PunctuationDelimiter,
    HighlightNames::VariableParameter,
    HighlightNames::VariableBuiltin,
    HighlightNames::Label,
    HighlightNames::Keyword,
    HighlightNames::String,
    HighlightNames::Escape,
    HighlightNames::Attribute,
    HighlightNames::Operator,
  ];

  fn color(&self) -> [f32; 4] {
    #[allow(clippy::excessive_precision)]
    match self {
      HighlightNames::Constant => [0.59607843, 0.4627451, 0.66666667, 1.0],
      HighlightNames::ConstantBuiltin => {
        [0.65882353, 0.33333333, 0.44705882, 1.0]
      }
      HighlightNames::Type => [0.94117647, 0.77647059, 0.45490196, 1.0], //
      HighlightNames::TypeBuiltin => [0.8, 0.47058824, 0.19607843, 1.0], //
      HighlightNames::Constructor => [0.91372549, 0.74509804, 0.40784314, 1.0], // TODO
      HighlightNames::Function => [0.9, 0.9, 0.9, 1.0], // TODO: function usage and definition
      HighlightNames::FunctionMethod => {
        [0.91372549, 0.74509804, 0.40784314, 1.0] // TODO: methods
      }
      HighlightNames::FunctionMacro => {
        [0.30588235, 0.67843137, 0.89803922, 1.0] //
      }
      HighlightNames::Property => [0.59607843, 0.46666667, 0.66666667, 1.0], //
      HighlightNames::Comment => [0.47843137, 0.34509804, 0.5254902, 1.0],   //
      HighlightNames::PunctuationBracket => {
        [0.9, 0.9, 0.9, 1.0] // TODO
      }
      HighlightNames::PunctuationDelimiter => {
        [0.278431371, 0.60784314, 0.49411765, 1.0] // TODO
      }
      HighlightNames::VariableParameter => [0.8, 0.4, 0.4, 1.0], //
      HighlightNames::VariableBuiltin => [0.8, 0.47058824, 0.19607843, 1.0],
      HighlightNames::Label => [0.1254902, 0.6, 0.61568627, 1.0], //
      HighlightNames::Keyword => [0.8, 0.47058824, 0.19607843, 1.0], //
      HighlightNames::String => [0.50588235, 0.72941176, 0.34901961, 1.0], //
      HighlightNames::Escape => [0.52941176, 0.74117647, 0.77647059, 1.0], //
      HighlightNames::Attribute => [0.83111111, 0.70980392, 0.16078431, 1.0],
      HighlightNames::Operator => [0.278431371, 0.60784314, 0.49411765, 1.0], //
    }
  }
}

impl std::fmt::Display for HighlightNames {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      HighlightNames::Constant => "constant",
      HighlightNames::ConstantBuiltin => "constant.builtin",
      HighlightNames::Type => "type",
      HighlightNames::TypeBuiltin => "type.builtin",
      HighlightNames::Constructor => "constructor",
      HighlightNames::Function => "function",
      HighlightNames::FunctionMethod => "function.method",
      HighlightNames::FunctionMacro => "function.macro",
      HighlightNames::Property => "property",
      HighlightNames::Comment => "comment",
      HighlightNames::PunctuationBracket => "punctuation.bracket",
      HighlightNames::PunctuationDelimiter => "punctuation.delimiter",
      HighlightNames::VariableParameter => "variable.parameter",
      HighlightNames::VariableBuiltin => "variable.builtin",
      HighlightNames::Label => "label",
      HighlightNames::Keyword => "keyword",
      HighlightNames::String => "string",
      HighlightNames::Escape => "escape",
      HighlightNames::Attribute => "attribute",
      HighlightNames::Operator => "operator",
    })
  }
}

impl Code {
  fn generate_glyph_text<'r>(
    &self,
    text: &'r ropey::Rope,
    start_line: usize,
    end_line: usize,
  ) -> Vec<Text<'r>> {
    let start_char = text.line_to_char(start_line);
    let end_char = text.line_to_char(end_line);

    self
      .highlights
      .iter()
      .enumerate()
      .skip_while(|(_, (_, end, _))| end <= &start_char)
      .take_while(|(_, (_, end, _))| end <= &end_char)
      .flat_map(|(_, (start, end, name))| {
        text
          .slice(start.max(&start_char)..end.min(&end_char))
          .chunks()
          .map(move |c| {
            Text::new(c)
              .with_color(
                name.map(|n| n.color()).unwrap_or([0.9, 0.9, 0.9, 1.0]),
              )
              .with_scale(self.font_height)
          })
      })
      .collect()
  }

  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<f32>,
    font: FontArc,
    font_height: f32,
    dimensions: Dimensions,
    text: Rc<RefCell<ropey::Rope>>,
  ) -> Self {
    let cursor = Cursor::new(
      device,
      screen_size,
      Dimensions {
        width: 4.0,
        height: font_height,
        ..dimensions
      },
      [0.68, 0.28, 0.26],
      Some(dimensions.into()),
    );

    let max_line_length = max_line_length(
      text.borrow().lines().map(|s| s.to_string()),
      font.clone(),
      font_height,
    );

    // TODO: language specific handling
    let mut highlight_config =
      tree_sitter_highlight::HighlightConfiguration::new(
        tree_sitter_rust::language(),
        tree_sitter_rust::HIGHLIGHT_QUERY,
        "",
        "",
      )
      .unwrap();

    highlight_config.names();

    highlight_config.configure(
      &HighlightNames::VARIANTS
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>(),
    );

    Self {
      font,
      font_height,
      text,
      scroll_offset: PhysicalPosition { x: 0.0, y: 0.0 },
      cursor,
      max_line_length,
      dimensions,
      highlight_config,
      highlights: vec![],
    }
  }

  fn generate_highlighting(&mut self) {
    let mut highlighter = Highlighter::new();
    let source = self.text.borrow().bytes().collect::<Vec<u8>>();
    let highlights = highlighter
      .highlight(&self.highlight_config, &source, None, |_| None)
      .unwrap();

    self.highlights.clear();
    let mut current_range = (0, 0);
    let mut current_highlight = None;
    let rope = self.text.borrow();
    for event in highlights {
      match event.unwrap() {
        HighlightEvent::Source { start, end } => {
          let start = rope.byte_to_char(start);
          let end = rope.byte_to_char(end);
          if current_highlight.is_none() {
            self.highlights.push((start, end, None));
          } else {
            current_range = (start, end);
          }
        }
        HighlightEvent::HighlightStart(s) => {
          current_highlight = HighlightNames::try_from(s.0 as u8).ok();
        }
        HighlightEvent::HighlightEnd => {
          self.highlights.push((
            current_range.0,
            current_range.1,
            current_highlight,
          ));
          current_highlight = None;
        }
      }
    }
  }
}

impl super::super::input::TextInput for Code {
  fn input_special(
    &mut self,
    screen_size: PhysicalSize<f32>,
    key: VirtualKeyCode,
  ) {
    super::super::input::input_special(
      screen_size,
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
    self.generate_highlighting(); // TODO: remove, shouldnt generate highglights when moving cursor around
  }

  fn input_char(&mut self, screen_size: PhysicalSize<f32>, ch: char) {
    self.max_line_length = super::super::input::input_char(
      screen_size,
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
    self.generate_highlighting();
  }
}

impl super::super::RenderElement for Code {
  fn resize(&mut self, screen_size: PhysicalSize<f32>) {
    self.dimensions.width = screen_size.width - self.dimensions.x;

    self.cursor.rect.resize(
      screen_size.cast(),
      Dimensions {
        y: self.font_height - (self.cursor.row as f32 * self.font_height),
        ..self.cursor.rect.dimensions
      },
    );

    self.cursor.rect.region = Some(self.dimensions.into());
  }

  fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    screen_size: PhysicalSize<f32>,
  ) {
    if offset.x.abs() > offset.y.abs() {
      self.scroll_offset.x = (self.scroll_offset.x - offset.x)
        .max((screen_size.width - self.max_line_length) as f64) // TODO
        .min(0.0);
    } else {
      self.scroll_offset.y = (self.scroll_offset.y + offset.y).min(0.0).max(
        -((self.text.borrow().len_lines() - 3) as f32 * self.font_height)
          as f64,
      );
    }

    self.cursor.rect.resize(
      screen_size,
      Dimensions {
        x: self.dimensions.x
          + self.scroll_offset.x as f32
          + self.cursor.x_offset,
        y: self.dimensions.y
          + self.scroll_offset.y as f32
          + (self.cursor.row as f32 * self.font_height),
        ..self.cursor.rect.dimensions
      },
    );
  }

  fn click(
    &mut self,
    position: PhysicalPosition<f64>,
    _screen_size: PhysicalSize<f32>,
  ) {
    let line = ((position.y - self.scroll_offset.y) / self.font_height as f64)
      .floor() as usize;
    let layout = Layout::default_wrap();

    let text = self.text.borrow();
    let text_line = text.line(line);
    let string = text_line.to_string();
    let section_glyphs = &layout.calculate_glyphs(
      &[self.font.clone()],
      &SectionGeometry {
        ..Default::default()
      },
      &[Text::new(&string).with_scale(self.font_height)],
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
      .min(self.text.borrow().len_lines());

    let text = self.text.borrow();
    glyph_brush.queue(Section {
      screen_position: (
        self.dimensions.x + self.scroll_offset.x as f32,
        -(((-self.scroll_offset.y as f32) % self.font_height)
          - self.dimensions.y),
      ),
      text: self.generate_glyph_text(&text, upper_bound, lower_bound),
      ..Section::default()
    });

    glyph_brush
      .draw_queued_with_transform_and_scissoring(
        device,
        staging_belt,
        encoder,
        target,
        wgpu_glyph::orthographic_projection(size.width, size.height),
        self.dimensions.into(),
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
