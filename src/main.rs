use eframe::egui;
use std::sync::mpsc;

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
                path: "/home/alex/Pictures/nitrohydra".to_string(),
                ..App::default()
            }))
        }),
    )
}

struct App {
    path: String,
    state: State,
    thumb_size: f32,
    receiver: Option<mpsc::Receiver<Result<(String, egui::ColorImage), String>>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            path: String::new(),
            state: State::default(),
            thumb_size: 150.0,
            receiver: None,
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
        if let Some(rx) = &self.receiver {
            loop {
                match rx.try_recv() {
                    Ok(Ok((name, color_image))) => {
                        let texture = ctx.load_texture(name, color_image, Default::default());
                        match &mut self.state {
                            State::Images(v) => v.push(texture),
                            _ => self.state = State::Images(vec![texture]),
                        }
                    }
                    Ok(Err(e)) => {
                        self.state = State::Error(e);
                        self.receiver = None;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if matches!(self.state, State::Loading) {
                            self.state = State::Images(vec![]);
                        }
                        self.receiver = None;
                        break;
                    }
                }
            }
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
    fn show_path_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Directory path:");
        ui.horizontal(|ui| {
            let clicked = ui.button("Read").clicked();
            ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(f32::INFINITY));
            if clicked {
                let path = self.path.clone();
                let ctx = ui.ctx().clone();
                let (tx, rx) = mpsc::channel();
                self.receiver = Some(rx);
                self.state = State::Loading;
                std::thread::spawn(move || {
                    decode_images(&path, &tx, &ctx);
                });
            }
        });
    }

    fn show_size_slider(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.thumb_size, 50.0..=400.0).text("Size"));
    }

    fn show_gallery(&self, ui: &mut egui::Ui) {
        let thumb_size = self.thumb_size;
        let loading = self.receiver.is_some();

        match &self.state {
            State::Empty => {}
            State::Loading => {
                ui.spinner();
            }
            State::Error(e) => {
                ui.colored_label(egui::Color32::RED, e);
            }
            State::Images(textures) if textures.is_empty() => {
                if loading {
                    ui.spinner();
                } else {
                    ui.label("No images found.");
                }
            }
            State::Images(textures) => {
                if loading {
                    ui.spinner();
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for texture in textures {
                            ui.add(
                                egui::Image::new(texture)
                                    .maintain_aspect_ratio(true)
                                    .fit_to_exact_size(egui::vec2(thumb_size, thumb_size)),
                            );
                        }
                    });
                });
            }
        }
    }
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

fn decode_images(
    path: &str,
    tx: &mpsc::Sender<Result<(String, egui::ColorImage), String>>,
    ctx: &egui::Context,
) {
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            let _ = tx.send(Err(format!("Error: {e}")));
            ctx.request_repaint();
            return;
        }
    };

    let paths: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .collect();

    for path in paths {
        let img = match image::open(&path) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Failed to load {}: {e}", path.display());
                continue;
            }
        };
        let rgba = img.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let name = path.to_string_lossy().into_owned();
        if tx.send(Ok((name, color_image))).is_err() {
            break; // receiver dropped — user clicked Read again
        }
        ctx.request_repaint();
    }
}
