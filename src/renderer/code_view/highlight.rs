use std::convert::TryFrom;
use tree_sitter_highlight::{HighlightEvent, Highlighter};

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

  pub fn color(&self) -> [f32; 4] {
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
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

pub struct Config {
  config: tree_sitter_highlight::HighlightConfiguration,
  /// Vec of tuples of char_start, chard_end and HighlightName
  pub highlights: Vec<(usize, usize, Option<HighlightNames>)>,
}

impl Config {
  pub fn generate(&mut self, rope: &ropey::Rope) {
    let mut highlighter = Highlighter::new();
    let source = rope.bytes().collect::<Vec<u8>>();
    let highlights = highlighter
      .highlight(&self.config, &source, None, |_| None)
      .unwrap();

    self.highlights.clear();
    let mut current_range = (0, 0);
    let mut current_highlight = None;
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

pub fn config_from_extension(ext: Option<&std::ffi::OsStr>) -> Option<Config> {
  let mut config = tree_sitter_highlight::HighlightConfiguration::new(
    tree_sitter_rust::language(),
    tree_sitter_rust::HIGHLIGHT_QUERY,
    "",
    "",
  )
  .unwrap();

  config.configure(
    &HighlightNames::VARIANTS
      .iter()
      .map(|v| v.to_string())
      .collect::<Vec<String>>(),
  );

  Some(Config {
    config,
    highlights: vec![],
  })
}
