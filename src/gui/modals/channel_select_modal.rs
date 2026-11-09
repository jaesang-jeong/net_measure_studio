use crate::model::channel::ChannelState;
use eframe::egui::{self, Color32, RichText, ScrollArea, Window};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelSelectTarget {
    TopGraph,    // Page 1 상단 메인 그래프
    BottomGraph, // Page 1 하단 서브 그래프
    SingleGraph, // Page 2~10 단일 대형 그래프
}

pub struct ChannelSelectModalState {
    pub is_open: bool,
    pub page_index: usize,
    pub target: ChannelSelectTarget,
    pub selected_channels: HashSet<usize>,
    pub search_query: String,
    pub has_changes: bool,
}

impl Default for ChannelSelectModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            page_index: 0,
            target: ChannelSelectTarget::TopGraph,
            selected_channels: HashSet::new(),
            search_query: String::new(),
            has_changes: false,
        }
    }
}

impl ChannelSelectModalState {
    pub fn open_for(
        &mut self,
        page_index: usize,
        target: ChannelSelectTarget,
        current_channels: &[usize],
    ) {
        self.is_open = true;
        self.page_index = page_index;
        self.target = target;
        self.selected_channels = current_channels.iter().copied().collect();
        self.search_query.clear();
        self.has_changes = false;
    }
}

pub fn render_channel_select_modal<F>(
    ctx: &egui::Context,
    state: &mut ChannelSelectModalState,
    channels_arc: &Arc<RwLock<Vec<ChannelState>>>,
    mut on_apply: F,
) where
    F: FnMut(usize, ChannelSelectTarget, Vec<usize>),
{
    if !state.is_open {
        return;
    }

    let target_title = match state.target {
        ChannelSelectTarget::TopGraph => format!("Page {}: 상단 메인 그래프 채널 선택", state.page_index + 1),
        ChannelSelectTarget::BottomGraph => format!("Page {}: 하단 서브 그래프 채널 선택", state.page_index + 1),
        ChannelSelectTarget::SingleGraph => format!("Page {}: 그래프 채널 선택", state.page_index + 1),
    };

    Window::new(format!("📊 {}", target_title))
        .collapsible(false)
        .resizable([false, true]) // 좌우 고정, 상하 조절 가능
        .min_width(700.0)
        .max_width(700.0)         // 내용에 맞게 좌우 크기 고정
        .default_height(750.0)     // 상하로 넉넉한 기본 높이
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("그래프에 표시할 채널을 체크하세요 (다중 선택 가능)");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("검색:");
                    ui.text_edit_singleline(&mut state.search_query);
                });
            });

            let channels = channels_arc.read().unwrap();
            let total_channels_count = channels.len();
            let query = state.search_query.to_lowercase();

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(format!("선택된 채널 수: {}개", state.selected_channels.len()));

                if ui.button(format!("전체 선택 ({}ch)", total_channels_count)).clicked() {
                    for i in 0..total_channels_count {
                        state.selected_channels.insert(i);
                    }
                    state.has_changes = true;
                }

                if ui.button("전체 해제").clicked() {
                    state.selected_channels.clear();
                    state.has_changes = true;
                }

                if ui.button("선택 반전").clicked() {
                    for i in 0..total_channels_count {
                        if state.selected_channels.contains(&i) {
                            state.selected_channels.remove(&i);
                        } else {
                            state.selected_channels.insert(i);
                        }
                    }
                    state.has_changes = true;
                }
            });

            ui.separator();

            // 하단 버튼 영역(약 45px) 제외 후 남은 상하 영역을 100% 꽉 채움!
            let table_height = (ui.available_height() - 45.0).max(250.0);

            ScrollArea::vertical()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("ch_select_modal_grid")
                        .striped(true)
                        .min_col_width(55.0)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("선택");
                            ui.strong("CH");
                            ui.strong("채널명");
                            ui.strong("센서타입");
                            ui.strong("단위");
                            ui.strong("현재 측정값");
                            ui.end_row();

                            for (idx, ch) in channels.iter().enumerate() {
                                if !query.is_empty()
                                    && !ch.config.name.to_lowercase().contains(&query)
                                    && !ch.config.sensor_type.to_lowercase().contains(&query)
                                    && !format!("ch{}", idx + 1).contains(&query)
                                {
                                    continue;
                                }

                                let mut is_checked = state.selected_channels.contains(&idx);
                                if ui.checkbox(&mut is_checked, "").changed() {
                                    if is_checked {
                                        state.selected_channels.insert(idx);
                                    } else {
                                        state.selected_channels.remove(&idx);
                                    }
                                    state.has_changes = true;
                                }

                                ui.label(format!("{:03}", idx + 1));
                                ui.label(&ch.config.name);
                                ui.label(&ch.config.sensor_type);
                                ui.label(&ch.config.unit);
                                ui.label(format!("{:.2}", ch.scaled_value));
                                ui.end_row();
                            }
                        });
                });

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

                    ui.add_space(8.0);
                    let apply_btn = egui::Button::new(
                        RichText::new("💾 적용 및 저장 (Apply & Save)").strong().color(Color32::from_rgb(100, 240, 100)),
                    );
                    if ui.add(apply_btn).clicked() {
                        let mut sorted: Vec<usize> = state.selected_channels.iter().copied().collect();
                        sorted.sort_unstable();
                        on_apply(state.page_index, state.target, sorted);
                        state.is_open = false;
                    }
                });
            });
        });
}
