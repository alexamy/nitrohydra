use crate::cache;
use eframe::egui;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];
const MAX_TEXTURE_SIZE: u32 = 512;

pub enum Poll {
    Image(String, egui::ColorImage),
    Error(String),
    Pending,
    Done,
}

pub struct ImageLoader {
    rx: mpsc::Receiver<Result<(String, egui::ColorImage), String>>,
}

impl ImageLoader {
    pub fn start(path: String, ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::sync_channel(32);
        std::thread::spawn(move || {
            decode(&path, tx, ctx);
        });
        Self { rx }
    }

    pub fn poll(&self) -> Poll {
        match self.rx.try_recv() {
            Ok(Ok((name, img))) => Poll::Image(name, img),
            Ok(Err(e)) => Poll::Error(e),
            Err(mpsc::TryRecvError::Empty) => Poll::Pending,
            Err(mpsc::TryRecvError::Disconnected) => Poll::Done,
        }
    }
}

fn decode(
    path: &str,
    tx: mpsc::SyncSender<Result<(String, egui::ColorImage), String>>,
    ctx: egui::Context,
) {
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            let _ = tx.send(Err(format!("Error: {e}")));
            ctx.request_repaint();
            return;
        }
    };

    let paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .collect();

    paths.par_iter().for_each_with(tx, |tx, path| {
        let Ok(color_image) = load_image(path) else { return };
        let name = path.to_string_lossy().into_owned();
        if tx.send(Ok((name, color_image))).is_ok() {
            ctx.request_repaint();
        }
    });
}

fn load_image(path: &Path) -> Result<egui::ColorImage, image::ImageError> {
    if let Some(cached) = cache::load(path) {
        return Ok(cached);
    }
    let img = image::open(path)?;
    let thumbnail = img.thumbnail(MAX_TEXTURE_SIZE, MAX_TEXTURE_SIZE);
    cache::save(path, &thumbnail);
    Ok(cache::to_color_image(&thumbnail))
}
