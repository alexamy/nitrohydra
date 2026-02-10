use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

pub enum Poll {
    Image(String, egui::ColorImage),
    Error(String),
    Pending,
    Done,
}

pub struct ImageLoader {
    rx: mpsc::Receiver<Result<(String, egui::ColorImage), String>>,
}

impl ImageLoader {
    pub fn start(path: String, ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            decode(&path, &tx, &ctx);
        });
        Self { rx }
    }

    pub fn poll(&self) -> Poll {
        match self.rx.try_recv() {
            Ok(Ok((name, img))) => Poll::Image(name, img),
            Ok(Err(e)) => Poll::Error(e),
            Err(mpsc::TryRecvError::Empty) => Poll::Pending,
            Err(mpsc::TryRecvError::Disconnected) => Poll::Done,
        }
    }
}

fn decode(
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

    let paths: Vec<PathBuf> = entries
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
            break;
        }
        ctx.request_repaint();
    }
}
