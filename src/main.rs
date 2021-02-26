#![deny(warnings)]

mod renderer;

use std::collections::HashMap;
use std::path::PathBuf;
use winit::event::{MouseScrollDelta, WindowEvent};

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

  let font = get_font(args.get(2))?;

  let event_loop = winit::event_loop::EventLoop::new();
  let mut ren = futures::executor::block_on(async {
    renderer::Renderer::new(&event_loop, font, text).await
  })?;

  ren.window.request_redraw();

  event_loop.run(move |event, _, control_flow| match event {
    winit::event::Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => {
        ren.resize(size);
        ren.window.request_redraw();
      }
      WindowEvent::CloseRequested => {
        *control_flow = winit::event_loop::ControlFlow::Exit;
      }
      WindowEvent::MouseWheel { delta, .. } => {
        match delta {
          MouseScrollDelta::LineDelta(x, y) => {
            ren.offset = renderer::Offset {
              x: ren.offset.x + x,
              y: ren.offset.y + y,
            }
          }
          MouseScrollDelta::PixelDelta(delta) => {
            ren.offset = renderer::Offset {
              x: ren.offset.x + delta.x as f32,
              y: ren.offset.y + delta.y as f32,
            }
          }
        }
        ren.window.request_redraw();
      }
      _ => {}
    },
    winit::event::Event::RedrawRequested(_) => ren.redraw().unwrap(),
    _ => *control_flow = winit::event_loop::ControlFlow::Wait,
  });
}

macro_rules! extend_fonts {
  ($e: expr, $p: expr) => {
    match std::fs::read_dir($p) {
      Ok(fonts) => $e.extend(fonts),
      Err(_) => {}
    }
  };
}

fn get_font_map() -> Result<HashMap<String, PathBuf>, anyhow::Error> {
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
      extend_fonts!(fonts, path);
      let path = std::path::Path::new("~/.fonts");
      extend_fonts!(fonts, path);
      let expanded_path = shellexpand::tilde("~/Library/Fonts");
      let expanded_path = expanded_path.to_string();
      let path = std::path::Path::new(&expanded_path);
      extend_fonts!(fonts, path);
      fonts
    }
  };
  Ok(
    fonts
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
      .collect(),
  )
}

fn get_font(
  name: Option<&String>,
) -> Result<wgpu_glyph::ab_glyph::FontArc, anyhow::Error> {
  let fonts = get_font_map()?;
  let font = name
    .and_then(|font| fonts.get(font))
    .unwrap_or_else(|| fonts.values().next().unwrap());
  let font_data = std::fs::read(font)?;

  Ok(wgpu_glyph::ab_glyph::FontArc::try_from_vec(font_data)?)
}

#[cfg(test)]
mod tests {
  use crate::*;

  #[test]
  fn font_map_contains() {
    assert!(get_font_map()
      .unwrap()
      .contains_key(&String::from("Helvetica")))
  }

  #[test]
  fn get_specific_font() {
    assert!(get_font(Some(&String::from("Helvetica"))).is_ok())
  }
  #[test]
  fn get_first_font() {
    assert!(get_font(None).is_ok())
  }
}
