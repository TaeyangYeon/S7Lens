mod model;
mod plc;
mod state;

use std::sync::{Arc, Mutex};

use state::SharedState;

fn main() {
    let shared = Arc::new(Mutex::new(SharedState::new()));
    // Poller starts with polling_active = false, so it idles safely until the UI enables it.
    let _poller = plc::poller::spawn_poller(Arc::clone(&shared));

    println!("Siemens PLC Monitor - Step 2 scaffold OK");
}
