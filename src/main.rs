#![deny(warnings)]

mod renderer;

use std::collections::HashMap;
use std::path::PathBuf;
use wgpu_glyph::ab_glyph;
use winit::event::WindowEvent;

fn main() -> Result<(), anyhow::Error> {
  let args: Vec<String> = std::env::args().collect();

  let file = args
    .get(1)
    .ok_or_else(|| anyhow::anyhow!("no file provided"))?;
  let filepath = std::path::PathBuf::from(file);
  if !filepath.exists() {
    anyhow::bail!("path doesn't exist");
  } else if !filepath.is_file() {
    anyhow::bail!("path isn't a file");
  }
  let text = std::fs::read_to_string(filepath)?;

  let font = {
    let fonts = {
      #[cfg(target_os = "macos")]
      {
        let path = std::path::Path::new("/Library/Fonts/");
        let mut fonts = std::fs::read_dir(path)?.collect::<Vec<_>>();
        let path = std::path::Path::new("/System/Library/Fonts/");
        fonts.extend(std::fs::read_dir(path)?);
        let expanded_path = shellexpand::tilde("~/Library/Fonts");
        let expanded_path = expanded_path.to_string();
        let path = std::path::Path::new(&expanded_path);
        fonts.extend(std::fs::read_dir(path)?);
        fonts
      }
      #[cfg(target_os = "windows")]
      {
        let path = std::path::Path::new(r"C:\Windows\Fonts");
        std::fs::read_dir(path)?.collect::<Vec<_>>()
      }
      #[cfg(target_os = "linux")]
      {
        let path = std::path::Path::new("/usr/share/fonts");
        let mut fonts = std::fs::read_dir(path)?.collect::<Vec<_>>();
        let path = std::path::Path::new("/usr/local/share/fonts");
        fonts.extend(std::fs::read_dir(path)?);
        let path = std::path::Path::new("~/.fonts");
        fonts.extend(std::fs::read_dir(path)?);
        let expanded_path = shellexpand::tilde("~/Library/Fonts");
        let expanded_path = expanded_path.to_string();
        let path = std::path::Path::new(&expanded_path);
        fonts.extend(std::fs::read_dir(path)?);
        fonts
      }
    };
    let fonts = fonts
      .iter()
      .filter(|font| font.as_ref().unwrap().path().is_file())
      .map(|font| {
        let font_path = font.as_ref().unwrap().path();
        (
          font_path
            .file_stem()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap(),
          font_path,
        )
      })
      .collect::<HashMap<String, PathBuf>>();
    let font = args
      .get(2)
      .and_then(|font| fonts.get(font))
      .unwrap_or_else(|| fonts.values().next().unwrap());
    let font_data = std::fs::read(font)?;

    ab_glyph::FontArc::try_from_vec(font_data)?
  };

  let event_loop = winit::event_loop::EventLoop::new();
  let mut ren = futures::executor::block_on(async {
    renderer::Renderer::new(&event_loop, font, text).await
  })?;

  ren.window.request_redraw();

  event_loop.run(move |event, _, control_flow| match event {
    winit::event::Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => ren.resize(size),
      WindowEvent::CloseRequested => {
        *control_flow = winit::event_loop::ControlFlow::Exit
      }
      _ => {}
    },
    winit::event::Event::RedrawRequested(_) => ren.redraw().unwrap(),
    _ => *control_flow = winit::event_loop::ControlFlow::Wait,
  });
}
