fn main() -> Result<(), anyhow::Error> {
  for language in std::fs::read_dir("./tree-sitter-languages")? {
    let language = language?;

    let dir = language.path().join("src");
    cc::Build::new()
      .include(&dir)
      .file(dir.join("parser.c"))
      .file(dir.join("scanner.c"))
      .extra_warnings(false)
      .compile(&format!(
        "tree-sitter-{}",
        language.file_name().to_str().unwrap()
      ));
  }

  Ok(())
}
