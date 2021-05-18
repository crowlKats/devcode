#![deny(warnings)]

mod renderer;

use crate::renderer::input::TextInput;
use std::collections::HashMap;
use std::path::PathBuf;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};

fn main() -> Result<(), anyhow::Error> {
  let args: Vec<String> = std::env::args().collect();

  let file = args
    .get(1)
    .ok_or_else(|| anyhow::anyhow!("no file provided"))?;
  let filepath = std::path::PathBuf::from(file);
  if !filepath.exists() {
    anyhow::bail!("path doesn't exist");
  }
  if !filepath.is_file() {
    anyhow::bail!("path isn't a file");
  }

  let font = get_font(args.get(2))?;

  let event_loop = winit::event_loop::EventLoop::new();
  let mut ren = futures::executor::block_on(async {
    renderer::Renderer::new(&event_loop, font, filepath).await
  })?;

  ren.window.request_redraw();

  let mut mouse_pos = PhysicalPosition::new(0.0f64, 0.0f64);

  event_loop.run(move |event, _, control_flow| match event {
    winit::event::Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => {
        ren.resize(size.cast());
        ren.window.request_redraw();
      }
      WindowEvent::CloseRequested => {
        *control_flow = winit::event_loop::ControlFlow::Exit;
      }
      WindowEvent::MouseWheel { delta, .. } => {
        match delta {
          MouseScrollDelta::LineDelta(x, y) => {
            ren.scroll(
              winit::dpi::PhysicalPosition {
                x: x as f64,
                y: y as f64,
              },
              mouse_pos,
            );
          }
          MouseScrollDelta::PixelDelta(delta) => {
            ren.scroll(delta, mouse_pos);
          }
        }
        ren.window.request_redraw();
      }
      WindowEvent::KeyboardInput { input, .. } => {
        if input.state == ElementState::Pressed {
          ren
            .code_views
            .borrow_mut()
            .input_special(ren.size.cast(), input.virtual_keycode.unwrap());
          ren.window.request_redraw();
        }
      }
      WindowEvent::ReceivedCharacter(ch) => {
        ren.code_views.borrow_mut().input_char(ren.size.cast(), ch);
      }
      WindowEvent::CursorMoved { position, .. } => mouse_pos = position,
      WindowEvent::MouseInput { state, .. } => {
        ren.click(mouse_pos, state);
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

fn get_font_map() -> HashMap<String, PathBuf> {
  let mut fonts = vec![];
  #[cfg(target_os = "linux")]
  {
    let path = std::path::Path::new("/usr/share/fonts");
    extend_fonts!(fonts, path);
    let path = std::path::Path::new("/usr/local/share/fonts");
    extend_fonts!(fonts, path);
    let expanded_path = shellexpand::tilde("~/.fonts");
    let expanded_path = expanded_path.to_string();
    let path = std::path::Path::new(&expanded_path);
    extend_fonts!(fonts, path);
  }
  #[cfg(target_os = "macos")]
  {
    let path = std::path::Path::new("/Library/Fonts");
    extend_fonts!(fonts, path);
    let path = std::path::Path::new("/System/Library/Fonts");
    extend_fonts!(fonts, path);
    let expanded_path = shellexpand::tilde("~/Library/Fonts");
    let expanded_path = expanded_path.to_string();
    let path = std::path::Path::new(&expanded_path);
    extend_fonts!(fonts, path);
  }
  #[cfg(target_os = "windows")]
  {
    let path = std::path::Path::new(r"C:\Windows\Fonts");
    extend_fonts!(fonts, path);
  }

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
    .collect()
}

fn get_font(
  name: Option<&String>,
) -> Result<wgpu_glyph::ab_glyph::FontArc, anyhow::Error> {
  let fonts = get_font_map();
  let font = name
    .and_then(|font| fonts.get(font))
    .map(std::fs::read)
    .transpose()?
    .unwrap_or_else(|| include_bytes!("./JetBrainsMono-Regular.ttf").to_vec());

  Ok(wgpu_glyph::ab_glyph::FontArc::try_from_vec(font)?)
}

#[cfg(test)]
mod tests {
  use crate::*;

  #[test]
  fn font_map_contains() {
    assert!(get_font_map().contains_key(&String::from("Montserrat-Regular")));
  }

  #[test]
  fn get_specific_font() {
    assert!(get_font(Some(&String::from("Montserrat-Regular"))).is_ok());
  }
  #[test]
  fn get_default_font() {
    assert!(get_font(None).is_ok());
  }
}
