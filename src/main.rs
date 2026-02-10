mod cache;
mod loader;

use core::f32;

use eframe::egui;
use loader::{ImageLoader, Poll};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "nitrohydra",
        options,
        Box::new(|_cc| {
            Ok(Box::new(App {
                path: "/home/alex/Dropbox/Wallpapers".to_string(),
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
}

impl Default for App {
    fn default() -> Self {
        Self {
            path: String::new(),
            state: State::default(),
            thumb_size: 150.0,
            loader: None,
            selected: Vec::new(),
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Empty,
    Loading,
    Error(String),
    Images(Vec<egui::TextureHandle>),
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_loader(ctx);

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
                    Poll::Image(name, img) => {
                        let texture = ctx.load_texture(name, img, Default::default());
                        match &mut self.state {
                            State::Images(v) => v.push(texture),
                            _ => self.state = State::Images(vec![texture]),
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
        });
    }

    fn show_gallery(&mut self, ui: &mut egui::Ui) {
        let thumb_size = self.thumb_size;
        let loading = self.loader.is_some();
        let mut clicked_index: Option<usize> = None;

        match &self.state {
            State::Empty => {}
            State::Loading => {}
            State::Error(e) => {
                ui.colored_label(egui::Color32::RED, e);
            }
            State::Images(textures) if textures.is_empty() && !loading => {
                ui.label("No images found.");
            }
            State::Images(textures) if textures.is_empty() => {}
            State::Images(textures) => {
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (i, texture) in textures.iter().enumerate() {
                                let response = ui.add(
                                    egui::Image::new(texture)
                                        .maintain_aspect_ratio(true)
                                        .fit_to_exact_size(egui::vec2(thumb_size, thumb_size))
                                        .sense(egui::Sense::click()),
                                );

                                if let Some(sel_pos) =
                                    self.selected.iter().position(|&idx| idx == i)
                                {
                                    let num = sel_pos + 1;
                                    let painter = ui.painter();
                                    let center =
                                        response.rect.left_top() + egui::vec2(16.0, 16.0);
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

                                if response.clicked() {
                                    clicked_index = Some(i);
                                }
                            }
                        });
                    });
            }
        }

        if let Some(i) = clicked_index {
            self.handle_image_click(i);
        }
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
