#![windows_subsystem = "windows"]

mod gui;
mod model;
mod network;
mod processing;
mod protocol;
mod simulator;

use gui::MeasurementStudioApp;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Headless SSH_ASKPASS handler for OpenSSH sftp/ssh password authentication
    if let Ok(pass) = std::env::var("NET_MEASURE_SSH_PASS") {
        if args.len() > 1 || args.contains(&"--ssh-askpass".to_string()) {
            use std::io::Write;
            let _ = writeln!(std::io::stdout(), "{}", pass);
            let _ = std::io::stdout().flush();
            std::process::exit(0);
        }
    }

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

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([1000.0, 600.0])
        .with_title("Network Measurement Studio (High-Performance Rust Edition)");

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    println!("============================================================");
    println!("  Network Measurement Studio (Host Monitoring Application)");
    println!("  - Real-time Stream: UDP (8000) / TCP (8001, MTU Framing)");
    println!("  - Target Control  : TCP (9000)");
    println!("  - Built-in Target Simulator: DISABLED (Default Stopped)");
    println!("============================================================");

    eframe::run_native(
        "Network Measurement Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(MeasurementStudioApp::new(cc)))),
    )
}

/// Parses the embedded `nxs.ICO` file and converts the highest resolution 32-bit icon into `egui::IconData` (RGBA).
fn load_app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../nxs.ICO");
    if bytes.len() < 6 || u16::from_le_bytes([bytes[2], bytes[3]]) != 1 {
        return None;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut best_entry = None;
    let mut best_dim = 0u32;
    for i in 0..count {
        let entry_offset = 6 + i * 16;
        if entry_offset + 16 > bytes.len() {
            break;
        }
        let mut w = bytes[entry_offset] as u32;
        let mut h = bytes[entry_offset + 1] as u32;
        if w == 0 {
            w = 256;
        }
        if h == 0 {
            h = 256;
        }
        let bpp = u16::from_le_bytes([bytes[entry_offset + 6], bytes[entry_offset + 7]]);
        if bpp == 32 && w >= best_dim {
            best_dim = w;
            best_entry = Some((w, h, entry_offset));
        }
    }
    let (w, h, entry_offset) = best_entry?;
    let img_size = u32::from_le_bytes([
        bytes[entry_offset + 8],
        bytes[entry_offset + 9],
        bytes[entry_offset + 10],
        bytes[entry_offset + 11],
    ]) as usize;
    let img_offset = u32::from_le_bytes([
        bytes[entry_offset + 12],
        bytes[entry_offset + 13],
        bytes[entry_offset + 14],
        bytes[entry_offset + 15],
    ]) as usize;

    if img_offset + img_size > bytes.len() || img_size < 40 {
        return None;
    }

    let dib_header_size = u32::from_le_bytes([
        bytes[img_offset],
        bytes[img_offset + 1],
        bytes[img_offset + 2],
        bytes[img_offset + 3],
    ]) as usize;

    let pixel_start = img_offset + dib_header_size;
    let row_stride = (w * 4) as usize;
    let total_pixels = (w * h) as usize;
    if pixel_start + total_pixels * 4 > bytes.len() {
        return None;
    }

    let mut rgba = vec![0u8; total_pixels * 4];
    for y in 0..h {
        let src_row = (h - 1 - y) as usize;
        let src_offset = pixel_start + src_row * row_stride;
        let dst_offset = (y as usize) * row_stride;
        for x in 0..(w as usize) {
            let b = bytes[src_offset + x * 4];
            let g = bytes[src_offset + x * 4 + 1];
            let r = bytes[src_offset + x * 4 + 2];
            let a = bytes[src_offset + x * 4 + 3];
            rgba[dst_offset + x * 4] = r;
            rgba[dst_offset + x * 4 + 1] = g;
            rgba[dst_offset + x * 4 + 2] = b;
            rgba[dst_offset + x * 4 + 3] = a;
        }
    }

    Some(egui::IconData {
        rgba,
        width: w,
        height: h,
    })
}
