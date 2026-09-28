mod app;
mod config;
mod finder_comment;
mod i18n;
mod matcher;
mod watcher;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Spine",
        native_options,
        Box::new(|cc| Ok(Box::new(app::SpineApp::new(&cc.egui_ctx)))),
    )
}
