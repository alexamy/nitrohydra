use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use eframe::egui;

use crate::monitors::Monitor;
use crate::{cache, wallpaper};

pub struct PreviewJob {
    rx: Option<mpsc::Receiver<Result<egui::ColorImage, String>>>,
    texture: Option<egui::TextureHandle>,
    started_at: Option<Instant>,
}

impl PreviewJob {
    const IMAGE_HEIGHT: f32 = 120.0;

    pub fn new() -> Self {
        Self {
            rx: None,
            texture: None,
            started_at: None,
        }
    }

    pub fn start(&mut self, assignments: Vec<(PathBuf, Monitor)>, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result =
                wallpaper::compose_preview(&assignments).map(|img| cache::to_color_image(&img));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.rx = Some(rx);
        self.started_at = Some(Instant::now());
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else { return };
        match rx.try_recv() {
            Ok(Ok(color_image)) => {
                self.texture =
                    Some(ctx.load_texture("preview", color_image, egui::TextureOptions::LINEAR));
                self.rx = None;
                self.started_at = None;
            }
            Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                self.rx = None;
                self.started_at = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    pub fn is_running(&self) -> bool {
        self.rx.is_some()
    }

    pub fn is_running_slow(&self) -> bool {
        self.started_at
            .is_some_and(|t| t.elapsed() > std::time::Duration::from_millis(500))
    }

    pub fn has_texture(&self) -> bool {
        self.texture.is_some()
    }

    pub fn clear(&mut self) {
        self.texture = None;
        self.rx = None;
    }

    pub fn show_image(&self, ui: &mut egui::Ui) {
        let Some(texture) = &self.texture else { return };
        let tex_size = texture.size_vec2();
        let aspect = tex_size.x / tex_size.y;
        let h = Self::IMAGE_HEIGHT;
        let w = h * aspect;
        ui.add(
            egui::Image::new(egui::load::SizedTexture::from_handle(texture))
                .fit_to_exact_size(egui::vec2(w, h)),
        );
    }
}
