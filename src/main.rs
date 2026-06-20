// Hide the console window on Windows GUI (release) builds.
// `cargo run` / debug builds keep the console so you still see panics/logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod gui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([660.0, 540.0])
            .with_min_inner_size([440.0, 320.0])
            .with_title("ytdl — YouTube Downloader"),
        ..Default::default()
    };

    eframe::run_native(
        "ytdl",
        options,
        Box::new(|_cc| Ok(Box::new(gui::YtdlApp::new()))),
    )
}
