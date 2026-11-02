mod gui;
mod model;
mod network;
mod processing;
mod protocol;
mod simulator;

use gui::MeasurementStudioApp;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Headless simulator CLI mode (optional)
    if args.contains(&"--simulator-only".to_string()) {
        println!("Starting standalone Target Simulator (UDP: 8000, TCP: 8001, Control: 9000)...");
        let sim_cfg = simulator::TargetSimulatorConfig::default();
        let sim = simulator::TargetSimulator::new(sim_cfg);
        sim.start();
        println!("Target simulator running. Press Ctrl+C to terminate.");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_title("Network Measurement Studio (High-Performance Rust Edition)"),
        ..Default::default()
    };

    println!("============================================================");
    println!("  Network Measurement Studio (Host Monitoring Application)");
    println!("  - Real-time Stream: UDP (8000) / TCP (8001, MTU Framing)");
    println!("  - Target Control  : TCP (9000)");
    println!("  - Built-in Target Simulator: ENABLED");
    println!("============================================================");

    eframe::run_native(
        "Network Measurement Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(MeasurementStudioApp::new(cc)))),
    )
}
