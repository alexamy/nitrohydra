use crate::monitors::Monitor;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, RgbImage};
use rayon::prelude::*;
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

    let tiles: Vec<(RgbImage, &Monitor)> = assignments
        .par_iter()
        .map(|(path, monitor)| {
            let img = image::open(path)
                .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
            Ok((cover_resize(&img, monitor.width, monitor.height), monitor))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut canvas = RgbImage::new(canvas_w, canvas_h);
    for (tile, monitor) in &tiles {
        image::imageops::overlay(
            &mut canvas,
            tile,
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

    let scaled_w = (f64::from(src_w) * scale).ceil() as u32;
    let scaled_h = (f64::from(src_h) * scale).ceil() as u32;

    let resized = img.resize_exact(scaled_w, scaled_h, FilterType::CatmullRom);

    // Center-crop to target dimensions
    let crop_x = (scaled_w.saturating_sub(target_w)) / 2;
    let crop_y = (scaled_h.saturating_sub(target_h)) / 2;

    resized.crop_imm(crop_x, crop_y, target_w, target_h).to_rgb8()
}

fn save_composed(canvas: &RgbImage) -> Result<PathBuf, String> {
    let cache_dir = dirs_cache().join("nitrohydra");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("failed to create cache dir: {e}"))?;

    let tmp_path = cache_dir.join("_composed.tmp");
    let final_path = cache_dir.join("_composed.jpg");

    let file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("failed to create wallpaper file: {e}"))?;
    let encoder = JpegEncoder::new_with_quality(file, 95);
    canvas
        .write_with_encoder(encoder)
        .map_err(|e| format!("failed to save wallpaper: {e}"))?;

    std::fs::rename(&tmp_path, &final_path)
        .map_err(|e| format!("failed to rename wallpaper file: {e}"))?;

    Ok(final_path)
}

const SCHEMAS: &[&str] = &[
    "org.cinnamon.desktop.background",
    "org.gnome.desktop.background",
    "org.mate.background",
];

fn set_wallpaper(path: &Path) -> Result<(), String> {
    let uri = format!("file://{}", path.display());
    let mut any_ok = false;
    for schema in SCHEMAS {
        if gsettings_set(schema, "picture-uri", &uri).is_ok()
            && gsettings_set(schema, "picture-options", "spanned").is_ok()
        {
            any_ok = true;
        }
    }
    if any_ok {
        Ok(())
    } else {
        Err("no supported desktop environment found".into())
    }
}

fn gsettings_set(schema: &str, key: &str, value: &str) -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .map_err(|e| format!("failed to run gsettings: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gsettings set {schema} {key} failed: {stderr}"));
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
