use crate::gui::modals::config_modal::ConfigModalState;
use crate::gui::modals::connection_modal::ConnectionModalState;
use crate::gui::modals::sftp_modal::SftpModalState;
use crate::model::target::{DataTransportProtocol, TargetDeviceConfig};
use crate::network::control_client::ControlClient;
use crate::simulator::target_sim::TargetSimulator;
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
    targets: &[TargetDeviceConfig],
    simulator: &TargetSimulator,
    on_save_layout_clicked: &mut bool,
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
                            100,
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

        ui.separator();

        if ui.button("📥 SFTP파일다운로드").clicked() {
            sftp_modal.is_open = true;
        }

        if ui.button("💾 레이아웃 저장").clicked() {
            *on_save_layout_clicked = true;
        }

        ui.separator();

        // Target Simulator Quick Toggle
        let sim_running = simulator.is_running();
        let sim_btn_text = if sim_running {
            RichText::new("🟢 내장 시뮬레이터: 동작 중").color(Color32::GREEN)
        } else {
            RichText::new("⚪ 내장 시뮬레이터: 정지").color(Color32::LIGHT_GRAY)
        };
        if ui.button(sim_btn_text).clicked() {
            if sim_running {
                simulator.stop();
            } else {
                simulator.start();
            }
        }

        // Quick Simulator Transport Selector (UDP vs TCP)
        ui.label("통신모드:");
        let mut cur_transport = targets.first().map(|t| t.transport).unwrap_or(DataTransportProtocol::Udp);
        let proto_text = match cur_transport {
            DataTransportProtocol::Udp => "UDP (MTU내)",
            DataTransportProtocol::Tcp => "TCP (대용량/MTU초과)",
        };
        egui::ComboBox::from_id_salt("top_transport_combo")
            .selected_text(proto_text)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(cur_transport == DataTransportProtocol::Udp, "UDP (MTU내)")
                    .clicked()
                {
                    cur_transport = DataTransportProtocol::Udp;
                    simulator.set_transport(DataTransportProtocol::Udp);
                }
                if ui
                    .selectable_label(cur_transport == DataTransportProtocol::Tcp, "TCP (대용량/MTU초과)")
                    .clicked()
                {
                    cur_transport = DataTransportProtocol::Tcp;
                    simulator.set_transport(DataTransportProtocol::Tcp);
                }
            });
    });
}
