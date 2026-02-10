use eframe::egui;
use md5::{Digest, Md5};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::SystemTime;

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
        let (tx, rx) = mpsc::channel();
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
    tx: mpsc::Sender<Result<(String, egui::ColorImage), String>>,
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
        let color_image = load_image(path);
        let name = path.to_string_lossy().into_owned();
        if tx.send(Ok((name, color_image))).is_ok() {
            ctx.request_repaint();
        }
    });
}

fn load_image(path: &Path) -> egui::ColorImage {
    if let Some(cached) = load_from_cache(path) {
        return cached;
    }
    let img = image::open(path).unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1));
    let thumbnail = img.thumbnail(MAX_TEXTURE_SIZE, MAX_TEXTURE_SIZE);
    save_to_cache(path, &thumbnail);
    to_color_image(&thumbnail)
}

fn to_color_image(img: &image::DynamicImage) -> egui::ColorImage {
    let rgba = img.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
}

// --- Thumbnail cache ---

fn cache_path(source: &Path) -> Option<PathBuf> {
    let hash = format!("{:x}", Md5::digest(source.to_string_lossy().as_bytes()));
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cache/nitrohydra").join(format!("{hash}.png")))
}

fn load_from_cache(source: &Path) -> Option<egui::ColorImage> {
    let cache = cache_path(source)?;
    if mtime(&cache)? < mtime(source)? {
        return None; // stale
    }
    Some(to_color_image(&image::open(&cache).ok()?))
}

fn save_to_cache(source: &Path, thumbnail: &image::DynamicImage) {
    let Some(cache) = cache_path(source) else { return };
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = thumbnail.save(&cache);
}

fn mtime(path: &Path) -> Option<SystemTime> {
    path.metadata().ok()?.modified().ok()
}
