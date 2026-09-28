mod app;
mod config;
mod finder_comment;
mod matcher;
mod watcher;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Finder Tagger",
        native_options,
        Box::new(|_cc| Ok(Box::new(app::TaggerApp::new()))),
    )
}
