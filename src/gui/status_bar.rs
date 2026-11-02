use crate::gui::top_menu::TopMenuActions;
use crate::model::target::{ConnectionStatus, DataTransportProtocol, TargetDeviceConfig, TargetDeviceRuntime};
use eframe::egui::{Color32, RichText, Ui};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use sysinfo::Disks;

pub struct StatusBarState {
    pub disks: Disks,
    pub last_disk_check: std::time::Instant,
    pub disk_free_gb: f64,
}

impl Default for StatusBarState {
    fn default() -> Self {
        let disks = Disks::new_with_refreshed_list();
        let free_bytes: u64 = disks.iter().map(|d| d.available_space()).sum();
        let disk_free_gb = free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        Self {
            disks,
            last_disk_check: std::time::Instant::now(),
            disk_free_gb,
        }
    }
}

pub fn render_status_bar(
    ui: &mut Ui,
    state: &mut StatusBarState,
    targets_config: &[TargetDeviceConfig],
    targets_runtime: &Arc<RwLock<HashMap<u16, TargetDeviceRuntime>>>,
    total_rx_arc: &Arc<AtomicU64>,
    total_lost_arc: &Arc<AtomicU64>,
    menu_actions: &TopMenuActions,
) {
    // Refresh disk every 10 seconds
    if state.last_disk_check.elapsed().as_secs() > 10 {
        state.disks.refresh();
        let free_bytes: u64 = state.disks.iter().map(|d| d.available_space()).sum();
        state.disk_free_gb = free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        state.last_disk_check = std::time::Instant::now();
    }

    let rx_count = total_rx_arc.load(Ordering::Relaxed);
    let lost_count = total_lost_arc.load(Ordering::Relaxed);
    let total = rx_count + lost_count;
    let rx_rate = if total > 0 {
        (rx_count as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    let (connected_count, total_targets) = {
        let guard = targets_runtime.read().unwrap();
        let mut conn = 0;
        for t in guard.values() {
            if t.status == ConnectionStatus::Normal {
                conn += 1;
            }
        }
        (conn, targets_config.len())
    };

    ui.horizontal(|ui| {
        // 1. Connected devices
        let conn_text = format!("연결장치: {}/{}", connected_count, total_targets.max(1));
        ui.label(RichText::new(conn_text).strong().color(if connected_count > 0 {
            Color32::from_rgb(100, 240, 100)
        } else {
            Color32::from_rgb(200, 200, 200)
        }));

        ui.separator();

        // 2. Total packet reception rate
        let rx_text = format!("총 패킷 수신률: {:.2}% (Rx: {}, Lost: {})", rx_rate, rx_count, lost_count);
        ui.label(RichText::new(rx_text).color(if rx_rate >= 99.0 {
            Color32::from_rgb(100, 240, 100)
        } else if rx_rate >= 95.0 {
            Color32::YELLOW
        } else {
            Color32::RED
        }));

        ui.separator();

        // 3. Transport protocol
        let cur_proto = targets_config.first().map(|t| t.transport).unwrap_or(DataTransportProtocol::Udp);
        let proto_badge = match cur_proto {
            DataTransportProtocol::Udp => "통신모드: UDP/IP (100ms)",
            DataTransportProtocol::Tcp => "통신모드: TCP/IP (MTU초과 대응 프레이밍)",
        };
        ui.label(RichText::new(proto_badge).color(Color32::from_rgb(120, 190, 255)));

        ui.separator();

        // 4. Free disk capacity
        let disk_text = format!("디스크 여유용량: {:.1} GB", state.disk_free_gb);
        ui.label(RichText::new(disk_text).color(Color32::from_rgb(220, 220, 220)));

        ui.separator();

        // 5. Recording Status & Elapsed time
        if menu_actions.recording_active {
            let elapsed_secs = menu_actions
                .recording_start_time
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);
            let hours = elapsed_secs / 3600;
            let mins = (elapsed_secs % 3600) / 60;
            let secs = elapsed_secs % 60;
            let time_str = format!("{:02}:{:02}:{:02}", hours, mins, secs);

            ui.label(
                RichText::new(format!("● 상태: 녹화중 ({})", time_str))
                    .strong()
                    .color(Color32::from_rgb(255, 80, 80)),
            );
        } else {
            ui.label(RichText::new("상태: 대기중 (IDLE)").color(Color32::GRAY));
        }
    });
}
