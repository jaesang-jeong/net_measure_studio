use crate::model::channel::ChannelState;
use crate::model::target::TargetDeviceConfig;
use crate::network::control_client::ControlClient;
use crate::protocol::commands::ChannelConfigItem;
use eframe::egui::{self, Color32, RichText, Window};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

pub struct ConfigModalState {
    pub is_open: bool,
    pub status_message: Option<(String, bool)>, // (msg, is_error)
    pub search_query: String,
}

impl Default for ConfigModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            status_message: None,
            search_query: String::new(),
        }
    }
}

pub fn render_config_modal(
    ctx: &egui::Context,
    state: &mut ConfigModalState,
    channels_arc: &Arc<RwLock<Vec<ChannelState>>>,
    targets: &[TargetDeviceConfig],
) {
    if !state.is_open {
        return;
    }

    Window::new("채널 파라미터 설정 및 동기화 (Channel Configuration)")
        .collapsible(false)
        .resizable([true, true])
        .default_size([1180.0, 750.0])
        .min_width(950.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("채널별 스케일링(Y = aX + b) 및 임계값 설정");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.search_query).desired_width(160.0));
                    ui.label("검색:");
                });
            });
            ui.add_space(4.0);

            // Action buttons: Download to target, Upload from target
            ui.horizontal(|ui| {
                if ui.button("⬆ 타겟으로 배포 (CMD_CFG_SET)").clicked() {
                    if let Some(target) = targets.first() {
                        let addr_str = format!("{}:{}", target.ip, target.control_port);
                        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                            let ch_list: Vec<ChannelConfigItem> = {
                                let guard = channels_arc.read().unwrap();
                                guard.iter().map(|c| c.config.clone()).collect()
                            };

                            match ControlClient::set_config(addr, ch_list) {
                                Ok(_) => {
                                    state.status_message = Some((
                                        format!("[{}] 채널 설정 배포 완료!", target.name),
                                        false,
                                    ));
                                }
                                Err(e) => {
                                    state.status_message = Some((
                                        format!("[{}] 배포 실패: {}", target.name, e),
                                        true,
                                    ));
                                }
                            }
                        }
                    }
                }

                if ui.button("⬇ 타겟에서 가져오기 (CMD_CFG_GET)").clicked() {
                    if let Some(target) = targets.first() {
                        let addr_str = format!("{}:{}", target.ip, target.control_port);
                        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                            match ControlClient::get_config(addr) {
                                Ok(remote_configs) => {
                                    let mut guard = channels_arc.write().unwrap();
                                    for item in remote_configs {
                                        let idx = item.channel_index;
                                        if idx < guard.len() {
                                            guard[idx].config = item;
                                        }
                                    }
                                    state.status_message = Some((
                                        format!("[{}] 타겟 설정값 동기화 성공!", target.name),
                                        false,
                                    ));
                                }
                                Err(e) => {
                                    state.status_message = Some((
                                        format!("[{}] 가져오기 실패: {}", target.name, e),
                                        true,
                                    ));
                                }
                            }
                        }
                    }
                }

                if let Some((ref msg, is_err)) = state.status_message {
                    let color = if is_err {
                        Color32::from_rgb(255, 100, 100)
                    } else {
                        Color32::from_rgb(100, 240, 100)
                    };
                    ui.label(RichText::new(msg).color(color));
                }
            });

            ui.separator();

            let mut channels = channels_arc.write().unwrap();
            let query = state.search_query.to_lowercase();

            // 하단 닫기 버튼 높이(약 45px)를 제외한 잔여 높이를 테이블에 100% 할당 (상하 확장 완벽 지원)
            let table_height = (ui.available_height() - 45.0).max(250.0);

            // both() 스크롤 지원 및 이름(Label) 컬럼 240px 강제 지정으로 컬럼 축소 방지
            egui::ScrollArea::both()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("channels_config_grid_v3")
                        .striped(true)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("CH");
                            ui.add_sized([240.0, 20.0], egui::Label::new(RichText::new("이름 (Label)").strong()));
                            ui.add_sized([85.0, 20.0], egui::Label::new(RichText::new("단위 (Unit)").strong()));
                            ui.add_sized([110.0, 20.0], egui::Label::new(RichText::new("센서 타입").strong()));
                            ui.strong("Gain (a)");
                            ui.strong("Offset (b)");
                            ui.strong("주의하한");
                            ui.strong("주의상한");
                            ui.strong("위험하한");
                            ui.strong("위험상한");
                            ui.end_row();

                            for ch in channels.iter_mut() {
                                if !query.is_empty()
                                    && !ch.config.name.to_lowercase().contains(&query)
                                    && !format!("ch{}", ch.config.channel_index + 1).contains(&query)
                                {
                                    continue;
                                }

                                ui.label(format!("{:03}", ch.config.channel_index + 1));
                                ui.add_sized([240.0, 20.0], egui::TextEdit::singleline(&mut ch.config.name));
                                ui.add_sized([85.0, 20.0], egui::TextEdit::singleline(&mut ch.config.unit));
                                ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut ch.config.sensor_type));

                                ui.add(egui::DragValue::new(&mut ch.config.gain).speed(0.01));
                                ui.add(egui::DragValue::new(&mut ch.config.offset).speed(0.1));
                                ui.add(egui::DragValue::new(&mut ch.config.warn_low).speed(0.5));
                                ui.add(egui::DragValue::new(&mut ch.config.warn_high).speed(0.5));
                                ui.add(egui::DragValue::new(&mut ch.config.alarm_low).speed(0.5));
                                ui.add(egui::DragValue::new(&mut ch.config.alarm_high).speed(0.5));
                                ui.end_row();
                            }
                        });
                });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("닫기").clicked() {
                    state.is_open = false;
                }
            });
        });
}
