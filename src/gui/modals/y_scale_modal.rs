use crate::model::channel::ChannelState;
use crate::model::page::GraphDisplayConfig;
use eframe::egui::{self, Color32, DragValue, RichText, Window};
use std::sync::{Arc, RwLock};

pub struct YScaleModalState {
    pub is_open: bool,
    pub title: String,
    pub channel_indices: Vec<usize>,
    pub temp_limits: Vec<(usize, f64, f64)>, // (ch_idx, min, max)
}

impl Default for YScaleModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            title: "Y스케일 사용자 지정".to_string(),
            channel_indices: Vec::new(),
            temp_limits: Vec::new(),
        }
    }
}

impl YScaleModalState {
    pub fn open_for(
        &mut self,
        title: &str,
        channels: &[usize],
        config: &GraphDisplayConfig,
        channels_state: &[ChannelState],
    ) {
        self.is_open = true;
        self.title = title.to_string();
        self.channel_indices = channels.to_vec();
        self.temp_limits.clear();

        for &ch_idx in channels {
            if ch_idx < channels_state.len() {
                let default_min = channels_state[ch_idx].config.alarm_low as f64;
                let default_max = channels_state[ch_idx].config.alarm_high as f64;
                let (min, max) = config.get_channel_y_limit(ch_idx, default_min, default_max);
                self.temp_limits.push((ch_idx, min, max));
            }
        }
    }
}

pub fn render_y_scale_modal<F>(
    ctx: &egui::Context,
    state: &mut YScaleModalState,
    channels_arc: &Arc<RwLock<Vec<ChannelState>>>,
    mut on_save: F,
) where
    F: FnMut(&[(usize, f64, f64)]),
{
    if !state.is_open {
        return;
    }

    Window::new(format!("⚙ {}", state.title))
        .collapsible(false)
        .resizable([true, true])
        .default_size([580.0, 600.0])
        .min_width(520.0)
        .min_height(350.0)
        .show(ctx, |ui| {
            ui.heading("채널별 Y축 표시 범위(Y-Min / Y-Max) 설정");
            ui.label(
                RichText::new("※ 다중 Y축 오버레이 모드 활성화 시 각 채널 파형이 아래 설정된 범위에 맞게 정규화됩니다.")
                    .small()
                    .color(Color32::from_rgb(100, 180, 255)),
            );
            ui.add_space(8.0);

            let channels = channels_arc.read().unwrap();

            // 하단 버튼 영역 제외 후 테이블 상하 유동 확장
            let table_height = (ui.available_height() - 45.0).max(200.0);

            egui::ScrollArea::vertical()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("y_scale_grid")
                        .striped(true)
                        .min_col_width(65.0)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("CH");
                            ui.strong("채널명");
                            ui.strong("단위");
                            ui.strong("Y-Min (하한)");
                            ui.strong("Y-Max (상한)");
                            ui.end_row();

                            for item in state.temp_limits.iter_mut() {
                                let (ch_idx, min, max) = item;
                                if *ch_idx < channels.len() {
                                    let ch = &channels[*ch_idx];
                                    ui.label(format!("{:03}", *ch_idx + 1));
                                    ui.label(&ch.config.name);
                                    ui.label(&ch.config.unit);
                                    ui.add(DragValue::new(min).speed(0.1));
                                    ui.add(DragValue::new(max).speed(0.1));
                                    ui.end_row();
                                }
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
                    if ui
                        .button(
                            RichText::new("💾 적용 및 저장").strong().color(Color32::from_rgb(100, 240, 100)),
                        )
                        .clicked()
                    {
                        on_save(&state.temp_limits);
                        state.is_open = false;
                    }
                });
            });
        });
}
