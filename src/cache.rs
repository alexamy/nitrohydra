use eframe::egui;
use md5::{Digest, Md5};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn load(source: &Path) -> Option<egui::ColorImage> {
    let cache = path(source)?;
    if mtime(&cache)? < mtime(source)? {
        return None; // stale
    }
    Some(to_color_image(&image::open(&cache).ok()?))
}

pub fn save(source: &Path, thumbnail: &image::DynamicImage) {
    let Some(cache) = path(source) else { return };
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = thumbnail.save(&cache);
}

fn path(source: &Path) -> Option<PathBuf> {
    let hash = format!("{:x}", Md5::digest(source.to_string_lossy().as_bytes()));
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cache/nitrohydra").join(format!("{hash}.png")))
}

fn mtime(path: &Path) -> Option<SystemTime> {
    path.metadata().ok()?.modified().ok()
}

pub fn load_dynamic(source: &Path) -> Option<image::DynamicImage> {
    let cache = path(source)?;
    if mtime(&cache)? < mtime(source)? {
        return None;
    }
    image::open(&cache).ok()
}

pub fn to_color_image(img: &image::DynamicImage) -> egui::ColorImage {
    let rgba = img.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
}
