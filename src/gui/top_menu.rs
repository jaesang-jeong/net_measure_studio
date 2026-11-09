use crate::gui::modals::config_modal::ConfigModalState;
use crate::gui::modals::connection_modal::ConnectionModalState;
use crate::gui::modals::sftp_modal::SftpModalState;
use crate::gui::modals::system_modal::SystemModalState;
use crate::model::target::TargetDeviceConfig;
use crate::network::control_client::ControlClient;
use eframe::egui::{self, Color32, RichText, Ui};
use std::net::SocketAddr;
use std::time::Instant;

pub struct TopMenuActions {
    pub recording_active: bool,
    pub recording_start_time: Option<Instant>,
    pub recording_session_id: Option<String>,
}

impl Default for TopMenuActions {
    fn default() -> Self {
        Self {
            recording_active: false,
            recording_start_time: None,
            recording_session_id: None,
        }
    }
}

pub fn render_top_menu(
    ui: &mut Ui,
    actions: &mut TopMenuActions,
    conn_modal: &mut ConnectionModalState,
    cfg_modal: &mut ConfigModalState,
    sftp_modal: &mut SftpModalState,
    sys_modal: &mut SystemModalState,
    targets: &[TargetDeviceConfig],
) {
    ui.horizontal(|ui| {
        ui.heading(
            RichText::new("⚡ Network Measurement Studio")
                .strong()
                .color(Color32::from_rgb(0, 200, 255)),
        );
        ui.separator();

        if ui.button("📡 연결관리").clicked() {
            conn_modal.is_open = true;
        }

        if ui.button("⚙ 채널설정/다운로드").clicked() {
            cfg_modal.is_open = true;
        }

        if ui.button("📥 SFTP파일다운로드").clicked() {
            sftp_modal.is_open = true;
        }

        if ui.button("🛠 시스템").on_hover_text("내장 시뮬레이터, 통신 프로토콜(UDP/TCP), CRC 검증 및 시스템 설정을 관리합니다.").clicked() {
            sys_modal.is_open = true;
        }

        ui.separator();

        // Batch Recording Start / Stop
        if !actions.recording_active {
            let start_btn = egui::Button::new(
                RichText::new("⏺ 일괄저장시작").color(Color32::from_rgb(255, 100, 100)),
            );
            if ui.add(start_btn).clicked() {
                let session = format!("SESS_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"));
                actions.recording_active = true;
                actions.recording_start_time = Some(Instant::now());
                actions.recording_session_id = Some(session.clone());

                // Broadcast to all targets
                for target in targets {
                    let addr_str = format!("{}:{}", target.ip, target.control_port);
                    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                        let _ = ControlClient::start_recording(
                            addr,
                            session.clone(),
                            "MEAS_DATA".to_string(),
                            target.packet_interval_ms,
                        );
                    }
                }
            }
        } else {
            let stop_btn = egui::Button::new(
                RichText::new("⏹ 일괄저장종료").color(Color32::from_rgb(255, 200, 100)),
            );
            if ui.add(stop_btn).clicked() {
                actions.recording_active = false;
                if let Some(session) = actions.recording_session_id.take() {
                    for target in targets {
                        let addr_str = format!("{}:{}", target.ip, target.control_port);
                        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                            let _ = ControlClient::stop_recording(addr, session.clone());
                        }
                    }
                }
                actions.recording_start_time = None;
            }
        }
    });
}
