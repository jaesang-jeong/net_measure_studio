use crate::model::target::{ConnectionStatus, DataTransportProtocol, TargetDeviceConfig, TargetDeviceRuntime};
use crate::network::control_client::ControlClient;
use eframe::egui::{self, Color32, RichText, Window};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TargetEditBuffer {
    pub idx: usize,
    pub name: String,
    pub ip: String,
    pub data_port: String,
    pub ctrl_port: String,
    pub channel_offset: String,
    pub channel_count: String,
    pub transport: DataTransportProtocol,
    pub interval_ms: String,
}

pub struct ConnectionModalState {
    pub is_open: bool,
    pub new_name: String,
    pub new_ip: String,
    pub new_data_port: String,
    pub new_ctrl_port: String,
    pub new_channel_offset: String,
    pub new_channel_count: String,
    pub new_interval_ms: String,
    pub new_transport: DataTransportProtocol,
    pub ping_result_msg: Option<String>,
    pub editing_target: Option<TargetEditBuffer>,
}

impl Default for ConnectionModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            new_name: "Target-02".to_string(),
            new_ip: "127.0.0.1".to_string(),
            new_data_port: "8000".to_string(),
            new_ctrl_port: "9001".to_string(),
            new_channel_offset: "51".to_string(),
            new_channel_count: "50".to_string(),
            new_interval_ms: "100".to_string(),
            new_transport: DataTransportProtocol::Udp,
            ping_result_msg: None,
            editing_target: None,
        }
    }
}

pub fn render_connection_modal(
    ctx: &egui::Context,
    state: &mut ConnectionModalState,
    target_configs: &mut Vec<TargetDeviceConfig>,
    targets_runtime: &Arc<RwLock<HashMap<u16, TargetDeviceRuntime>>>,
    verify_crc: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    if !state.is_open {
        return;
    }

    Window::new("연결 관리 (Connection Management)")
        .collapsible(false)
        .resizable([true, true])
        .default_size([880.0, 680.0])
        .min_width(800.0)
        .show(ctx, |ui| {
            ui.heading("타겟 장치 및 통신 프로토콜 설정");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let mut cur_verify = verify_crc.load(std::sync::atomic::Ordering::Relaxed);
                if ui
                    .checkbox(&mut cur_verify, "🛡 수신 패킷 CRC-16 무결성 검증 활성화 (체크 해제 시 CRC 오류 무시)")
                    .changed()
                {
                    verify_crc.store(cur_verify, std::sync::atomic::Ordering::Relaxed);
                }
            });
            ui.add_space(2.0);

            ui.label(
                RichText::new("※ 실시간 계측 데이터 통신 프로토콜은 MTU 초과 대용량 데이터 시 TCP를 권장합니다.\n※ 각 타겟 장치는 동일한 목적지 데이터 포트(8000)로 전송하며, 시작 채널(CH)을 다르게 지정하여 화면에 동시 표시합니다.")
                    .color(Color32::from_rgb(100, 180, 255))
                    .small(),
            );
            ui.add_space(6.0);

            let table_height = (ui.available_height() - 220.0).max(180.0);
            let mut targets_changed = false;

            // Target List Table
            egui::ScrollArea::vertical()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("target_list_grid")
                        .striped(true)
                        .min_col_width(60.0)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("ID");
                            ui.strong("수신 상태");
                            ui.strong("장치명");
                            ui.strong("IP 주소");
                            ui.strong("데이터 포트");
                            ui.strong("제어 포트");
                            ui.strong("시작 CH");
                            ui.strong("채널 수");
                            ui.strong("전송주기");
                            ui.strong("프로토콜");
                            ui.strong("동작");
                            ui.end_row();

                            let mut remove_idx = None;
                            let mut test_idx = None;
                            let mut start_edit = None;
                            let mut save_edit_idx = None;
                            let mut cancel_edit = false;

                            let rt_snapshot: HashMap<u16, (ConnectionStatus, u64)> = {
                                let guard = targets_runtime.read().unwrap();
                                guard.iter().map(|(&id, rt)| (id, (rt.status, rt.total_packets_received))).collect()
                            };

                            for (idx, target) in target_configs.iter_mut().enumerate() {
                                ui.label(format!("#{}", target.device_id));

                                // 1. 실시간 수신 상태 표시 (Normal, Delayed, Offline)
                                let (status, rx_cnt) = rt_snapshot.get(&target.device_id).copied().unwrap_or((ConnectionStatus::Offline, 0));
                                match status {
                                    ConnectionStatus::Normal => {
                                        ui.label(RichText::new(format!("🟢 수신중 ({})", rx_cnt)).color(Color32::from_rgb(100, 240, 100)).small());
                                    }
                                    ConnectionStatus::Delayed => {
                                        ui.label(RichText::new(format!("🟡 지연 ({})", rx_cnt)).color(Color32::YELLOW).small());
                                    }
                                    ConnectionStatus::Offline => {
                                        ui.label(RichText::new("⚪ 미수신").color(Color32::GRAY).small());
                                    }
                                }

                                let is_editing = state.editing_target.as_ref().map(|e| e.idx) == Some(idx);
                                if is_editing {
                                    let edit = state.editing_target.as_mut().unwrap();
                                    ui.add_sized([85.0, 20.0], egui::TextEdit::singleline(&mut edit.name));
                                    ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut edit.ip));
                                    ui.add_sized([50.0, 20.0], egui::TextEdit::singleline(&mut edit.data_port));
                                    ui.add_sized([50.0, 20.0], egui::TextEdit::singleline(&mut edit.ctrl_port));
                                    ui.add_sized([45.0, 20.0], egui::TextEdit::singleline(&mut edit.channel_offset));
                                    ui.add_sized([45.0, 20.0], egui::TextEdit::singleline(&mut edit.channel_count))
                                        .on_hover_text("이 타겟이 담당할 채널 수 (예: 50, 16, 32)");
                                    ui.add_sized([45.0, 20.0], egui::TextEdit::singleline(&mut edit.interval_ms))
                                        .on_hover_text("패킷 전송 주기 (ms, 기본 100ms)");

                                    egui::ComboBox::from_id_salt(format!("edit_proto_{}", idx))
                                        .selected_text(match edit.transport {
                                            DataTransportProtocol::Udp => "UDP",
                                            DataTransportProtocol::Tcp => "TCP",
                                        })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut edit.transport, DataTransportProtocol::Udp, "UDP (표준)");
                                            ui.selectable_value(&mut edit.transport, DataTransportProtocol::Tcp, "TCP (대용량)");
                                        });

                                    ui.horizontal(|ui| {
                                        if ui.button(RichText::new("✔ 저장").color(Color32::from_rgb(100, 240, 100))).clicked() {
                                            save_edit_idx = Some(idx);
                                        }
                                        if ui.button("✕ 취소").clicked() {
                                            cancel_edit = true;
                                        }
                                    });
                                } else {
                                    ui.label(&target.name);
                                    ui.label(target.ip.to_string());
                                    ui.label(target.data_port.to_string());
                                    ui.label(target.control_port.to_string());
                                    ui.label(format!("CH{:03}", target.channel_offset + 1));
                                    ui.label(format!("{}ch", target.channel_count));
                                    ui.label(format!("{}ms", target.packet_interval_ms));
                                    ui.label(match target.transport {
                                        DataTransportProtocol::Udp => "UDP",
                                        DataTransportProtocol::Tcp => "TCP",
                                    });

                                    ui.horizontal(|ui| {
                                        if ui.button("✏ 수정").clicked() {
                                            start_edit = Some((idx, target.clone()));
                                        }
                                        if ui.button("Ping").clicked() {
                                            test_idx = Some(idx);
                                        }
                                        if ui.button("삭제").clicked() {
                                            remove_idx = Some(idx);
                                        }
                                    });
                                }
                                ui.end_row();
                            }

                            if let Some((idx, t)) = start_edit {
                                state.editing_target = Some(TargetEditBuffer {
                                    idx,
                                    name: t.name,
                                    ip: t.ip.to_string(),
                                    data_port: t.data_port.to_string(),
                                    ctrl_port: t.control_port.to_string(),
                                    channel_offset: format!("{}", t.channel_offset + 1),
                                    channel_count: format!("{}", t.channel_count),
                                    transport: t.transport,
                                    interval_ms: format!("{}", t.packet_interval_ms),
                                });
                            }

                            if cancel_edit {
                                state.editing_target = None;
                            }

                            if let Some(idx) = save_edit_idx {
                                if let Some(edit) = state.editing_target.take() {
                                    if let (Ok(ip), Ok(data_p), Ok(ctrl_p), Ok(ch_off)) = (
                                        edit.ip.parse(),
                                        edit.data_port.parse(),
                                        edit.ctrl_port.parse(),
                                        edit.channel_offset.parse::<usize>(),
                                    ) {
                                        let ch_cnt = edit.channel_count.parse::<usize>().unwrap_or(50).max(1);
                                        let interval = edit.interval_ms.parse::<u32>().unwrap_or(100).max(1);
                                        let t = &mut target_configs[idx];
                                        t.name = edit.name;
                                        t.ip = ip;
                                        t.data_port = data_p;
                                        t.control_port = ctrl_p;
                                        t.channel_offset = ch_off.saturating_sub(1);
                                        t.channel_count = ch_cnt;
                                        t.transport = edit.transport;
                                        t.packet_interval_ms = interval;
                                        targets_changed = true;
                                    }
                                }
                            }

                            if let Some(idx) = remove_idx {
                                target_configs.remove(idx);
                                targets_changed = true;
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

            // Row 1: 장치명 & IP 주소 & 시작 채널 & 채널 수
            ui.horizontal(|ui| {
                ui.label("장치명:");
                ui.add_sized([130.0, 22.0], egui::TextEdit::singleline(&mut state.new_name));

                ui.add_space(10.0);
                ui.label("IP 주소:");
                ui.add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut state.new_ip));

                ui.add_space(10.0);
                ui.label("시작 CH:");
                ui.add_sized([55.0, 22.0], egui::TextEdit::singleline(&mut state.new_channel_offset))
                    .on_hover_text("이 타겟의 첫 번째 데이터가 매핑될 Studio 채널 번호 (예: 51은 CH051부터 시작)");

                ui.add_space(10.0);
                ui.label("채널 수:");
                ui.add_sized([55.0, 22.0], egui::TextEdit::singleline(&mut state.new_channel_count))
                    .on_hover_text("이 타겟이 수신할 채널 개수 (예: 50, 16, 32)");
            });

            ui.add_space(6.0);

            // Row 2: 데이터 포트 & 제어 포트 & 프로토콜 & 등록 버튼
            ui.horizontal(|ui| {
                ui.label("데이터 포트:");
                ui.add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut state.new_data_port))
                    .on_hover_text("기본값: 8000");

                ui.add_space(14.0);
                ui.label("제어 포트:");
                ui.add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut state.new_ctrl_port));

                ui.add_space(10.0);
                ui.label("전송주기:");
                ui.add_sized([45.0, 22.0], egui::TextEdit::singleline(&mut state.new_interval_ms))
                    .on_hover_text("패킷 전송 주기 (ms, 기본 100ms)");

                ui.add_space(10.0);
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
                    if let (Ok(ip), Ok(data_p), Ok(ctrl_p), Ok(ch_off)) = (
                        state.new_ip.parse(),
                        state.new_data_port.parse(),
                        state.new_ctrl_port.parse(),
                        state.new_channel_offset.parse::<usize>(),
                    ) {
                        let ch_cnt = state.new_channel_count.parse::<usize>().unwrap_or(50).max(1);
                        let interval = state.new_interval_ms.parse::<u32>().unwrap_or(100).max(1);
                        let dev_id = (target_configs.len() + 1) as u16;
                        let default_local = format!("downloads/Target-{:02}", dev_id);
                        target_configs.push(TargetDeviceConfig {
                            device_id: dev_id,
                            name: state.new_name.clone(),
                            ip,
                            data_port: data_p,
                            control_port: ctrl_p,
                            sftp_port: 22,
                            transport: state.new_transport,
                            channel_offset: ch_off.saturating_sub(1),
                            channel_count: ch_cnt,
                            enabled: true,
                            sftp_ip: Some(ip),
                            sftp_user: "admin".to_string(),
                            sftp_password: "".to_string(),
                            remote_dir: "/home/admin/data".to_string(),
                            local_dir: default_local,
                            packet_interval_ms: interval,
                        });
                        targets_changed = true;

                        // 다음 장치 입력값 자동 세팅
                        let next_num = target_configs.len() + 1;
                        state.new_name = format!("Target-{:02}", next_num);
                        state.new_ctrl_port = format!("{}", 9000 + next_num - 1);
                        let next_offset = ch_off.saturating_sub(1) + ch_cnt;
                        state.new_channel_offset = format!("{}", next_offset + 1);
                    }
                }
            });

            // 변경 사항이 발생하면 런타임 HashMap 동기화 및 파일에 영구 저장
            if targets_changed {
                let mut guard = targets_runtime.write().unwrap();
                guard.clear();
                for t in target_configs.iter() {
                    guard.insert(t.device_id, TargetDeviceRuntime::new(t.clone()));
                }
                let config_path = crate::model::target::resolve_target_config_path();
                let _ = TargetDeviceConfig::save_to_file(&config_path, target_configs);
            }

            let remaining = (ui.available_height() - 32.0).max(0.0);
            if remaining > 0.0 {
                ui.add_space(remaining);
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("닫기").clicked() {
                        state.is_open = false;
                    }
                });
            });
        });
}
