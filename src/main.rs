mod apply_job;
mod cache;
mod gallery;
mod loader;
mod monitors;
mod preview;
mod selection;
mod wallpaper;

use std::path::{Path, PathBuf};

use eframe::egui;
use apply_job::ApplyJob;
use gallery::{Gallery, ImageEntry};
use monitors::Monitor;
use preview::PreviewJob;
use selection::Selection;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "nitrohydra",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

enum SelectionAction {
    Apply(Vec<(PathBuf, Monitor)>),
    Preview(Vec<(PathBuf, Monitor)>),
}

struct App {
    path: String,
    gallery: Gallery,
    thumb_size: f32,
    selected: Selection,
    monitors: Result<Vec<Monitor>, String>,
    apply: ApplyJob,
    preview: PreviewJob,
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

        egui::TopBottomPanel::bottom("selection_panel")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(8.0, 12.0)),
            )
            .show_animated(ctx, !self.selected.is_empty(), |ui| self.show_selection(ui));

        self.preview.show(ctx);

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

        let mut app = Self {
            path: path.clone(),
            monitors: monitors::detect(),
            ..Self::default()
        };
        app.gallery.load(&path, &cc.egui_ctx);
        app
    }

    fn show_path_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Directory path:");
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
            ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(f32::INFINITY));
        });
    }

    fn load_images(&mut self, ctx: &egui::Context) {
        self.gallery.load(&self.path, ctx);
        self.selected.clear();
    }

    fn show_size_slider(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Size");
            ui.add(egui::Slider::new(&mut self.thumb_size, 50.0..=400.0));
            if self.gallery.is_loading() {
                ui.spinner();
            }

            let text = match &self.monitors {
                Ok(monitors) if !monitors.is_empty() => {
                    monitors.iter()
                        .enumerate()
                        .map(|(i, m)| format!("#{} {} — {}×{}", i + 1, m.name, m.width, m.height))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
                Ok(_) => "No monitors detected".into(),
                Err(e) => e.clone(),
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(&text);
            });
        });
    }

    fn show_selection(&mut self, ui: &mut egui::Ui) {
        let Some(entries) = self.gallery.entries() else {
            return;
        };
        if self.selected.is_empty() {
            return;
        }

        match self.show_selection_row(ui, entries) {
            Some(SelectionAction::Apply(assignments)) => {
                self.apply.start(assignments, ui.ctx());
            }
            Some(SelectionAction::Preview(assignments)) => {
                self.preview.start(assignments, ui.ctx());
            }
            None => {}
        }
    }

    fn show_selection_row(
        &self,
        ui: &mut egui::Ui,
        entries: &[ImageEntry],
    ) -> Option<SelectionAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            self.show_selection_previews(ui, entries);
            action = self.show_apply_button(ui, entries);
        });

        action
    }

    fn show_selection_previews(&self, ui: &mut egui::Ui, entries: &[ImageEntry]) {
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
    }

    fn show_apply_button(
        &self,
        ui: &mut egui::Ui,
        entries: &[ImageEntry],
    ) -> Option<SelectionAction> {
        let Ok(monitors) = &self.monitors else { return None };
        if self.selected.len() != 2 || monitors.len() < 2 {
            return None;
        }

        let mut action = None;
        let busy = self.apply.is_running() || self.preview.is_running();

        ui.vertical(|ui| {
            ui.add_space(16.0);
            if busy {
                ui.spinner();
                let log = self.apply.log();
                if !log.is_empty() {
                    ui.weak(log);
                }
            } else {
                ui.horizontal(|ui| {
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

                    if ui.button("Preview").clicked() {
                        action = Some(SelectionAction::Preview(assignments()));
                    }
                    if ui.button("Apply").clicked() {
                        action = Some(SelectionAction::Apply(assignments()));
                    }
                });
            }

            if let Some(status) = self.apply.status() {
                match status {
                    Ok(()) => { ui.label("Applied!"); }
                    Err(e) => { ui.colored_label(egui::Color32::RED, e); }
                }
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
                if let Some((i, shift)) = clicked && !loading {
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
