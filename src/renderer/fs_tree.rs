use crate::renderer::rectangle::Rectangle;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;
use wgpu::util::StagingBelt;
use wgpu::{CommandEncoder, Device, TextureView};
use wgpu_glyph::{GlyphBrush, Region, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};

struct TreeEntry {
  name: String,
  #[allow(dead_code)]
  path: PathBuf,
  inset: usize,
  sub_entry: Option<Vec<TreeEntry>>,
  folded: bool,
}

impl TreeEntry {
  fn gen(
    path: &PathBuf,
    inset: usize,
    ignore_set: &HashSet<OsString>,
  ) -> Vec<Self> {
    let mut sections = vec![];

    let entries = path.read_dir().unwrap().collect::<Vec<_>>();
    let mut entries = entries
      .iter()
      .filter_map(|x| x.as_ref().ok())
      .collect::<Vec<_>>();
    entries.sort_unstable_by(|a, b| {
      b.file_type()
        .unwrap()
        .is_dir()
        .cmp(&a.file_type().unwrap().is_dir())
        .then(a.file_name().cmp(&b.file_name()))
    });

    for entry in entries {
      let path = entry.path();

      if ignore_set.contains(&entry.file_name()) {
        continue;
      }

      if path.is_dir() {
        sections.push(Self {
          name: entry.file_name().into_string().unwrap(),
          sub_entry: Some(Self::gen(&path, inset + 1, ignore_set)),
          path,
          inset,
          folded: true,
        });
      } else if path.is_file() {
        sections.push(Self {
          name: entry.file_name().into_string().unwrap(),
          path,
          inset,
          sub_entry: None,
          folded: false,
        });
      }
    }

    sections
  }

  fn new(path: PathBuf, ignore_set: HashSet<OsString>) -> Self {
    assert!(path.is_dir());

    TreeEntry {
      name: path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap(),
      sub_entry: Some(Self::gen(&path, 1, &ignore_set)),
      path,
      inset: 0,
      folded: false,
    }
  }

  fn walk<F>(&mut self, cb: &mut F) -> i32
  where
    F: FnMut(&mut Self) -> bool,
  {
    fn walk_inner<F>(fs_tree: &mut TreeEntry, cb: &mut F, counter: &mut i32)
    where
      F: FnMut(&mut TreeEntry) -> bool,
    {
      *counter += 1;
      let depth_continue = cb(fs_tree);

      if let Some(tree) = fs_tree.sub_entry.as_mut() {
        if depth_continue {
          for entry in tree.iter_mut() {
            *counter += entry.walk(cb);
          }
        }
      }
    }

    let mut counter = 0;
    walk_inner(self, cb, &mut counter);
    counter
  }
}

pub struct FsTree {
  rect: Rectangle,
  font_height: f32,
  pub position: PhysicalPosition<u32>,
  pub size: PhysicalSize<u32>,
  scroll_offset: PhysicalPosition<f64>,
  tree: TreeEntry,
  counter: i32,
}

impl FsTree {
  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<u32>,
    font_height: f32,
    position: PhysicalPosition<u32>,
    size: PhysicalSize<u32>,
    path: PathBuf,
  ) -> Self {
    let rect = Rectangle::new(
      device,
      screen_size,
      PhysicalPosition { x: 0.0, y: 0.0 },
      size,
      [0.04, 0.04, 0.04],
      None,
    );

    let mut ignore_set = HashSet::new();
    ignore_set.insert(OsString::from_str(".DS_Store").unwrap());

    Self {
      rect,
      font_height,
      position,
      size,
      scroll_offset: PhysicalPosition { x: 0.0, y: 0.0 },
      tree: TreeEntry::new(path, ignore_set),
      counter: 0,
    }
  }
}

impl super::RenderElement for FsTree {
  fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.rect]
  }

  fn resize(&mut self, screen_size: PhysicalSize<u32>) {
    self.rect.resize(
      screen_size,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalSize {
        width: self.size.width,
        height: screen_size.height,
      },
    );
    self.size.height = screen_size.height;
  }

  fn scroll(
    &mut self,
    offset: PhysicalPosition<f64>,
    _size: PhysicalSize<u32>,
  ) {
    self.scroll_offset.y = (self.scroll_offset.y + offset.y)
      .min(0.0)
      .max(-((self.counter - 3) as f32 * self.font_height) as f64);
  }

  fn redraw(
    &mut self,
    glyph_brush: &mut GlyphBrush<()>,
    device: &Device,
    staging_belt: &mut StagingBelt,
    encoder: &mut CommandEncoder,
    target: &TextureView,
    screen_size: PhysicalSize<u32>,
  ) {
    let font_height = self.font_height;
    let y_offset = self.scroll_offset.y;
    let mut index = 0;
    self.counter = self.tree.walk(&mut |entry| {
      glyph_brush.queue(Section {
        screen_position: (
          entry.inset as f32 * font_height,
          (index as f32 * font_height) + y_offset as f32,
        ),
        bounds: (f32::INFINITY, f32::INFINITY),
        layout: Default::default(),
        text: vec![Text::new(&entry.name)
          .with_scale(font_height)
          .with_color([0.0, 0.9, 0.0, 1.0])],
      });
      index += 1;

      !entry.folded
    });

    glyph_brush
      .draw_queued_with_transform_and_scissoring(
        device,
        staging_belt,
        encoder,
        target,
        wgpu_glyph::orthographic_projection(
          screen_size.width,
          screen_size.height,
        ),
        Region {
          x: 0,
          y: 0,
          width: self.size.width,
          height: self.size.height,
        },
      )
      .unwrap();
  }

  fn click(&mut self, position: PhysicalPosition<f64>) {
    let index = ((position.y - self.scroll_offset.y) / self.font_height as f64)
      .floor() as usize;
    let mut i = 0;
    self.counter = self.tree.walk(&mut |entry| {
      if index == i && entry.sub_entry.is_some() {
        entry.folded = !entry.folded;
      }
      i += 1;
      !entry.folded
    });
  }
}
