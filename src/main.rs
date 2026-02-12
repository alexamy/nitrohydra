mod apply_job;
mod cache;
mod gallery;
mod loader;
mod logic;
mod monitors;
mod preview;
mod selection;
mod ui;
mod wallpaper;

use eframe::egui;
use logic::App;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        1 => run_gui(),
        3 => logic::run_cli(&args[1], &args[2]),
        _ => {
            logic::show_help();
            let is_help = args.get(1).is_some_and(|a| a == "--help" || a == "-h");
            std::process::exit(if is_help { 0 } else { 1 });
        }
    }
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
                .resizable(true)
                .default_height(200.0)
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
