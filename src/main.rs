mod apply_job;
mod cache;
mod gallery;
mod loader;
mod monitors;
mod preview;
mod selection;
mod wallpaper;

use std::path::{Path, PathBuf};

use apply_job::ApplyJob;
use eframe::egui;
use gallery::{Gallery, ImageEntry};
use monitors::Monitor;
use preview::PreviewJob;
use selection::Selection;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        1 => run_gui(),
        3 => run_cli(&args[1], &args[2]),
        _ => {
            show_help();
            let is_help = args.get(1).is_some_and(|a| a == "--help" || a == "-h");
            std::process::exit(if is_help { 0 } else { 1 });
        }
    }
}

fn show_help() {
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

fn run_gui() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "nitrohydra",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    ) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run_cli(left: &str, right: &str) {
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

struct App {
    path: String,
    gallery: Gallery,
    thumb_size: f32,
    selected: Selection,
    monitors: Result<Vec<Monitor>, String>,
    apply: ApplyJob,
    preview: PreviewJob,
    preview_items: Option<[usize; 2]>,
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

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "path", &self.path);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gallery.poll(ctx);
        self.apply.poll();
        self.preview.poll(ctx);
        self.auto_preview(ctx);

        if !self.selected.is_empty() {
            egui::TopBottomPanel::bottom("selection_panel")
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .inner_margin(egui::Margin::symmetric(8.0, 12.0)),
                )
                .min_height(160.0)
                .show(ctx, |ui| {
                    self.show_selection(ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_path_input(ui);
            self.show_size_slider(ui);
            ui.separator();
            self.show_gallery(ui);
        });
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
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

    fn show_path_input(&mut self, ui: &mut egui::Ui) {
        ui.add_space(3.0);
        ui.label("Directory path:");
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            if ui.button("Open").clicked()
                && let Some(dir) = rfd::FileDialog::new()
                    .set_directory(&self.path)
                    .pick_folder()
            {
                self.path = dir.to_string_lossy().into_owned();
                self.load_images(ui.ctx());
            }
            if ui.button("Reload").clicked() {
                self.load_images(ui.ctx());
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.path)
                    .desired_width(f32::INFINITY)
                    .margin(egui::Margin::symmetric(7.0, 5.0)),
            );
        });
        ui.add_space(3.0);
    }

    fn load_images(&mut self, ctx: &egui::Context) {
        self.gallery.load(&self.path, ctx);
        self.selected.clear();
    }

    fn auto_preview(&mut self, ctx: &egui::Context) {
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

    fn show_size_slider(&mut self, ui: &mut egui::Ui) {
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.label("Size");
            ui.add(egui::Slider::new(&mut self.thumb_size, 50.0..=400.0));
            if self.gallery.is_loading() {
                ui.spinner();
            }

            let text = match &self.monitors {
                Ok(monitors) if !monitors.is_empty() => monitors
                    .iter()
                    .enumerate()
                    .map(|(i, m)| format!("#{} {} — {}×{}", i + 1, m.name, m.width, m.height))
                    .collect::<Vec<_>>()
                    .join(", "),
                Ok(_) => "No monitors detected".into(),
                Err(e) => e.clone(),
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(&text);
            });
        });
        ui.add_space(3.0);
    }

    fn show_selection(&mut self, ui: &mut egui::Ui) {
        let Some(entries) = self.gallery.entries() else {
            return;
        };
        if self.selected.is_empty() {
            return;
        }

        if let Some(assignments) = self.show_selection_row(ui, entries) {
            self.apply.start(assignments, ui.ctx());
        }
    }

    fn show_selection_row(
        &self,
        ui: &mut egui::Ui,
        entries: &[ImageEntry],
    ) -> Option<Vec<(PathBuf, Monitor)>> {
        let mut action = None;
        let can_act = matches!(&self.monitors, Ok(m) if m.len() >= 2) && self.selected.len() == 2;
        let busy = self.apply.is_running();

        ui.horizontal(|ui| {
            for (slot, &idx) in self.selected.items().iter().enumerate() {
                let entry = &entries[idx];
                ui.vertical(|ui| {
                    ui.label(format!("#{}", slot + 1));
                    ui.add(
                        egui::Image::new(&entry.texture)
                            .maintain_aspect_ratio(true)
                            .fit_to_exact_size(egui::vec2(120.0, 120.0)),
                    );
                });
            }

            if self.preview.has_texture() || self.preview.is_running() {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label("Wallpaper");
                    if self.preview.is_running() {
                        ui.spinner();
                    } else {
                        self.preview.show_image(ui);
                    }

                    if busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            let log = self.apply.log();
                            if !log.is_empty() {
                                ui.weak(log);
                            }
                        });
                    } else if can_act {
                        let monitors = self.monitors.as_ref().unwrap();
                        let assignments = || {
                            self.selected
                                .items()
                                .iter()
                                .zip(monitors.iter())
                                .map(|(&idx, monitor)| {
                                    let path = PathBuf::from(entries[idx].texture.name());
                                    (path, monitor.clone())
                                })
                                .collect()
                        };
                        if ui.button("Apply").clicked() {
                            action = Some(assignments());
                        }
                    }

                    if let Some(status) = self.apply.status() {
                        match status {
                            Ok(()) => {
                                ui.label("Applied!");
                            }
                            Err(e) => {
                                ui.colored_label(egui::Color32::RED, e);
                            }
                        }
                    }
                });
            }
        });

        action
    }

    fn show_gallery(&mut self, ui: &mut egui::Ui) {
        let loading = self.gallery.is_loading();

        match self.gallery.state() {
            gallery::State::Empty => {}
            gallery::State::Error(e) => {
                ui.colored_label(egui::Color32::RED, e);
            }
            gallery::State::Loaded(entries) if entries.is_empty() && !loading => {
                ui.label("No images found.");
            }
            gallery::State::Loaded(entries) if entries.is_empty() => {}
            gallery::State::Loaded(entries) => {
                let clicked = self.show_image_grid(ui, entries);
                if let Some((i, shift)) = clicked
                    && !loading
                {
                    self.handle_image_click(i, shift);
                }
            }
        }
    }

    fn show_image_grid(&self, ui: &mut egui::Ui, entries: &[ImageEntry]) -> Option<(usize, bool)> {
        let thumb_size = self.thumb_size;
        let mut clicked = None;

        egui::ScrollArea::vertical()
            .max_width(f32::INFINITY)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (i, entry) in entries.iter().enumerate() {
                        let response = ui.add(
                            egui::Image::new(&entry.texture)
                                .maintain_aspect_ratio(true)
                                .fit_to_exact_size(egui::vec2(thumb_size, thumb_size))
                                .sense(egui::Sense::click()),
                        );

                        if let Some(label) = self.selected.badge(i) {
                            paint_selection_badge(ui, response.rect, label);
                        }

                        if response.clicked() {
                            let shift = ui.input(|i| i.modifiers.shift);
                            clicked = Some((i, shift));
                        }

                        response.on_hover_ui(|ui| {
                            show_image_tooltip(ui, entry);
                        });
                    }
                });
            });

        clicked
    }

    fn handle_image_click(&mut self, index: usize, shift: bool) {
        self.apply.clear_status();
        self.selected.click(index, shift);
    }
}

fn paint_selection_badge(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    let center = rect.left_top() + egui::vec2(16.0, 16.0);
    let painter = ui.painter();
    painter.circle_filled(
        center,
        14.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
    );
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(20.0),
        egui::Color32::WHITE,
    );
}

fn show_image_tooltip(ui: &mut egui::Ui, entry: &ImageEntry) {
    let full_path = entry.texture.name();
    let path = Path::new(&full_path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let [w, h] = entry.original_size;
    ui.label(format!("{name}\n{w} × {h}\n\n{full_path}"));
}
