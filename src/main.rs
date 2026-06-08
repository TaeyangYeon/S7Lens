mod app;
mod config;
mod export;
mod model;
mod plc;
mod state;

use std::sync::{Arc, Mutex};

use app::PlcMonitorApp;
use state::SharedState;

fn main() {
    let shared = Arc::new(Mutex::new(SharedState::new()));
    let _poller = plc::poller::spawn_poller(Arc::clone(&shared));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Siemens PLC Monitor",
        options,
        Box::new(|cc| Ok(Box::new(PlcMonitorApp::new(cc, shared)))),
    )
    .expect("eframe failed to start");
}
