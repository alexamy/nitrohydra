use crate::monitors::Monitor;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, RgbImage};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Compose images to fill each monitor, save the result, and set it as the wallpaper.
pub fn apply(assignments: &[(PathBuf, Monitor)]) -> Result<(), String> {
    let canvas_w: u32 = assignments
        .iter()
        .map(|(_, m)| m.x + m.width)
        .max()
        .unwrap_or(0);
    let canvas_h: u32 = assignments
        .iter()
        .map(|(_, m)| m.y + m.height)
        .max()
        .unwrap_or(0);

    let mut canvas = RgbImage::new(canvas_w, canvas_h);

    for (path, monitor) in assignments {
        let img = image::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
        let cropped = cover_resize(&img, monitor.width, monitor.height);

        image::imageops::overlay(
            &mut canvas,
            &cropped,
            i64::from(monitor.x),
            i64::from(monitor.y),
        );
    }

    let save_path = save_composed(&canvas)?;
    set_wallpaper(&save_path)
}

/// Resize image to fully cover target dimensions (no letterboxing), then center-crop.
fn cover_resize(img: &DynamicImage, target_w: u32, target_h: u32) -> RgbImage {
    let (src_w, src_h) = img.dimensions();

    // Scale factor: pick the larger scale so the image fully covers the target
    let scale_w = f64::from(target_w) / f64::from(src_w);
    let scale_h = f64::from(target_h) / f64::from(src_h);
    let scale = scale_w.max(scale_h);

    let scaled_w = (f64::from(src_w) * scale).round() as u32;
    let scaled_h = (f64::from(src_h) * scale).round() as u32;

    let resized = img.resize_exact(scaled_w, scaled_h, FilterType::Lanczos3);

    // Center-crop to target dimensions
    let crop_x = (scaled_w.saturating_sub(target_w)) / 2;
    let crop_y = (scaled_h.saturating_sub(target_h)) / 2;

    resized.crop_imm(crop_x, crop_y, target_w, target_h).to_rgb8()
}

fn save_composed(canvas: &RgbImage) -> Result<PathBuf, String> {
    let cache_dir = dirs_cache().join("nitrohydra");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("failed to create cache dir: {e}"))?;

    let path = cache_dir.join("wallpaper.png");
    canvas
        .save(&path)
        .map_err(|e| format!("failed to save wallpaper: {e}"))?;
    Ok(path)
}

fn set_wallpaper(path: &Path) -> Result<(), String> {
    let uri = format!("file://{}", path.display());

    gsettings_set("picture-uri", &uri)?;
    gsettings_set("picture-options", "spanned")?;
    Ok(())
}

fn gsettings_set(key: &str, value: &str) -> Result<(), String> {
    let status = Command::new("gsettings")
        .args(["set", "org.cinnamon.desktop.background", key, value])
        .status()
        .map_err(|e| format!("failed to run gsettings: {e}"))?;

    if !status.success() {
        return Err(format!("gsettings set {key} failed"));
    }
    Ok(())
}

fn dirs_cache() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
            home.push(".cache");
            home
        })
}
