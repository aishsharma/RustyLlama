mod provider;
mod ui;
mod db;
mod config;
mod models;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RustyLlama",
        options,
        Box::new(|cc| Ok(Box::new(ui::RustyLlamaApp::new(cc)))),
    )
}