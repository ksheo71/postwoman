//! Postwoman — Rust + egui 기반 API 테스트 데스크탑 앱.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod http;
mod persist;
mod theme;

use app::PostwomanApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 720.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("Postwoman"),
        ..Default::default()
    };

    eframe::run_native(
        "Postwoman",
        options,
        Box::new(|cc| Ok(Box::new(PostwomanApp::new(cc)))),
    )
}
