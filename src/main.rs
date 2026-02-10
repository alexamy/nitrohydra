mod cache;
mod loader;
mod monitors;
mod wallpaper;

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use eframe::egui;
use loader::{ImageLoader, Poll};
use monitors::Monitor;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "nitrohydra",
        options,
        Box::new(|cc| {
            let path = "/home/alex/Dropbox/Wallpapers".to_string();
            let loader = ImageLoader::start(path.clone(), cc.egui_ctx.clone());
            let monitors = monitors::detect().unwrap_or_default();
            Ok(Box::new(App {
                path,
                state: State::Loading,
                loader: Some(loader),
                monitors,
                ..App::default()
            }))
        }),
    )
}

struct App {
    path: String,
    state: State,
    thumb_size: f32,
    loader: Option<ImageLoader>,
    selected: Vec<usize>,
    monitors: Vec<Monitor>,
    apply_rx: Option<mpsc::Receiver<Result<(), String>>>,
    apply_status: Option<Result<(), String>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            path: String::new(),
            state: State::default(),
            thumb_size: 150.0,
            loader: None,
            selected: Vec::new(),
            monitors: Vec::new(),
            apply_rx: None,
            apply_status: None,
        }
    }
}

struct ImageEntry {
    texture: egui::TextureHandle,
    original_size: [u32; 2],
}

#[derive(Default)]
enum State {
    #[default]
    Empty,
    Loading,
    Error(String),
    Images(Vec<ImageEntry>),
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_loader(ctx);
        self.poll_apply();

        egui::TopBottomPanel::bottom("selection_panel")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(8.0, 12.0)),
            )
            .show_animated(ctx, !self.selected.is_empty(), |ui| self.show_selection(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_path_input(ui);
            self.show_size_slider(ui);
            ui.separator();
            self.show_gallery(ui);
        });
    }
}

impl App {
    fn poll_loader(&mut self, ctx: &egui::Context) {
        if let Some(loader) = &self.loader {
            loop {
                match loader.poll() {
                    Poll::Image(name, img, original_size) => {
                        let texture = ctx.load_texture(name, img, Default::default());
                        let entry = ImageEntry {
                            texture,
                            original_size,
                        };
                        match &mut self.state {
                            State::Images(v) => v.push(entry),
                            _ => self.state = State::Images(vec![entry]),
                        }
                    }
                    Poll::Error(e) => {
                        self.state = State::Error(e);
                        self.loader = None;
                        break;
                    }
                    Poll::Pending => break,
                    Poll::Done => {
                        if !matches!(&self.state, State::Images(v) if !v.is_empty()) {
                            self.state = State::Images(vec![]);
                        }
                        self.loader = None;
                        break;
                    }
                }
            }
        }
    }

    fn show_path_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Directory path:");
        ui.horizontal(|ui| {
            let clicked = ui.button("Read").clicked();
            ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(f32::INFINITY));
            if clicked {
                self.loader = Some(ImageLoader::start(self.path.clone(), ui.ctx().clone()));
                self.state = State::Loading;
                self.selected.clear();
            }
        });
    }

    fn show_size_slider(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Size");
            ui.add(egui::Slider::new(&mut self.thumb_size, 50.0..=400.0));
            if self.loader.is_some() {
                ui.spinner();
            }

            if !self.monitors.is_empty() {
                let text: String = self
                    .monitors
                    .iter()
                    .enumerate()
                    .map(|(i, m)| format!("#{} {} — {}×{}", i + 1, m.name, m.width, m.height))
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(&text);
                });
            }
        });
    }

    fn poll_apply(&mut self) {
        let Some(rx) = &self.apply_rx else { return };
        match rx.try_recv() {
            Ok(result) => {
                self.apply_status = Some(result);
                self.apply_rx = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.apply_status = Some(Err("apply thread crashed".into()));
                self.apply_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn start_apply(&mut self, assignments: Vec<(PathBuf, Monitor)>, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = wallpaper::apply(&assignments);
            let _ = tx.send(result);
            ctx.request_repaint();
        });

        self.apply_rx = Some(rx);
        self.apply_status = None;
    }

    fn show_selection(&mut self, ui: &mut egui::Ui) {
        let State::Images(entries) = &self.state else {
            return;
        };
        if self.selected.is_empty() {
            return;
        }

        if let Some(assignments) = self.show_selection_row(ui, entries) {
            let ctx = ui.ctx().clone();
            self.start_apply(assignments, &ctx);
        }
    }

    fn show_selection_row(
        &self,
        ui: &mut egui::Ui,
        entries: &[ImageEntry],
    ) -> Option<Vec<(PathBuf, Monitor)>> {
        let mut assignments = None;

        ui.horizontal(|ui| {
            self.show_selection_previews(ui, entries);
            assignments = self.show_apply_button(ui, entries);
        });

        assignments
    }

    fn show_selection_previews(&self, ui: &mut egui::Ui, entries: &[ImageEntry]) {
        for (slot, &idx) in self.selected.iter().enumerate() {
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
    ) -> Option<Vec<(PathBuf, Monitor)>> {
        if self.selected.len() != 2 || self.monitors.len() < 2 {
            return None;
        }

        let mut assignments = None;

        ui.vertical(|ui| {
            ui.add_space(16.0);
            if self.apply_rx.is_some() {
                ui.spinner();
            } else if ui.button("Apply").clicked() {
                assignments = Some(
                    self.selected
                        .iter()
                        .zip(self.monitors.iter())
                        .map(|(&idx, monitor)| {
                            let path = PathBuf::from(entries[idx].texture.name());
                            (path, monitor.clone())
                        })
                        .collect(),
                );
            }

            if let Some(status) = &self.apply_status {
                match status {
                    Ok(()) => { ui.label("Applied!"); }
                    Err(e) => { ui.colored_label(egui::Color32::RED, e); }
                }
            }
        });

        assignments
    }

    fn show_gallery(&mut self, ui: &mut egui::Ui) {
        let loading = self.loader.is_some();

        match &self.state {
            State::Empty | State::Loading => {}
            State::Error(e) => {
                ui.colored_label(egui::Color32::RED, e);
            }
            State::Images(entries) if entries.is_empty() && !loading => {
                ui.label("No images found.");
            }
            State::Images(entries) if entries.is_empty() => {}
            State::Images(entries) => {
                let clicked = self.show_image_grid(ui, entries);
                if let Some(i) = clicked {
                    self.handle_image_click(i);
                }
            }
        }
    }

    fn show_image_grid(&self, ui: &mut egui::Ui, entries: &[ImageEntry]) -> Option<usize> {
        let thumb_size = self.thumb_size;
        let mut clicked_index = None;

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

                        if let Some(pos) = self.selected.iter().position(|&idx| idx == i) {
                            paint_selection_badge(ui, response.rect, pos + 1);
                        }

                        if response.clicked() {
                            clicked_index = Some(i);
                        }

                        response.on_hover_ui(|ui| {
                            show_image_tooltip(ui, entry);
                        });
                    }
                });
            });

        clicked_index
    }

    fn handle_image_click(&mut self, index: usize) {
        if let Some(sel_pos) = self.selected.iter().position(|&idx| idx == index) {
            // Already selected — deselect it; if it was #1, #2 shifts down
            self.selected.remove(sel_pos);
        } else if self.selected.len() < 2 {
            // Room for another selection
            self.selected.push(index);
        } else {
            // Both slots full — cycle: start fresh with this as #1
            self.selected.clear();
            self.selected.push(index);
        }
    }
}

fn paint_selection_badge(ui: &egui::Ui, rect: egui::Rect, num: usize) {
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
        num.to_string(),
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
