use std::path::PathBuf;

use eframe::egui;

use crate::apply_job::ApplyJob;
use crate::gallery::Gallery;
use crate::monitors::{self, Monitor};
use crate::preview::PreviewJob;
use crate::selection::Selection;
use crate::wallpaper;

pub(crate) struct App {
    pub(crate) path: String,
    pub(crate) gallery: Gallery,
    pub(crate) thumb_size: f32,
    pub(crate) selected: Selection,
    pub(crate) monitors: Result<Vec<Monitor>, String>,
    pub(crate) apply: ApplyJob,
    pub(crate) preview: PreviewJob,
    pub(crate) preview_items: Option<[usize; 2]>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            path: String::new(),
            gallery: Gallery::new(),
            thumb_size: 150.0,
            selected: Selection::new(),
            monitors: Ok(Vec::new()),
            apply: ApplyJob::new(),
            preview: PreviewJob::new(),
            preview_items: None,
        }
    }
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let path: String = cc
            .storage
            .and_then(|s| eframe::get_value(s, "path"))
            .unwrap_or_default();

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.button_padding += egui::vec2(3.0, 3.0);
        cc.egui_ctx.set_style(style);

        let mut app = Self {
            path: path.clone(),
            monitors: monitors::detect(),
            ..Self::default()
        };
        app.gallery.load(&path, &cc.egui_ctx);
        app
    }

    pub(crate) fn load_images(&mut self, ctx: &egui::Context) {
        self.gallery.load(&self.path, ctx);
        self.selected.clear();
    }

    pub(crate) fn auto_preview(&mut self, ctx: &egui::Context) {
        if self.selected.len() != 2 || !matches!(&self.monitors, Ok(m) if m.len() >= 2) {
            if self.preview_items.is_some() {
                self.preview.clear();
                self.preview_items = None;
            }
            return;
        }

        let items = [self.selected.items()[0], self.selected.items()[1]];
        if self.preview_items == Some(items) || self.preview.is_running() {
            return;
        }

        let Some(entries) = self.gallery.entries() else {
            return;
        };
        let monitors = self.monitors.as_ref().unwrap();
        let assignments: Vec<(PathBuf, Monitor)> = self
            .selected
            .items()
            .iter()
            .zip(monitors.iter())
            .map(|(&idx, monitor)| {
                let path = PathBuf::from(entries[idx].texture.name());
                (path, monitor.clone())
            })
            .collect();
        self.preview.start(assignments, ctx);
        self.preview_items = Some(items);
    }

    pub(crate) fn handle_image_click(&mut self, index: usize, shift: bool) {
        self.apply.clear_status();
        self.selected.click(index, shift);
    }
}

pub(crate) fn show_help() {
    let bin = std::env::args()
        .next()
        .unwrap_or_else(|| "nitrohydra".into());
    eprintln!("nitrohydra — multi-monitor wallpaper composer");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {bin}                     Start the GUI");
    eprintln!("  {bin} <image1> <image2>   Join images and set as wallpaper");
    eprintln!();
    eprintln!("Images are assigned to monitors left-to-right.");
}

pub(crate) fn run_cli(left: &str, right: &str) {
    let monitors = match monitors::detect() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if monitors.len() < 2 {
        eprintln!("error: need at least 2 monitors, found {}", monitors.len());
        std::process::exit(1);
    }

    let assignments: Vec<(PathBuf, Monitor)> = vec![
        (PathBuf::from(left), monitors[0].clone()),
        (PathBuf::from(right), monitors[1].clone()),
    ];

    if let Err(e) = wallpaper::apply(&assignments, &|msg| eprintln!("{msg}")) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }

    eprintln!("Wallpaper applied!");
}
