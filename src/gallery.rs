use std::time::SystemTime;

use eframe::egui;

use crate::loader::{ImageLoader, Poll};

pub enum State {
    Empty,
    Error(String),
    Loaded(Vec<ImageEntry>),
}

pub struct ImageEntry {
    pub texture: egui::TextureHandle,
    pub original_size: [u32; 2],
    pub modified: SystemTime,
}

impl Clone for ImageEntry {
    fn clone(&self) -> Self {
        Self {
            texture: self.texture.clone(),
            original_size: self.original_size,
            modified: self.modified,
        }
    }
}

pub struct Gallery {
    state: State,
    loader: Option<ImageLoader>,
}

impl Gallery {
    pub fn new() -> Self {
        Self {
            state: State::Empty,
            loader: None,
        }
    }

    pub fn load(&mut self, path: &str, ctx: &egui::Context) {
        self.loader = Some(ImageLoader::start(path.to_string(), ctx.clone()));
        self.state = State::Loaded(vec![]);
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(loader) = &self.loader else { return };
        loop {
            match loader.poll() {
                Poll::Image(modified, name, image, dimensions) => {
                    let texture = ctx.load_texture(name, image, Default::default());
                    let entry = ImageEntry {
                        texture,
                        original_size: dimensions,
                        modified,
                    };
                    if let State::Loaded(v) = &mut self.state {
                        v.push(entry);
                        v.sort_by(|a, b| b.modified.cmp(&a.modified));
                    } else {
                        self.state = State::Loaded(vec![entry]);
                    }
                }
                Poll::Error(e) => {
                    self.state = State::Error(e);
                    self.loader = None;
                    break;
                }
                Poll::Pending => break,
                Poll::Done => {
                    if !matches!(&self.state, State::Loaded(_)) {
                        self.state = State::Loaded(vec![]);
                    }
                    self.loader = None;
                    break;
                }
            }
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loader.is_some()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn entries(&self) -> Option<&[ImageEntry]> {
        match &self.state {
            State::Loaded(entries) => Some(entries),
            _ => None,
        }
    }
}
