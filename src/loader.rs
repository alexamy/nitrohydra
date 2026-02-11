use crate::cache;
use eframe::egui;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::SystemTime;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];
const MAX_TEXTURE_SIZE: u32 = 512;

pub struct LoadResult {
    modified: SystemTime,
    name: String,
    image: egui::ColorImage,
    dimensions: [u32; 2],
}

pub enum Poll {
    Image(SystemTime, String, egui::ColorImage, [u32; 2]),
    Error(String),
    Pending,
    Done,
}

type LoadResultPayload = Result<LoadResult, String>;

pub struct ImageLoader {
    rx: mpsc::Receiver<LoadResultPayload>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for ImageLoader {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl ImageLoader {
    pub fn start(path: String, ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::sync_channel(32);
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_clone = Arc::clone(&cancelled);
        std::thread::spawn(move || {
            decode(&path, tx, ctx, cancelled_clone);
        });
        Self { rx, cancelled }
    }

    pub fn poll(&self) -> Poll {
        match self.rx.try_recv() {
            Ok(Ok(LoadResult { modified, name, image, dimensions })) => Poll::Image(modified, name, image, dimensions),
            Ok(Err(e)) => Poll::Error(e),
            Err(mpsc::TryRecvError::Empty) => Poll::Pending,
            Err(mpsc::TryRecvError::Disconnected) => Poll::Done,
        }
    }
}

fn decode(
    path: &str,
    tx: mpsc::SyncSender<LoadResultPayload>,
    ctx: egui::Context,
    cancelled: Arc<AtomicBool>,
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
        if cancelled.load(Ordering::Relaxed) { return; }
        let modified = path.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        let Ok((image, dimensions)) = load_image(path) else { return };
        let name = path.to_string_lossy().into_owned();
        if tx.send(Ok(LoadResult { modified, name, image, dimensions })).is_ok() {
            ctx.request_repaint();
        }
    });
}

fn load_image(path: &Path) -> Result<(egui::ColorImage, [u32; 2]), image::ImageError> {
    let (w, h) = image::image_dimensions(path)?;
    let color_image = if let Some(cached) = cache::load(path) {
        cached
    } else {
        let img = image::open(path)?;
        let thumbnail = img.thumbnail(MAX_TEXTURE_SIZE, MAX_TEXTURE_SIZE);
        cache::save(path, &thumbnail);
        cache::to_color_image(&thumbnail)
    };
    Ok((color_image, [w, h]))
}
