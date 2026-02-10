use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "nitrohydra",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Default)]
struct App {
    path: String,
    output: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Directory path:");
            ui.horizontal(|ui| {
                let clicked = ui.button("Read").clicked();
                ui.add(egui::TextEdit::singleline(&mut self.path).desired_width(f32::INFINITY));
                if clicked {
                    self.output = read_dir(&self.path);
                }
            });
            ui.label("Files:");
            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.output),
            );
        });
    }
}

fn read_dir(path: &str) -> String {
    match std::fs::read_dir(path) {
        Ok(entries) => entries
            .map(|e| match e {
                Ok(entry) => entry.file_name().to_string_lossy().into_owned(),
                Err(e) => format!("[error: {e}]"),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Err(e) => format!("Error: {e}"),
    }
}
