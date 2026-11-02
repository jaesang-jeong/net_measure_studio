use crate::model::target::{DataTransportProtocol, TargetDeviceConfig};
use crate::network::control_client::ControlClient;
use eframe::egui::{self, Color32, RichText, Window};
use std::net::SocketAddr;
use std::time::Duration;

pub struct ConnectionModalState {
    pub is_open: bool,
    pub new_name: String,
    pub new_ip: String,
    pub new_data_port: String,
    pub new_ctrl_port: String,
    pub new_transport: DataTransportProtocol,
    pub ping_result_msg: Option<String>,
}

impl Default for ConnectionModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            new_name: "Target-01".to_string(),
            new_ip: "127.0.0.1".to_string(),
            new_data_port: "8000".to_string(),
            new_ctrl_port: "9000".to_string(),
            new_transport: DataTransportProtocol::Udp,
            ping_result_msg: None,
        }
    }
}

pub fn render_connection_modal(
    ctx: &egui::Context,
    state: &mut ConnectionModalState,
    target_configs: &mut Vec<TargetDeviceConfig>,
) {
    if !state.is_open {
        return;
    }

    Window::new("연결 관리 (Connection Management)")
        .collapsible(false)
        .resizable([true, true])
        .default_size([760.0, 650.0])
        .min_width(700.0)
        .show(ctx, |ui| {
            ui.heading("타겟 장치 및 통신 프로토콜 설정");
            ui.add_space(4.0);

            ui.label(
                RichText::new("※ 실시간 계측 데이터 통신 프로토콜은 MTU 초과 대용량 데이터 시 TCP를 권장합니다.")
                    .color(Color32::from_rgb(100, 180, 255))
                    .small(),
            );
            ui.add_space(6.0);

            let table_height = (ui.available_height() - 220.0).max(180.0);

            // Target List Table
            egui::ScrollArea::vertical()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("target_list_grid")
                        .striped(true)
                        .min_col_width(80.0)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("ID");
                            ui.strong("장치명");
                            ui.strong("IP 주소");
                            ui.strong("데이터 포트");
                            ui.strong("제어 포트");
                            ui.strong("프로토콜");
                            ui.strong("동작");
                            ui.end_row();

                            let mut remove_idx = None;
                            let mut test_idx = None;

                            for (idx, target) in target_configs.iter_mut().enumerate() {
                                ui.label(format!("#{}", target.device_id));
                                ui.label(&target.name);
                                ui.label(target.ip.to_string());
                                ui.label(target.data_port.to_string());
                                ui.label(target.control_port.to_string());

                                // Transport protocol selector (UDP vs TCP)
                                egui::ComboBox::from_id_salt(format!("proto_combo_{}", idx))
                                    .selected_text(match target.transport {
                                        DataTransportProtocol::Udp => "UDP (표준)",
                                        DataTransportProtocol::Tcp => "TCP (대용량/MTU대비)",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut target.transport,
                                            DataTransportProtocol::Udp,
                                            "UDP (표준 100ch)",
                                        );
                                        ui.selectable_value(
                                            &mut target.transport,
                                            DataTransportProtocol::Tcp,
                                            "TCP (대용량/MTU초과대비)",
                                        );
                                    });

                                ui.horizontal(|ui| {
                                    if ui.button("Ping").clicked() {
                                        test_idx = Some(idx);
                                    }
                                    if ui.button("삭제").clicked() {
                                        remove_idx = Some(idx);
                                    }
                                });
                                ui.end_row();
                            }

                            if let Some(idx) = remove_idx {
                                target_configs.remove(idx);
                            }

                            if let Some(idx) = test_idx {
                                let t = &target_configs[idx];
                                let addr_str = format!("{}:{}", t.ip, t.control_port);
                                if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                                    match ControlClient::send_command(
                                        addr,
                                        crate::protocol::commands::ControlCommand::Ping,
                                        Duration::from_millis(500),
                                    ) {
                                        Ok(resp) => {
                                            state.ping_result_msg = Some(format!(
                                                "[{}] Ping 성공! 응답: {:?}",
                                                t.name, resp
                                            ));
                                        }
                                        Err(e) => {
                                            state.ping_result_msg = Some(format!(
                                                "[{}] Ping 실패: {}",
                                                t.name, e
                                            ));
                                        }
                                    }
                                }
                            }
                        });
                });

            if let Some(ref msg) = state.ping_result_msg {
                ui.add_space(4.0);
                ui.label(RichText::new(msg).color(Color32::from_rgb(100, 230, 100)));
            }

            ui.separator();
            ui.heading("새 타겟 추가");
            ui.add_space(4.0);

            // Row 1: 장치명 & IP 주소 (IP 주소 입력칸을 220px로 넓고 시원하게 배치)
            ui.horizontal(|ui| {
                ui.label("장치명:");
                ui.add_sized([160.0, 22.0], egui::TextEdit::singleline(&mut state.new_name));

                ui.add_space(20.0);
                ui.label("IP 주소:");
                ui.add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut state.new_ip));
            });

            ui.add_space(6.0);

            // Row 2: 데이터 포트 & 제어 포트 & 프로토콜 & 등록 버튼
            ui.horizontal(|ui| {
                ui.label("데이터 포트:");
                ui.add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut state.new_data_port));

                ui.add_space(14.0);
                ui.label("제어 포트:");
                ui.add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut state.new_ctrl_port));

                ui.add_space(14.0);
                ui.label("프로토콜:");
                egui::ComboBox::from_id_salt("new_proto_combo")
                    .selected_text(match state.new_transport {
                        DataTransportProtocol::Udp => "UDP (표준)",
                        DataTransportProtocol::Tcp => "TCP (대용량 MTU대비)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.new_transport,
                            DataTransportProtocol::Udp,
                            "UDP (표준)",
                        );
                        ui.selectable_value(
                            &mut state.new_transport,
                            DataTransportProtocol::Tcp,
                            "TCP (대용량 MTU대비)",
                        );
                    });

                ui.add_space(10.0);
                if ui.button("➕ 장치 등록").clicked() {
                    if let (Ok(ip), Ok(data_p), Ok(ctrl_p)) = (
                        state.new_ip.parse(),
                        state.new_data_port.parse(),
                        state.new_ctrl_port.parse(),
                    ) {
                        let dev_id = (target_configs.len() + 1) as u16;
                        target_configs.push(TargetDeviceConfig {
                            device_id: dev_id,
                            name: state.new_name.clone(),
                            ip,
                            data_port: data_p,
                            control_port: ctrl_p,
                            sftp_port: 22,
                            transport: state.new_transport,
                            enabled: true,
                        });
                    }
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("닫기").clicked() {
                    state.is_open = false;
                }
            });
        });
}
