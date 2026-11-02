use crate::gui::modals::y_scale_modal::YScaleModalState;
use crate::model::channel::ChannelState;
use crate::model::page::GraphDisplayConfig;
use crate::processing::ring_buffer::ChannelRingBuffer;
use eframe::egui::{self, Color32, DragValue, Ui};
use egui_plot::{CoordinatesFormatter, Corner, Line, Plot, PlotBounds, PlotPoints};
use std::sync::{Arc, RwLock};

const COLOR_PALETTE: [Color32; 10] = [
    Color32::from_rgb(0, 190, 255),   // Cyan/Blue
    Color32::from_rgb(255, 120, 0),   // Orange
    Color32::from_rgb(50, 220, 100),  // Green
    Color32::from_rgb(255, 80, 80),   // Red
    Color32::from_rgb(230, 200, 40),  // Yellow
    Color32::from_rgb(200, 100, 255), // Purple
    Color32::from_rgb(0, 230, 200),   // Teal
    Color32::from_rgb(255, 150, 200), // Pink
    Color32::from_rgb(180, 220, 80),  // Lime
    Color32::from_rgb(180, 180, 220), // Light Gray
];

pub fn render_dual_or_single_plots(
    ui: &mut Ui,
    is_dual: bool,
    top_channels: &[usize],
    bottom_channels: &[usize],
    top_config: &mut GraphDisplayConfig,
    bottom_config: &mut GraphDisplayConfig,
    channels_arc: &Arc<RwLock<Vec<ChannelState>>>,
    ring_buffers_arc: &Arc<RwLock<Vec<ChannelRingBuffer>>>,
    y_scale_modal: &mut YScaleModalState,
    layout_changed: &mut bool,
) {
    let channels = channels_arc.read().unwrap();
    let ring_buffers = ring_buffers_arc.read().unwrap();

    let available_height = ui.available_height();

    if is_dual {
        // [Page 1: 2 그래프 구성 (상단 메인 그래프 + 하단 서브 그래프)]
        let plot_section_height = (available_height * 0.49).max(200.0);

        // 1. TOP GRAPH
        ui.group(|ui| {
            ui.set_height(plot_section_height);
            render_graph_control_bar(
                ui,
                "📈 [상단 메인 그래프]",
                top_config,
                top_channels,
                &channels,
                y_scale_modal,
                layout_changed,
            );

            render_single_plot_canvas(
                ui,
                "top_main_plot",
                plot_section_height - 38.0,
                top_channels,
                top_config,
                &channels,
                &ring_buffers,
                layout_changed,
            );
        });

        ui.add_space(4.0);

        // 2. BOTTOM GRAPH
        ui.group(|ui| {
            ui.set_height(plot_section_height);
            render_graph_control_bar(
                ui,
                "📉 [하단 서브 그래프]",
                bottom_config,
                bottom_channels,
                &channels,
                y_scale_modal,
                layout_changed,
            );

            render_single_plot_canvas(
                ui,
                "bottom_sub_plot",
                plot_section_height - 38.0,
                bottom_channels,
                bottom_config,
                &channels,
                &ring_buffers,
                layout_changed,
            );
        });
    } else {
        // [Page 2~10: 1 그래프 구성 (단일 대형 고해상도 그래프)]
        ui.group(|ui| {
            ui.set_height(available_height - 8.0);
            render_graph_control_bar(
                ui,
                "📊 [단일 대형 그래프]",
                top_config,
                top_channels,
                &channels,
                y_scale_modal,
                layout_changed,
            );

            render_single_plot_canvas(
                ui,
                "single_large_plot",
                available_height - 48.0,
                top_channels,
                top_config,
                &channels,
                &ring_buffers,
                layout_changed,
            );
        });
    }
}

/// 그래프 상단 컨트롤 툴바: X축 길이, Y스케일(Auto/수동), Multi-Y 오버레이 토글, Y설정 모달 버튼
fn render_graph_control_bar(
    ui: &mut Ui,
    title: &str,
    config: &mut GraphDisplayConfig,
    active_channels: &[usize],
    channels: &[ChannelState],
    y_scale_modal: &mut YScaleModalState,
    layout_changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.strong(title);
        ui.separator();

        // 1. X축 시간 길이 지정 (X-Axis Span)
        ui.label("X축 길이:");
        let prev_x = config.x_window_secs;
        ui.add(
            DragValue::new(&mut config.x_window_secs)
                .speed(0.5)
                .range(1.0..=300.0)
                .suffix("s"),
        );

        // 빠른 X축 프리셋 버튼
        for &preset in &[5.0, 10.0, 30.0, 60.0] {
            if ui
                .selectable_label(config.x_window_secs == preset, format!("{}s", preset))
                .clicked()
            {
                config.x_window_secs = preset;
            }
        }
        if config.x_window_secs != prev_x {
            *layout_changed = true;
        }

        ui.separator();

        // 2. Multi-Y축 다중스케일 오버레이 모드 토글
        let prev_multi = config.multi_y_overlay;
        ui.checkbox(&mut config.multi_y_overlay, "다중 Y축(오버레이)");
        if config.multi_y_overlay != prev_multi {
            *layout_changed = true;
        }

        ui.separator();

        // 3. Y스케일 사용자 지정 / Auto-Scale
        let prev_auto = config.auto_scale_y;
        ui.checkbox(&mut config.auto_scale_y, "Auto-Y");
        if config.auto_scale_y != prev_auto {
            *layout_changed = true;
        }

        if !config.auto_scale_y && !config.multi_y_overlay {
            ui.label("Y-Min:");
            let prev_ymin = config.custom_y_min;
            ui.add(DragValue::new(&mut config.custom_y_min).speed(0.5));

            ui.label("Y-Max:");
            let prev_ymax = config.custom_y_max;
            ui.add(DragValue::new(&mut config.custom_y_max).speed(0.5));

            if config.custom_y_min != prev_ymin || config.custom_y_max != prev_ymax {
                *layout_changed = true;
            }
        }

        // 4. 채널별 Y스케일 사용자 지정 팝업 버튼
        if config.multi_y_overlay {
            if ui.button("⚙ 채널별 Y스케일 설정").clicked() {
                y_scale_modal.open_for(
                    &format!("{} 채널별 Y스케일 설정", title),
                    active_channels,
                    config,
                    channels,
                );
            }
        }
    });
}

/// 실제 플롯 캔버스 렌더링 + [플롯 범례: 채널이름 [단위] ● 우측 정렬 오버레이]
fn render_single_plot_canvas(
    ui: &mut Ui,
    plot_id: &str,
    height: f32,
    channels_to_plot: &[usize],
    config: &mut GraphDisplayConfig,
    channels: &[ChannelState],
    ring_buffers: &[ChannelRingBuffer],
    layout_changed: &mut bool,
) {
    let window_secs = config.x_window_secs.max(1.0);

    // 1. 현재 최신 시각 탐색 (가장 최근 포인트의 time_secs)
    let mut latest_time: f64 = 0.0;
    for &ch_idx in channels_to_plot {
        if ch_idx < ring_buffers.len() {
            let pts = ring_buffers[ch_idx].get_points();
            if let Some(last) = pts.last() {
                if last[0] > latest_time {
                    latest_time = last[0];
                }
            }
        }
    }

    // 2. Y축 Bounds 계산
    let (plot_y_min, plot_y_max) = if config.multi_y_overlay {
        // 다중 Y축 모드일 때는 모든 채널이 0.0 ~ 100.0%로 정규화되어 오버레이됨
        (-5.0, 105.0)
    } else if config.auto_scale_y {
        // Auto-Scale 모드
        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;
        for &ch_idx in channels_to_plot {
            if config.is_channel_hidden(ch_idx) || ch_idx >= ring_buffers.len() {
                continue;
            }
            let pts = ring_buffers[ch_idx].get_points();
            for pt in pts {
                let v = pt[1];
                if v < min_val { min_val = v; }
                if v > max_val { max_val = v; }
            }
        }
        if min_val >= max_val {
            (-10.0, 10.0)
        } else {
            let margin = ((max_val - min_val) * 0.1).max(0.5);
            (min_val - margin, max_val + margin)
        }
    } else {
        // 사용자 직접 지정 단일 Y스케일
        (config.custom_y_min, config.custom_y_max)
    };

    let plot = Plot::new(plot_id)
        .height(height)
        .coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default())
        .show_axes([true, true])
        .show_grid([true, true]);

    let plot_resp = plot.show(ui, |plot_ui| {
        // [플롯 영역 좌우로 100% 꽉 채우기]: 최신 시각 기준 [-window_secs, 0.0] 범위로 플롯 경계 고정!
        let bounds = PlotBounds::from_min_max([-window_secs, plot_y_min], [0.0, plot_y_max]);
        plot_ui.set_plot_bounds(bounds);

        // [플롯 호버 강조 상태 파악]: 가시 채널 중 현재 호버된 채널 확인
        let active_hovered_visible = config.hovered_channel.filter(|&h_idx| !config.is_channel_hidden(h_idx));
        let mut hovered_line_to_render = None;

        // 플롯 렌더링
        for (i, &ch_idx) in channels_to_plot.iter().enumerate() {
            // [플롯 감추기 기능]: 감춰진 채널은 렌더링 건너뜀
            if config.is_channel_hidden(ch_idx) {
                continue;
            }

            if ch_idx < ring_buffers.len() && ch_idx < channels.len() {
                let base_color = COLOR_PALETTE[i % COLOR_PALETTE.len()];
                let ch_info = &channels[ch_idx];
                let pts = ring_buffers[ch_idx].get_points();

                let (ch_ymin, ch_ymax) = config.get_channel_y_limit(
                    ch_idx,
                    ch_info.config.alarm_low as f64,
                    ch_info.config.alarm_high as f64,
                );

                // 최신 시간 기준 상대 시간 [-window_secs ~ 0.0]으로 변환하여 좌우 빈틈없이 100% 꽉 채움!
                let mut mapped_points: Vec<[f64; 2]> = Vec::with_capacity(pts.len() + 2);
                let mut first_in_window: Option<[f64; 2]> = None;
                let mut prev_outside_point: Option<[f64; 2]> = None;

                for pt in &pts {
                    let rel_x = pt[0] - latest_time;
                    let y_val = if config.multi_y_overlay {
                        let span = (ch_ymax - ch_ymin).max(0.001);
                        ((pt[1] as f64 - ch_ymin) / span) * 100.0
                    } else {
                        pt[1] as f64
                    };

                    if rel_x < -window_secs {
                        prev_outside_point = Some([rel_x, y_val]);
                    } else if rel_x <= 0.05 {
                        if first_in_window.is_none() {
                            first_in_window = Some([rel_x, y_val]);
                            if let Some(prev) = prev_outside_point {
                                let t_frac = (-window_secs - prev[0]) / (rel_x - prev[0]).max(1e-6);
                                let interp_y = prev[1] + t_frac * (y_val - prev[1]);
                                mapped_points.push([-window_secs, interp_y]);
                            } else {
                                mapped_points.push([-window_secs, y_val]);
                            }
                        }
                        mapped_points.push([rel_x, y_val]);
                    }
                }

                if mapped_points.is_empty() {
                    if let Some(last) = pts.last() {
                        let y = if config.multi_y_overlay { 50.0 } else { last[1] as f64 };
                        mapped_points.push([-window_secs, y]);
                        mapped_points.push([0.0, y]);
                    }
                } else if let Some(last) = mapped_points.last().copied() {
                    if last[0] < -0.05 {
                        mapped_points.push([0.0, last[1]]);
                    }
                }

                let plot_points = PlotPoints::new(mapped_points);

                // [플롯 호버 강조 렌더링 (자연스럽고 부드러운 강도 조절)]:
                // - 호버된 채널은 2.6px 굵기로 최상단에 깨끗하게 표시
                // - 다른 채널들은 완전히 지워지지 않도록 1.5px 및 0.65 감쇠율로 자연스럽게 정돈
                // - 호버가 없으면 기본 1.8px 원본 색상 유지
                if active_hovered_visible == Some(ch_idx) {
                    hovered_line_to_render = Some((plot_points, base_color));
                } else {
                    let (line_color, line_width) = if active_hovered_visible.is_some() {
                        (base_color.gamma_multiply(0.65), 1.5_f32)
                    } else {
                        (base_color, 1.8_f32)
                    };

                    plot_ui.line(
                        Line::new(plot_points)
                            .color(line_color)
                            .width(line_width),
                    );
                }
            }
        }

        // 호버된 채널 선을 가장 마지막에 최상단으로 굵기 2.6px로 렌더링
        if let Some((points, color)) = hovered_line_to_render {
            plot_ui.line(
                Line::new(points)
                    .color(color)
                    .width(2.6_f32),
            );
        }
    });

    // =========================================================================
    // [플롯 범례]: 채널 이름 및 단위만 표시 (우측 정렬: 이름 [단위] ●) + 컴팩트 사이즈 & 마우스 호버 강조
    // =========================================================================
    let mut newly_hovered: Option<usize> = None;

    if !channels_to_plot.is_empty() {
        let plot_rect = plot_resp.response.rect;

        // 1. 범례 내부 텍스트 최대 폭을 측정하여 범례 박스를 컨텐츠 크기에 딱 맞춤 (컴팩트 자동 피팅)
        let font_id = egui::FontId::proportional(12.0);
        let mut max_text_width: f32 = 60.0;
        for &ch_idx in channels_to_plot {
            if ch_idx < channels.len() {
                let ch = &channels[ch_idx];
                let label_text = format!("{} [{}]", ch.config.name, ch.config.unit);
                let galley = ui.painter().layout_no_wrap(label_text, font_id.clone(), Color32::WHITE);
                max_text_width = max_text_width.max(galley.size().x);
            }
        }

        let dot_width = 16.0_f32;
        let spacing_x = 8.0_f32;
        let row_width = max_text_width + spacing_x + dot_width + 8.0;
        let row_height = 20.0_f32;
        let padding_x = 8.0_f32;
        let padding_y = 6.0_f32;
        let spacing_y = 3.0_f32;

        let total_legend_width = row_width + padding_x * 2.0;
        let total_legend_height = (channels_to_plot.len() as f32 * row_height)
            + (channels_to_plot.len().saturating_sub(1) as f32 * spacing_y)
            + padding_y * 2.0;

        // 그래프 캔버스 내부 우측 상단에 플롯과 동일한 레이어로 배치 (모달 팝업창 뒤로 깔끔하게 가려짐)
        let legend_rect = egui::Rect::from_min_size(
            egui::pos2(plot_rect.right() - total_legend_width - 10.0, plot_rect.top() + 10.0),
            egui::vec2(total_legend_width, total_legend_height),
        );

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(legend_rect), |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(18, 18, 22, 225))
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(55, 55, 60)))
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(padding_x, padding_y))
                .show(ui, |ui| {
                    ui.set_width(row_width);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = spacing_y;

                        for (i, &ch_idx) in channels_to_plot.iter().enumerate() {
                            if ch_idx >= channels.len() {
                                continue;
                            }
                            let ch = &channels[ch_idx];
                            let color = COLOR_PALETTE[i % COLOR_PALETTE.len()];
                            let is_hidden = config.is_channel_hidden(ch_idx);
                            let is_this_hovered = config.hovered_channel == Some(ch_idx);
                            let label_text = format!("{} [{}]", ch.config.name, ch.config.unit);

                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                egui::vec2(row_width, row_height),
                                egui::Sense::click(),
                            );

                            if row_resp.hovered() {
                                newly_hovered = Some(ch_idx);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            if row_resp.clicked() {
                                config.toggle_channel_visibility(ch_idx);
                                *layout_changed = true;
                            }

                            row_resp.on_hover_text(if is_hidden {
                                format!("클릭하여 [{}] 플롯 표시", ch.config.name)
                            } else {
                                format!("클릭하여 [{}] 플롯 감추기", ch.config.name)
                            });

                            let painter = ui.painter_at(row_rect);

                            // [범례 강조 시 배경은 강조하지 않음 - 글자와 아이콘만 강조]

                            // 1. 색상 아이콘 (Dot ● / ○) - 우측 끝 정렬
                            let dot_center = egui::pos2(row_rect.max.x - 10.0, row_rect.center().y);
                            if is_hidden {
                                // 감춤 상태: 빈 원
                                let stroke_c = if is_this_hovered {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(110, 110, 110)
                                };
                                let radius = if is_this_hovered { 5.2_f32 } else { 4.5_f32 };
                                let stroke_w = if is_this_hovered { 1.6_f32 } else { 1.4_f32 };
                                painter.circle_stroke(dot_center, radius, egui::Stroke::new(stroke_w, stroke_c));
                            } else {
                                // 표시 상태: 색상 점 (호버 시 크기 4.5px -> 5.5px 자연스럽게 확대 및 또렷한 림 스트로크)
                                let radius = if is_this_hovered { 5.5_f32 } else { 4.5_f32 };
                                painter.circle_filled(dot_center, radius, color);
                                if is_this_hovered {
                                    painter.circle_stroke(
                                        dot_center,
                                        radius + 0.5,
                                        egui::Stroke::new(0.8_f32, Color32::WHITE),
                                    );
                                }
                            }

                            // 2. 채널명 및 단위 텍스트 - 색상 점 좌측으로 우측 정렬
                            let text_pos = egui::pos2(row_rect.max.x - 24.0, row_rect.center().y);
                            if is_this_hovered {
                                // 마우스 오버 시 채널 글자 순백색(White) 강조
                                painter.text(
                                    text_pos,
                                    egui::Align2::RIGHT_CENTER,
                                    &label_text,
                                    egui::FontId::proportional(12.5),
                                    Color32::WHITE,
                                );
                            } else {
                                let text_c = if is_hidden {
                                    Color32::from_rgb(115, 115, 120)
                                } else {
                                    Color32::from_rgb(215, 218, 225)
                                };
                                painter.text(
                                    text_pos,
                                    egui::Align2::RIGHT_CENTER,
                                    &label_text,
                                    egui::FontId::proportional(12.0),
                                    text_c,
                                );
                            }
                        }
                    });
                });
        });
    }

    // 마우스 호버 상태 변경 시 즉시 다시 렌더링 요청하여 부드럽고 지연 없는 하이라이트 제공
    if config.hovered_channel != newly_hovered {
        config.hovered_channel = newly_hovered;
        ui.ctx().request_repaint();
    }
}
