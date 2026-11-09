use crate::model::page::PageLayoutConfig;
use crate::model::target::{DataTransportProtocol, TargetDeviceConfig};
use crate::simulator::target_sim::TargetSimulator;
use eframe::egui::{self, Color32, RichText, Window};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct SystemModalState {
    pub is_open: bool,
}

impl Default for SystemModalState {
    fn default() -> Self {
        Self { is_open: false }
    }
}

pub fn render_system_modal(
    ctx: &egui::Context,
    state: &mut SystemModalState,
    simulator: &TargetSimulator,
    targets: &mut [TargetDeviceConfig],
    verify_crc: &Arc<AtomicBool>,
    targets_config_path: &Path,
    pages: &mut [PageLayoutConfig],
    layout_config_path: &Path,
) {
    if !state.is_open {
        return;
    }

    Window::new("🛠 시스템 설정 (System Configuration)")
        .collapsible(false)
        .resizable([true, true])
        .default_size([660.0, 680.0])
        .min_width(620.0)
        .show(ctx, |ui| {
            ui.heading("시스템 운영 및 통신 제어");
            ui.add_space(4.0);
            ui.label(
                RichText::new("내장 시뮬레이터, 계측 통신 프로토콜, 패킷 검증 및 페이지 탭 설정을 제어합니다.")
                    .color(Color32::from_rgb(120, 190, 255))
                    .small(),
            );
            ui.add_space(10.0);

            // 1. 내장 시뮬레이터 제어
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(RichText::new("🛰 내장 타겟 시뮬레이터 (Virtual Generator)").color(Color32::from_rgb(100, 220, 255)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let is_running = simulator.is_running();
                        if is_running {
                            if ui.button(RichText::new("⏹ 시뮬레이터 정지").color(Color32::from_rgb(255, 120, 120))).clicked() {
                                simulator.stop();
                            }
                        } else {
                            if ui.button(RichText::new("▶ 시뮬레이터 시작").color(Color32::from_rgb(100, 240, 100))).clicked() {
                                simulator.start();
                            }
                        }
                    });
                });
                ui.separator();

                let is_running = simulator.is_running();
                let sim_interval = simulator.get_interval_ms();
                let sim_hz = if sim_interval > 0 { 1000.0 / sim_interval as f64 } else { 0.0 };
                ui.horizontal(|ui| {
                    ui.label("현재 동작 상태:");
                    if is_running {
                        ui.label(RichText::new(format!("🟢 동작 중 ({}ms / {:.1}Hz 주기 가상 계측 데이터 송신)", sim_interval, sim_hz)).color(Color32::GREEN).strong());
                    } else {
                        ui.label(RichText::new("⚪ 정지됨 (외부 타겟 장비 데이터 수신 대기)").color(Color32::LIGHT_GRAY).strong());
                    }
                });

                ui.add_space(3.0);
                ui.label(
                    RichText::new("※ 실제 타겟 장비가 없을 때 내부에서 가상 신호(전압, 전류, 온도 등)를 합성하여 차트 및 GUI 기능을 테스트할 수 있습니다.\n※ 외부 장비 연결 시에는 시뮬레이터를 정지 상태로 유지하는 것을 권장합니다.")
                        .small()
                        .color(Color32::GRAY),
                );
            });

            ui.add_space(10.0);

            // 2. 패킷 전송 속도 및 주기 제어 (Packet Transmission Rate)
            ui.group(|ui| {
                ui.strong(RichText::new("⏱ 패킷 전송 속도 및 주기 (Transmission Rate)").color(Color32::from_rgb(100, 220, 255)));
                ui.separator();

                let cur_interval = targets.first().map(|t| t.packet_interval_ms).unwrap_or_else(|| simulator.get_interval_ms() as u32);
                let mut edit_interval = cur_interval;
                let mut interval_changed = false;

                ui.horizontal(|ui| {
                    ui.label("전송 주기:");
                    let drag = egui::DragValue::new(&mut edit_interval)
                        .range(1..=10000)
                        .speed(1.0)
                        .suffix(" ms");
                    if ui.add(drag).changed() {
                        interval_changed = true;
                    }

                    let hz = if edit_interval > 0 { 1000.0 / (edit_interval as f64) } else { 0.0 };
                    ui.label(RichText::new(format!("({:.1} Hz / 초당 {:.1}회 전송)", hz, hz)).color(Color32::from_rgb(100, 240, 100)).strong());
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("빠른 선택:");
                    let presets = [
                        (10, "10ms (100Hz)"),
                        (20, "20ms (50Hz)"),
                        (50, "50ms (20Hz)"),
                        (100, "100ms (10Hz)"),
                        (200, "200ms (5Hz)"),
                        (500, "500ms (2Hz)"),
                        (1000, "1000ms (1Hz)"),
                    ];
                    for (ms, label) in presets {
                        let is_cur = edit_interval == ms;
                        if ui.selectable_label(is_cur, label).clicked() {
                            edit_interval = ms;
                            interval_changed = true;
                        }
                    }
                });

                if interval_changed {
                    edit_interval = edit_interval.clamp(1, 10000);
                    simulator.set_interval_ms(edit_interval as u64);
                    for t in targets.iter_mut() {
                        t.packet_interval_ms = edit_interval;
                    }
                    let _ = TargetDeviceConfig::save_to_file(targets_config_path, targets);
                }

                ui.add_space(3.0);
                ui.label(
                    RichText::new("※ 기본값은 100ms(10Hz)이며 설정값은 targets_config.json에 영구 저장됩니다.\n※ 내장 시뮬레이터 송신 및 타겟 장비 제어 주기에 즉시 반영됩니다.")
                        .small()
                        .color(Color32::GRAY),
                );
            });

            ui.add_space(10.0);

            // 3. 통신 프로토콜 모드 (UDP vs TCP)
            ui.group(|ui| {
                ui.strong(RichText::new("🌐 계측 데이터 통신 프로토콜 (Transport Mode)").color(Color32::from_rgb(100, 220, 255)));
                ui.separator();

                let mut cur_proto = targets.first().map(|t| t.transport).unwrap_or(DataTransportProtocol::Udp);
                let mut proto_changed = false;

                ui.horizontal(|ui| {
                    if ui.radio_value(&mut cur_proto, DataTransportProtocol::Udp, "UDP (표준 고속 전송)").clicked() {
                        proto_changed = true;
                    }
                    ui.add_space(16.0);
                    if ui.radio_value(&mut cur_proto, DataTransportProtocol::Tcp, "TCP (대용량 MTU초과 대응 프레이밍)").clicked() {
                        proto_changed = true;
                    }
                });

                if proto_changed {
                    simulator.set_transport(cur_proto);
                    for t in targets.iter_mut() {
                        t.transport = cur_proto;
                    }
                    let _ = TargetDeviceConfig::save_to_file(targets_config_path, targets);
                }

                ui.add_space(4.0);
                match cur_proto {
                    DataTransportProtocol::Udp => {
                        ui.label(
                            RichText::new("• UDP 모드: 비연결형 고속 전송 (기본 포트: UDP 8000). 100채널 이하 표준 환경에 최적화되어 있습니다.")
                                .small()
                                .color(Color32::from_rgb(180, 220, 255)),
                        );
                    }
                    DataTransportProtocol::Tcp => {
                        ui.label(
                            RichText::new("• TCP 모드: 연결 지향형 신뢰성 전송 (기본 포트: TCP 8001). 100~200채널 이상 패킷 크기가 1.5KB MTU를 초과할 때 패킷 분할 손실을 방지합니다.")
                                .small()
                                .color(Color32::from_rgb(180, 220, 255)),
                        );
                    }
                }
            });

            ui.add_space(10.0);

            // 4. CRC-16 검증 옵션
            ui.group(|ui| {
                ui.strong(RichText::new("🛡 수신 패킷 무결성 검증 (CRC-16 Check)").color(Color32::from_rgb(100, 220, 255)));
                ui.separator();

                let mut cur_verify = verify_crc.load(Ordering::Relaxed);
                if ui
                    .checkbox(&mut cur_verify, "수신 패킷 CRC-16 무결성 검증 활성화")
                    .on_hover_text("체크 시 패킷 끝 2바이트 CRC-16을 계산하여 일치하지 않는 패킷을 자동 폐기합니다.")
                    .changed()
                {
                    verify_crc.store(cur_verify, Ordering::Relaxed);
                }

                ui.add_space(3.0);
                if cur_verify {
                    ui.label(RichText::new("✔ 활성화됨: 전송 중 데이터 오류 및 손상 패킷을 완벽히 차단합니다.").color(Color32::from_rgb(100, 240, 100)).small());
                } else {
                    ui.label(RichText::new("⚠ 우회됨: CRC 오류를 무시하고 강제 수신합니다. (타사 장비 또는 랩뷰 엔디안 차이 점검 시 유용)").color(Color32::from_rgb(255, 180, 80)).small());
                }
            });

            ui.add_space(10.0);

            // 5. 페이지별 탭 이름 설정 (Page Tab Names)
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(RichText::new("📑 페이지별 탭 이름 설정 (Page Tab Names)").color(Color32::from_rgb(100, 220, 255)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("↺ 기본 탭 이름으로 초기화")
                            .on_hover_text("모든 페이지 탭 이름을 기본값(메인 종합, Page 2~10)으로 초기화합니다.")
                            .clicked()
                        {
                            for (i, p) in pages.iter_mut().enumerate() {
                                p.tab_name = if i == 0 {
                                    "메인 종합".to_string()
                                } else {
                                    format!("Page {}", i + 1)
                                };
                            }
                            let _ = PageLayoutConfig::save_to_file(layout_config_path, pages);
                        }
                    });
                });
                ui.separator();
                ui.label(
                    RichText::new("하단 페이지 선택 바에 표시되는 10개 탭의 이름을 사용자 정의할 수 있습니다. (layout_config.json에 자동 저장)")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.add_space(4.0);

                let mut tab_changed = false;
                ui.columns(2, |columns| {
                    // 왼쪽 컬럼: Page 1 ~ Page 5
                    for i in 0..5 {
                        if i < pages.len() {
                            columns[0].horizontal(|ui| {
                                ui.label(RichText::new(format!("Page {:02}:", i + 1)).strong().color(Color32::from_rgb(180, 200, 220)));
                                let edit = egui::TextEdit::singleline(&mut pages[i].tab_name)
                                    .desired_width(170.0)
                                    .hint_text(format!("Page {}", i + 1));
                                if ui.add(edit).changed() {
                                    tab_changed = true;
                                }
                            });
                        }
                    }
                    // 오른쪽 컬럼: Page 6 ~ Page 10
                    for i in 5..10 {
                        if i < pages.len() {
                            columns[1].horizontal(|ui| {
                                ui.label(RichText::new(format!("Page {:02}:", i + 1)).strong().color(Color32::from_rgb(180, 200, 220)));
                                let edit = egui::TextEdit::singleline(&mut pages[i].tab_name)
                                    .desired_width(170.0)
                                    .hint_text(format!("Page {}", i + 1));
                                if ui.add(edit).changed() {
                                    tab_changed = true;
                                }
                            });
                        }
                    }
                });

                if tab_changed {
                    let _ = PageLayoutConfig::save_to_file(layout_config_path, pages);
                }
            });

            ui.add_space(10.0);

            // 6. 레이아웃 및 설정 자동 저장 상태
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(RichText::new("💾 레이아웃 및 환경설정").color(Color32::from_rgb(100, 220, 255)));
                    ui.label(RichText::new("✔ 마지막 상태 상시 자동 저장 활성화").color(Color32::from_rgb(100, 240, 100)).small());
                });
                ui.label(
                    RichText::new("※ 페이지별 그래프 채널 배치, Y스케일 리밋, 시간 윈도우 및 타겟 장치 연결 설정이 변경될 때마다 파일에 실시간 영구 저장됩니다.")
                        .small()
                        .color(Color32::GRAY),
                );
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
                });
            });
        });
}
