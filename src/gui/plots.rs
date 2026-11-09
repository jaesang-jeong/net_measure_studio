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
    page_title: &mut String,
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
            let default_title = if !page_title.trim().is_empty() {
                page_title.as_str()
            } else {
                "📊 [단일 대형 그래프]"
            };
            render_graph_control_bar(
                ui,
                default_title,
                top_config,
                top_channels,
                &channels,
                y_scale_modal,
                layout_changed,
            );

            // 단일 그래프 타이틀이 변경된 경우 페이지 전체 title과 동기화
            if !top_config.title.trim().is_empty() && top_config.title != *page_title {
                *page_title = top_config.title.clone();
                *layout_changed = true;
            }

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
    default_title: &str,
    config: &mut GraphDisplayConfig,
    active_channels: &[usize],
    channels: &[ChannelState],
    y_scale_modal: &mut YScaleModalState,
    layout_changed: &mut bool,
) {
    ui.horizontal(|ui| {
        // 그래프 타이틀: 평상시 레이블 표시, 더블클릭 시 인라인 텍스트필드로 전환하여 수정
        if config.is_editing_title {
            let edit_resp = ui.add(
                egui::TextEdit::singleline(&mut config.title)
                    .desired_width(220.0)
                    .hint_text(default_title),
            );

            if !edit_resp.has_focus() {
                edit_resp.request_focus();
            }

            if edit_resp.changed() {
                *layout_changed = true;
            }

            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let esc_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let btn_confirm = ui
                .button(egui::RichText::new("✔").color(Color32::from_rgb(100, 240, 100)))
                .on_hover_text("타이틀 수정 완료 (Enter)")
                .clicked();

            if enter_pressed || btn_confirm || (edit_resp.lost_focus() && !edit_resp.has_focus()) {
                config.title = config.title.trim().to_string();
                if config.title.is_empty() {
                    config.title = default_title.to_string();
                }
                config.is_editing_title = false;
                *layout_changed = true;
            } else if esc_pressed {
                config.is_editing_title = false;
            }
        } else {
            let display_title = if !config.title.trim().is_empty() {
                &config.title
            } else {
                default_title
            };

            let title_label = egui::Label::new(
                egui::RichText::new(display_title)
                    .strong()
                    .color(Color32::from_rgb(220, 235, 255)),
            )
            .sense(egui::Sense::click());

            let label_resp = ui
                .add(title_label)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("💡 더블클릭하여 그래프 타이틀을 변경할 수 있습니다.");

            if label_resp.double_clicked() {
                if config.title.trim().is_empty() {
                    config.title = default_title.to_string();
                }
                config.is_editing_title = true;
            }
        }

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

        // 4. 채널별 Y스케일 사용자 지정 팝업 버튼 (상시 노출하여 첫번째 탭 상단/하단 및 모든 페이지에서 즉시 설정 가능)
        let current_title = if !config.title.trim().is_empty() {
            &config.title
        } else {
            default_title
        };
        if ui
            .button("⚙ 채널별 Y스케일 설정")
            .on_hover_text("채널별 Y축 상한/하한값 커스텀 수동 입력 및 스케일 조정 (클릭 시 다중 Y축 오버레이 모드 활성화)")
            .clicked()
        {
            if !config.multi_y_overlay {
                config.multi_y_overlay = true;
                *layout_changed = true;
            }
            y_scale_modal.open_for(
                &format!("{} 채널별 Y스케일 설정", current_title),
                active_channels,
                config,
                channels,
            );
        }
    });
}

/// "Nice numbers" 알고리즘: Y축 눈금과 경계가 프레임마다 소수점으로 떨리지 않도록
/// 1, 2, 5 * 10^k 단위의 안정적인 정수/소수 단위로 라운딩하여 상하단 범위를 스냅
fn compute_nice_bounds(raw_min: f64, raw_max: f64) -> (f64, f64) {
    if !raw_min.is_finite() || !raw_max.is_finite() || raw_min >= raw_max {
        return (-10.0, 10.0);
    }
    let span = raw_max - raw_min;
    if span <= 1e-6 {
        return (raw_min - 1.0, raw_max + 1.0);
    }

    // 5~8개 정도의 깔끔한 눈금 간격 생성
    let rough_step = span / 6.0;
    let exponent = 10.0_f64.powf(rough_step.log10().floor());
    let fraction = rough_step / exponent;
    let nice_fraction = if fraction < 1.5 {
        1.0
    } else if fraction < 3.0 {
        2.0
    } else if fraction < 7.0 {
        5.0
    } else {
        10.0
    };
    let step = nice_fraction * exponent;

    let nice_min = (raw_min / step).floor() * step;
    let nice_max = (raw_max / step).ceil() * step;

    if nice_max <= nice_min {
        (nice_min - step, nice_max + step)
    } else {
        (nice_min, nice_max)
    }
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

    let window_start = latest_time - window_secs;

    // 2. [다중 Y축 모드]: 가시 채널들을 '단위(Unit)'별로 그룹화하여 단위별 스케일(Min/Max) 산출
    struct UnitScaleGroup {
        unit: String,
        color: Color32,
        channels: Vec<usize>,
        min_val: f64,
        max_val: f64,
    }

    let mut unit_groups: Vec<UnitScaleGroup> = Vec::new();
    if config.multi_y_overlay {
        for &ch_idx in channels_to_plot {
            if config.is_channel_hidden(ch_idx) || ch_idx >= channels.len() {
                continue;
            }
            let ch = &channels[ch_idx];
            let unit_str = ch.config.unit.trim().to_string();
            let color = COLOR_PALETTE[channels_to_plot.iter().position(|&x| x == ch_idx).unwrap_or(0) % COLOR_PALETTE.len()];

            if let Some(group) = unit_groups.iter_mut().find(|g| g.unit == unit_str) {
                group.channels.push(ch_idx);
            } else {
                unit_groups.push(UnitScaleGroup {
                    unit: unit_str,
                    color,
                    channels: vec![ch_idx],
                    min_val: f64::MAX,
                    max_val: f64::MIN,
                });
            }
        }

        // 단위별 Min / Max 계산 (상하 과다 마진 없이 데이터 전 구간 꽉 채움)
        for group in &mut unit_groups {
            let mut g_min = f64::MAX;
            let mut g_max = f64::MIN;
            let mut has_pts = false;

            if config.auto_scale_y {
                for &ch_idx in &group.channels {
                    if ch_idx < ring_buffers.len() {
                        let pts = ring_buffers[ch_idx].get_points();
                        for pt in &pts {
                            if pt[0] >= window_start - 0.1 && pt[0] <= latest_time + 0.1 {
                                let v = pt[1];
                                if v.is_finite() {
                                    if v < g_min { g_min = v; }
                                    if v > g_max { g_max = v; }
                                    has_pts = true;
                                }
                            }
                        }
                        if !has_pts {
                            if let Some(last) = pts.last() {
                                let v = last[1];
                                if v.is_finite() {
                                    if v < g_min { g_min = v; }
                                    if v > g_max { g_max = v; }
                                    has_pts = true;
                                }
                            }
                        }
                    }
                }
            } else {
                // 수동 지정 모드: channel_y_limits 참조
                for &ch_idx in &group.channels {
                    if let Some(&(cmin, cmax)) = config.channel_y_limits.get(&ch_idx) {
                        if cmin < g_min { g_min = cmin; }
                        if cmax > g_max { g_max = cmax; }
                        has_pts = true;
                    }
                }
            }

            // 데이터가 없거나 유효하지 않은 경우 알람 임계값 참조
            if !has_pts || g_min >= g_max {
                for &ch_idx in &group.channels {
                    if ch_idx < channels.len() {
                        let low = channels[ch_idx].config.alarm_low as f64;
                        let high = channels[ch_idx].config.alarm_high as f64;
                        if low < g_min { g_min = low; }
                        if high > g_max { g_max = high; }
                    }
                }
            }

            if g_min >= g_max || !g_min.is_finite() || !g_max.is_finite() {
                g_min = -10.0;
                g_max = 10.0;
            } else if (g_max - g_min).abs() < 1e-4 {
                let center = g_min;
                let half = if center.abs() > 1e-3 { (center.abs() * 0.05).max(0.5) } else { 0.5 };
                g_min = center - half;
                g_max = center + half;
            }

            group.min_val = g_min;
            group.max_val = g_max;
        }
    }

    // 3. Y축 Bounds 계산
    let (plot_y_min, plot_y_max) = if config.multi_y_overlay {
        // 다중 Y축 모드: 상하 마진(기존 -5~105)을 완전히 제거하고 0.0 ~ 100.0%로 전 구간 꽉 채움!
        (0.0, 100.0)
    } else if config.auto_scale_y {
        // [단일 Y축 Auto-Scale]: 가시 윈도우 스캔 후 상하 불필요한 과다 마진 제거
        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;
        let mut has_points = false;

        for &ch_idx in channels_to_plot {
            if config.is_channel_hidden(ch_idx) || ch_idx >= ring_buffers.len() {
                continue;
            }
            let pts = ring_buffers[ch_idx].get_points();
            for pt in &pts {
                if pt[0] >= window_start - 0.1 && pt[0] <= latest_time + 0.1 {
                    let v = pt[1];
                    if v.is_finite() {
                        if v < min_val { min_val = v; }
                        if v > max_val { max_val = v; }
                        has_points = true;
                    }
                }
            }
            if !has_points {
                if let Some(last) = pts.last() {
                    let v = last[1];
                    if v.is_finite() {
                        if v < min_val { min_val = v; }
                        if v > max_val { max_val = v; }
                        has_points = true;
                    }
                }
            }
        }

        if !has_points || min_val == f64::MAX || max_val == f64::MIN {
            (-10.0, 10.0)
        } else if (max_val - min_val).abs() < 1e-4 {
            let center = min_val;
            let half = if center.abs() > 1e-3 {
                (center.abs() * 0.05).max(0.5)
            } else {
                0.5
            };
            compute_nice_bounds(center - half, center + half)
        } else {
            // 상하 과다 마진 제거: 바로 눈금 스냅
            compute_nice_bounds(min_val, max_val)
        }
    } else {
        // 사용자 직접 지정 단일 Y스케일
        (config.custom_y_min, config.custom_y_max)
    };

    // Auto-Y가 켜져 있을 때 현재 계산된 bounds를 custom_y_min/max에 동기화
    if config.auto_scale_y && !config.multi_y_overlay {
        config.custom_y_min = plot_y_min;
        config.custom_y_max = plot_y_max;
    }

    // 다중 Y축 오버레이 모드에서는 0~100 일괄 축을 숨기고 단위별 전용 Y축 스케일을 직접 렌더링
    let show_default_y_axis = !config.multi_y_overlay || unit_groups.is_empty();

    let num_units = if config.multi_y_overlay { unit_groups.len() } else { 0 };
    let col_width = 48.0_f32;
    let col_gap = 3.0_f32;
    let total_axis_width = if num_units > 0 {
        (num_units as f32) * (col_width + col_gap) + 2.0
    } else {
        0.0
    };

    let mut newly_hovered: Option<usize> = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // 1. [좌측 전용 Y축 영역 할당]: 그래프 캔버스 바깥 좌측에 배치하여 파형과 툴바를 절대 침범하지 않음
        let axis_rect = if total_axis_width > 0.0 {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(total_axis_width, height),
                egui::Sense::hover(),
            );
            Some(rect)
        } else {
            None
        };

        // 2. [우측 플롯 캔버스]: Y축 스케일 영역 다음 위치에서 남은 가용 폭 100%를 채움
        let plot_width = (ui.available_width() - 2.0).max(100.0);
        let plot = Plot::new(plot_id)
            .height(height)
            .width(plot_width)
            .coordinates_formatter(Corner::RightBottom, CoordinatesFormatter::default())
            .show_axes([true, show_default_y_axis])
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

                // 다중 Y축 모드일 때는 채널의 단위 그룹의 (min, max)를 기준으로 0~100% 정규화
                let (ch_ymin, ch_ymax) = if config.multi_y_overlay {
                    let ch_unit = ch_info.config.unit.trim();
                    if let Some(grp) = unit_groups.iter().find(|g| g.unit == ch_unit) {
                        (grp.min_val, grp.max_val)
                    } else {
                        (ch_info.config.alarm_low as f64, ch_info.config.alarm_high as f64)
                    }
                } else if let Some(&(min, max)) = config.channel_y_limits.get(&ch_idx) {
                    (min, max)
                } else {
                    config.get_channel_y_limit(
                        ch_idx,
                        ch_info.config.alarm_low as f64,
                        ch_info.config.alarm_high as f64,
                    )
                };

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

        // 3. [다중 Y축 스케일]: 플롯 바깥 좌측에 완벽히 분리된 axis_rect 영역 내부에 단위별 스케일 렌더링
        if let Some(axis_box) = axis_rect {
            let p_bottom = plot_resp.transform.position_from_point(&egui_plot::PlotPoint::new(0.0, 0.0));
            let p_top = plot_resp.transform.position_from_point(&egui_plot::PlotPoint::new(0.0, 100.0));

            let grid_top_y = p_top.y;
            let grid_bottom_y = p_bottom.y;
            let grid_height = (grid_bottom_y - grid_top_y).max(10.0);

            let painter = ui.painter();

            for (u_idx, group) in unit_groups.iter().enumerate() {
                let col_x = axis_box.min.x + (u_idx as f32) * (col_width + col_gap);
                // 툴바 및 상하 경계를 벗어나지 않도록 axis_box 내부에 클램핑
                let col_top = (grid_top_y - 14.0).max(axis_box.min.y + 2.0);
                let col_bottom = (grid_bottom_y + 2.0).min(axis_box.max.y - 2.0);
                let col_rect = egui::Rect::from_min_max(
                    egui::pos2(col_x, col_top),
                    egui::pos2(col_x + col_width, col_bottom),
                );

                // 반투명 다크 배경 패널
                painter.rect_filled(
                    col_rect,
                    3.0,
                    Color32::from_rgba_premultiplied(16, 18, 24, 230),
                );
                painter.rect_stroke(
                    col_rect,
                    3.0,
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(50, 55, 65)),
                );

                // 상단 단위(Unit) 헤더 배지
                let header_str = if group.unit.is_empty() { "단위" } else { &group.unit };
                painter.text(
                    egui::pos2(col_rect.center().x, col_rect.top() + 7.5),
                    egui::Align2::CENTER_CENTER,
                    format!("[{}]", header_str),
                    egui::FontId::proportional(11.0),
                    group.color,
                );

                // 우측 수직 Y축 라인
                let axis_line_x = col_rect.right() - 4.0;
                painter.line_segment(
                    [egui::pos2(axis_line_x, grid_top_y), egui::pos2(axis_line_x, grid_bottom_y)],
                    egui::Stroke::new(1.2_f32, group.color),
                );

                // 5개 틱 레벨 (100%, 75%, 50%, 25%, 0%)
                let span = group.max_val - group.min_val;
                let tick_fracs = [1.00_f64, 0.75_f64, 0.50_f64, 0.25_f64, 0.00_f64];

                for &frac in &tick_fracs {
                    let tick_y = grid_bottom_y - (grid_height * (frac as f32));

                    // 틱 마크 선
                    painter.line_segment(
                        [egui::pos2(axis_line_x - 3.0, tick_y), egui::pos2(axis_line_x, tick_y)],
                        egui::Stroke::new(1.0_f32, group.color),
                    );

                    // 틱 수치 텍스트 포맷 (오른쪽 정렬)
                    let val = group.min_val + span * frac;
                    let val_str = if span.abs() >= 100.0 {
                        format!("{:.0}", val)
                    } else if span.abs() >= 10.0 {
                        format!("{:.1}", val)
                    } else if span.abs() >= 1.0 {
                        format!("{:.2}", val)
                    } else {
                        format!("{:.3}", val)
                    };

                    painter.text(
                        egui::pos2(axis_line_x - 4.5, tick_y),
                        egui::Align2::RIGHT_CENTER,
                        val_str,
                        egui::FontId::monospace(9.0),
                        Color32::from_rgb(230, 235, 245),
                    );
                }

                // 마우스 호버 툴팁
                let col_resp = ui.interact(col_rect, ui.id().with(format!("{}_unit_axis_{}", plot_id, u_idx)), egui::Sense::hover());
                let mut tooltip = format!("단위: [{}]\n스케일: {:.2} ~ {:.2}\n속한 채널 목록:", header_str, group.min_val, group.max_val);
                for &c_idx in &group.channels {
                    if c_idx < channels.len() {
                        tooltip.push_str(&format!("\n• {}", channels[c_idx].config.name));
                    }
                }
                col_resp.on_hover_text(tooltip);
            }
        }

        // 4. [플롯 범례]: 플롯 캔버스 내부 우측 상단에 배치
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

    });

    // 마우스 호버 상태 변경 시 즉시 다시 렌더링 요청하여 부드럽고 지연 없는 하이라이트 제공
    if config.hovered_channel != newly_hovered {
        config.hovered_channel = newly_hovered;
        ui.ctx().request_repaint();
    }
}
