use std::convert::TryFrom;
use tree_sitter_highlight::{
  HighlightConfiguration, HighlightEvent, Highlighter,
};

#[derive(Copy, Clone, Debug, num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum HighlightNames {
  Constant,
  ConstantBuiltin,
  Tag,
  Type,
  TypeBuiltin,
  Constructor,
  Function,
  FunctionBuiltin,
  FunctionMethod,
  FunctionMacro,
  Property,
  Comment,
  PunctuationBracket,
  PunctuationDelimiter,
  PunctuationSpecial,
  Variable,
  VariableParameter,
  VariableBuiltin,
  Label,
  Keyword,
  String,
  StringSpecial,
  Escape,
  Attribute,
  Operator,
  Embedded,
  Number,
  InjectionLanguage,
  InjectionContent,
  LocalScope,
  LocalDefinition,
  LocalReference,
}

impl HighlightNames {
  const VARIANTS: [HighlightNames; 32] = [
    HighlightNames::Constant,
    HighlightNames::ConstantBuiltin,
    HighlightNames::Tag,
    HighlightNames::Type,
    HighlightNames::TypeBuiltin,
    HighlightNames::Constructor,
    HighlightNames::Function,
    HighlightNames::FunctionBuiltin,
    HighlightNames::FunctionMethod,
    HighlightNames::FunctionMacro,
    HighlightNames::Property,
    HighlightNames::Comment,
    HighlightNames::PunctuationBracket,
    HighlightNames::PunctuationDelimiter,
    HighlightNames::PunctuationSpecial,
    HighlightNames::Variable,
    HighlightNames::VariableParameter,
    HighlightNames::VariableBuiltin,
    HighlightNames::Label,
    HighlightNames::Keyword,
    HighlightNames::String,
    HighlightNames::StringSpecial,
    HighlightNames::Escape,
    HighlightNames::Attribute,
    HighlightNames::Operator,
    HighlightNames::Embedded,
    HighlightNames::Number,
    HighlightNames::InjectionLanguage,
    HighlightNames::InjectionContent,
    HighlightNames::LocalScope,
    HighlightNames::LocalDefinition,
    HighlightNames::LocalReference,
  ];

  pub fn color(&self) -> [f32; 4] {
    #[allow(clippy::excessive_precision)]
    match self {
      HighlightNames::Constant => [0.59607843, 0.4627451, 0.66666667, 1.0],
      HighlightNames::ConstantBuiltin => {
        [0.65882353, 0.33333333, 0.44705882, 1.0]
      }
      HighlightNames::Tag => [0.94117647, 0.77647059, 0.45490196, 1.0], // TODO
      HighlightNames::Type => [0.94117647, 0.77647059, 0.45490196, 1.0], //
      HighlightNames::TypeBuiltin => [0.8, 0.47058824, 0.19607843, 1.0], //
      HighlightNames::Constructor => [0.91372549, 0.74509804, 0.40784314, 1.0], // TODO
      HighlightNames::Function => [0.9, 0.9, 0.9, 1.0], // TODO: function usage and definition
      HighlightNames::FunctionBuiltin => [0.9, 0.9, 0.9, 1.0], // TODO: function usage and definition
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
      HighlightNames::PunctuationSpecial => {
        [0.278431371, 0.60784314, 0.49411765, 1.0] // TODO
      }
      HighlightNames::Variable => [0.8, 0.47058824, 0.19607843, 1.0], // TODO
      HighlightNames::VariableParameter => [0.8, 0.4, 0.4, 1.0],      //
      HighlightNames::VariableBuiltin => [0.8, 0.47058824, 0.19607843, 1.0],
      HighlightNames::Label => [0.1254902, 0.6, 0.61568627, 1.0], //
      HighlightNames::Keyword => [0.8, 0.47058824, 0.19607843, 1.0], //
      HighlightNames::String => [0.50588235, 0.72941176, 0.34901961, 1.0], //
      HighlightNames::StringSpecial => {
        [0.50588235, 0.72941176, 0.34901961, 1.0]
      } // TODO
      HighlightNames::Escape => [0.52941176, 0.74117647, 0.77647059, 1.0], //
      HighlightNames::Attribute => [0.83111111, 0.70980392, 0.16078431, 1.0],
      HighlightNames::Operator => [0.278431371, 0.60784314, 0.49411765, 1.0], //
      HighlightNames::Embedded => [0.278431371, 0.60784314, 0.49411765, 1.0], // TODO
      HighlightNames::Number => [0.278431371, 0.60784314, 0.49411765, 1.0], // TODO

      HighlightNames::InjectionLanguage => {
        [0.278431371, 0.60784314, 0.49411765, 1.0]
      } // TODO
      HighlightNames::InjectionContent => {
        [0.278431371, 0.60784314, 0.49411765, 1.0]
      } // TODO
      HighlightNames::LocalScope => [0.278431371, 0.60784314, 0.49411765, 1.0], // TODO
      HighlightNames::LocalDefinition => {
        [0.278431371, 0.60784314, 0.49411765, 1.0]
      } // TODO
      HighlightNames::LocalReference => {
        [0.278431371, 0.60784314, 0.49411765, 1.0]
      } // TODO
    }
  }
}

impl std::fmt::Display for HighlightNames {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      HighlightNames::Constant => "constant",
      HighlightNames::ConstantBuiltin => "constant.builtin",
      HighlightNames::Tag => "tag",
      HighlightNames::Type => "type",
      HighlightNames::TypeBuiltin => "type.builtin",
      HighlightNames::Constructor => "constructor",
      HighlightNames::Function => "function",
      HighlightNames::FunctionBuiltin => "function.builtin",
      HighlightNames::FunctionMethod => "function.method",
      HighlightNames::FunctionMacro => "function.macro",
      HighlightNames::Property => "property",
      HighlightNames::Comment => "comment",
      HighlightNames::PunctuationBracket => "punctuation.bracket",
      HighlightNames::PunctuationDelimiter => "punctuation.delimiter",
      HighlightNames::PunctuationSpecial => "punctuation.special",
      HighlightNames::Variable => "variable",
      HighlightNames::VariableParameter => "variable.parameter",
      HighlightNames::VariableBuiltin => "variable.builtin",
      HighlightNames::Label => "label",
      HighlightNames::Keyword => "keyword",
      HighlightNames::String => "string",
      HighlightNames::StringSpecial => "string.special",
      HighlightNames::Escape => "escape",
      HighlightNames::Attribute => "attribute",
      HighlightNames::Operator => "operator",
      HighlightNames::Embedded => "embedded",
      HighlightNames::Number => "number",
      HighlightNames::InjectionLanguage => "injection.language",
      HighlightNames::InjectionContent => "injection.content",
      HighlightNames::LocalScope => "local.scope",
      HighlightNames::LocalDefinition => "local.definition",
      HighlightNames::LocalReference => "local.reference",
    })
  }
}

pub struct Config {
  config: HighlightConfiguration,
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
  let mut config = match ext?.to_string_lossy().as_ref() {
    "cpp" | "cxx" | "cc" => HighlightConfiguration::new(
      tree_sitter_cpp::language(),
      tree_sitter_cpp::HIGHLIGHT_QUERY,
      "",
      "",
    ),
    "java" => HighlightConfiguration::new(
      tree_sitter_java::language(),
      tree_sitter_java::HIGHLIGHT_QUERY,
      "",
      "",
    ),
    "js" | "cjs" | "mjs" => HighlightConfiguration::new(
      tree_sitter_javascript::language(),
      tree_sitter_javascript::HIGHLIGHT_QUERY,
      tree_sitter_javascript::INJECTION_QUERY,
      tree_sitter_javascript::LOCALS_QUERY,
    ),
    "jsx" => HighlightConfiguration::new(
      tree_sitter_javascript::language(),
      tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
      tree_sitter_javascript::INJECTION_QUERY,
      tree_sitter_javascript::LOCALS_QUERY,
    ),
    "ml" => HighlightConfiguration::new(
      tree_sitter_ocaml::language_ocaml(),
      tree_sitter_ocaml::HIGHLIGHT_QUERY,
      "",
      tree_sitter_ocaml::LOCALS_QUERY,
    ),
    "mli" => HighlightConfiguration::new(
      tree_sitter_ocaml::language_ocaml_interface(),
      tree_sitter_ocaml::HIGHLIGHT_QUERY,
      "",
      tree_sitter_ocaml::LOCALS_QUERY,
    ),
    "py" => HighlightConfiguration::new(
      tree_sitter_python::language(),
      tree_sitter_python::HIGHLIGHT_QUERY,
      "",
      "",
    ),
    "rs" => tree_sitter_highlight::HighlightConfiguration::new(
      tree_sitter_rust::language(),
      tree_sitter_rust::HIGHLIGHT_QUERY,
      "",
      "",
    ),
    "ts" => tree_sitter_highlight::HighlightConfiguration::new(
      tree_sitter_typescript::language_typescript(),
      tree_sitter_typescript::HIGHLIGHT_QUERY,
      "",
      tree_sitter_typescript::LOCALS_QUERY,
    ),
    "tsx" => tree_sitter_highlight::HighlightConfiguration::new(
      tree_sitter_typescript::language_tsx(),
      tree_sitter_typescript::HIGHLIGHT_QUERY,
      "",
      tree_sitter_typescript::LOCALS_QUERY,
    ),
    _ => return None,
  }
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

#[cfg(test)]
#[test]
fn names_contains_all_language_names() {
  let names = HighlightNames::VARIANTS
    .iter()
    .map(|v| v.to_string())
    .collect::<Vec<String>>();

  for lang in [
    "cpp", "java", "js", "jsx", "ml", "mli", "py", "rs", "ts", "tsx",
  ] {
    let config =
      config_from_extension(Some(std::ffi::OsStr::new(lang))).unwrap();
    assert!(
      config
        .config
        .names()
        .iter()
        .all(|name| names.contains(name)),
      "Language '{}' doesnt have all names in Names struct. Missing are: {:?}",
      lang,
      config
        .config
        .names()
        .iter()
        .filter(|name| !names.contains(name))
        .collect::<Vec<_>>()
    );
  }
}
