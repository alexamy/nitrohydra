use std::path::{Path, PathBuf};

use eframe::egui;

use crate::gallery::{self, ImageEntry};
use crate::logic::App;
use crate::monitors::Monitor;

impl App {
    pub(crate) fn show_path_input(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn show_size_slider(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn show_selection(&mut self, ui: &mut egui::Ui) {
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
                });

                ui.vertical(|ui| {
                    // Align with the preview image (skip past the "Wallpaper" label).
                    let label_height =
                        ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
                    ui.add_space(label_height);

                    if busy {
                        ui.spinner();
                        let log = self.apply.log();
                        if !log.is_empty() {
                            ui.weak(log);
                        }
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

    pub(crate) fn show_gallery(&mut self, ui: &mut egui::Ui) {
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
