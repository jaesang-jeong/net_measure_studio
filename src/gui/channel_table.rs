use crate::gui::modals::channel_select_modal::{ChannelSelectModalState, ChannelSelectTarget};
use crate::model::channel::{ChannelAlarmLevel, ChannelState};
use crate::model::page::GraphDisplayConfig;
use eframe::egui::{self, Color32, RichText, ScrollArea, Ui};
use std::sync::{Arc, RwLock};

pub fn render_channel_panels(
    ui: &mut Ui,
    page_index: usize,
    is_dual: bool,
    channels_arc: &Arc<RwLock<Vec<ChannelState>>>,
    top_channels: &mut Vec<usize>,
    bottom_channels: &mut Vec<usize>,
    top_config: &mut GraphDisplayConfig,
    bottom_config: &mut GraphDisplayConfig,
    select_modal_state: &mut ChannelSelectModalState,
    on_layout_changed: &mut bool,
) {
    let channels = channels_arc.read().unwrap();
    let available_height = ui.available_height();

    if is_dual {
        // [Page 1: 메인 페이지 - 2개의 분리된 채널 리스트 (상단 그래프 채널 / 하단 그래프 채널)]
        let half_height = (available_height * 0.49).max(180.0);

        // 1. 상단 그래프 채널 리스트
        ui.group(|ui| {
            ui.set_height(half_height);
            ui.horizontal(|ui| {
                ui.strong(
                    RichText::new(format!("📈 상단 그래프 채널 ({})", top_channels.len()))
                        .color(Color32::from_rgb(0, 190, 255)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ 채널 선택").clicked() {
                        select_modal_state.open_for(
                            page_index,
                            ChannelSelectTarget::TopGraph,
                            top_channels,
                        );
                    }
                });
            });

            ui.separator();

            if top_channels.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label("(표시할 채널이 없습니다)");
                    ui.label(RichText::new("위 [➕ 채널 선택] 버튼을 눌러 채널을 추가하세요").small().color(Color32::GRAY));
                });
            } else {
                ScrollArea::vertical()
                    .id_salt("top_channel_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_channel_items_table(
                            ui,
                            top_channels,
                            top_config,
                            &channels,
                            on_layout_changed,
                        );
                    });
            }
        });

        ui.add_space(4.0);

        // 2. 하단 그래프 채널 리스트
        ui.group(|ui| {
            ui.set_height(half_height);
            ui.horizontal(|ui| {
                ui.strong(
                    RichText::new(format!("📉 하단 서브 그래프 채널 ({})", bottom_channels.len()))
                        .color(Color32::from_rgb(255, 140, 0)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ 채널 선택").clicked() {
                        select_modal_state.open_for(
                            page_index,
                            ChannelSelectTarget::BottomGraph,
                            bottom_channels,
                        );
                    }
                });
            });

            ui.separator();

            if bottom_channels.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label("(표시할 채널이 없습니다)");
                    ui.label(RichText::new("위 [➕ 채널 선택] 버튼을 눌러 채널을 추가하세요").small().color(Color32::GRAY));
                });
            } else {
                ScrollArea::vertical()
                    .id_salt("bottom_channel_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_channel_items_table(
                            ui,
                            bottom_channels,
                            bottom_config,
                            &channels,
                            on_layout_changed,
                        );
                    });
            }
        });
    } else {
        // [Page 2~10: 1 그래프 구성 - 1개의 채널 리스트]
        ui.group(|ui| {
            ui.set_height(available_height - 8.0);
            ui.horizontal(|ui| {
                ui.strong(
                    RichText::new(format!("📊 그래프 채널 리스트 ({})", top_channels.len()))
                        .color(Color32::from_rgb(100, 220, 255)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ 채널 선택").clicked() {
                        select_modal_state.open_for(
                            page_index,
                            ChannelSelectTarget::SingleGraph,
                            top_channels,
                        );
                    }
                });
            });

            ui.separator();

            if top_channels.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("(표시할 채널이 없습니다)");
                    ui.label(RichText::new("위 [➕ 채널 선택] 버튼을 눌러 그래프에 띄울 채널을 선택하세요").small().color(Color32::GRAY));
                });
            } else {
                ScrollArea::vertical()
                    .id_salt("single_channel_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_channel_items_table(
                            ui,
                            top_channels,
                            top_config,
                            &channels,
                            on_layout_changed,
                        );
                    });
            }
        });
    }
}

/// 그래프에 할당된 채널 목록 테이블: [CH], [채널명], [실시간 수치], [단위], [상태], [제거(✕)]
fn render_channel_items_table(
    ui: &mut Ui,
    channel_list: &mut Vec<usize>,
    config: &mut GraphDisplayConfig,
    channels: &[ChannelState],
    on_layout_changed: &mut bool,
) {
    egui::Grid::new(ui.next_auto_id())
        .striped(true)
        .min_col_width(28.0)
        .spacing([8.0, 5.0])
        .show(ui, |ui| {
            ui.strong("CH");
            ui.strong("채널명");
            ui.strong("실시간 수치");
            ui.strong("단위");
            ui.strong("상태");
            ui.strong("제거");
            ui.end_row();

            let mut remove_idx = None;

            for (pos, &ch_idx) in channel_list.iter().enumerate() {
                if ch_idx >= channels.len() {
                    continue;
                }
                let ch = &channels[ch_idx];
                let is_hidden = config.is_channel_hidden(ch_idx);

                // 1. CH ID
                let ch_id_color = if is_hidden { Color32::GRAY } else { Color32::WHITE };
                ui.label(RichText::new(format!("{:03}", ch_idx + 1)).color(ch_id_color));

                // 2. 채널명 (범례에서 감춰진 채널은 회색 텍스트로 표시)
                ui.label(RichText::new(&ch.config.name).color(ch_id_color));

                // 3. 실시간 값
                let val_str = format!("{:>8.2}", ch.scaled_value);
                let val_text = if is_hidden {
                    RichText::new(val_str).color(Color32::GRAY)
                } else {
                    match ch.alarm_level {
                        ChannelAlarmLevel::Normal => RichText::new(val_str).strong().color(Color32::from_rgb(180, 240, 180)),
                        ChannelAlarmLevel::Warning => RichText::new(val_str).strong().color(Color32::from_rgb(255, 220, 80)),
                        ChannelAlarmLevel::Alarm => RichText::new(val_str).strong().color(Color32::from_rgb(255, 80, 80)),
                    }
                };
                ui.label(val_text);

                // 4. 단위
                ui.label(RichText::new(&ch.config.unit).color(ch_id_color));

                // 5. 상태 배지 (범례에서 감춰진 경우 '(감춤)' 표시)
                if is_hidden {
                    ui.label(RichText::new("감춤").color(Color32::GRAY).small());
                } else {
                    let (badge_text, badge_color) = match ch.alarm_level {
                        ChannelAlarmLevel::Normal => ("정상", Color32::from_rgb(40, 180, 40)),
                        ChannelAlarmLevel::Warning => ("주의", Color32::from_rgb(220, 160, 20)),
                        ChannelAlarmLevel::Alarm => ("위험", Color32::from_rgb(220, 40, 40)),
                    };
                    ui.label(RichText::new(badge_text).color(badge_color).small());
                }

                // 6. 제거 [X] 버튼 (폰트 호환성을 위해 ASCII 'X' 사용)
                if ui.button(RichText::new("X").color(Color32::from_rgb(255, 90, 90)).strong().size(11.0)).on_hover_text("그래프에서 채널 제거").clicked() {
                    remove_idx = Some(pos);
                }

                ui.end_row();
            }

            if let Some(pos) = remove_idx {
                let removed_ch = channel_list.remove(pos);
                // 감춘 목록에서도 정리
                if let Some(h_pos) = config.hidden_channels.iter().position(|&x| x == removed_ch) {
                    config.hidden_channels.remove(h_pos);
                }
                *on_layout_changed = true;
            }
        });
}
