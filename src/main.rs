// Pas de console noire en build release sous Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod anim;
mod app;
mod bitmap;
mod celebrate;
mod model;
mod reel;
mod theme;
mod veil;
mod vhs;
mod wheel;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Roulette Sport")
            .with_inner_size([1040.0, 940.0])
            .with_min_inner_size([560.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Roulette Sport",
        options,
        Box::new(|cc| Ok(Box::new(app::RouletteApp::new(cc)))),
    )
}
