use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "nitrohydra",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
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
}

impl Default for App {
    fn default() -> Self {
        Self {
            path: String::new(),
            state: State::default(),
            thumb_size: 150.0,
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Empty,
    Error(String),
    Images(Vec<PathBuf>),
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Directory path:");
            ui.horizontal(|ui| {
                let clicked = ui.button("Read").clicked();
                ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(f32::INFINITY));
                if clicked {
                    self.state = match read_images(&self.path) {
                        Ok(images) => State::Images(images),
                        Err(e) => State::Error(e),
                    };
                }
            });

            ui.add(egui::Slider::new(&mut self.thumb_size, 50.0..=400.0).text("Size"));

            ui.separator();

            let thumb_size = self.thumb_size;
            match &self.state {
                State::Empty => {}
                State::Error(e) => {
                    ui.colored_label(egui::Color32::RED, e);
                }
                State::Images(images) if images.is_empty() => {
                    ui.label("No images found.");
                }
                State::Images(images) => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for path in images {
                                let uri = format!("file://{}", path.display());
                                ui.add(egui::Image::new(uri).max_size(egui::vec2(thumb_size, thumb_size)));
                            }
                        });
                    });
                }
            }
        });
    }
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

fn read_images(path: &str) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(path).map_err(|e| format!("Error: {e}"))?;
    Ok(entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .collect())
}
