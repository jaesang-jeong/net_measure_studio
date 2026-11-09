use crate::gui::channel_table::render_channel_panels;
use crate::gui::modals::channel_select_modal::{
    render_channel_select_modal, ChannelSelectModalState, ChannelSelectTarget,
};
use crate::gui::modals::config_modal::{render_config_modal, ConfigModalState};
use crate::gui::modals::connection_modal::{render_connection_modal, ConnectionModalState};
use crate::gui::modals::sftp_modal::{render_sftp_modal, SftpModalState};
use crate::gui::modals::system_modal::{render_system_modal, SystemModalState};
use crate::gui::modals::y_scale_modal::{render_y_scale_modal, YScaleModalState};
use crate::gui::page_tabs::render_page_tabs;
use crate::gui::plots::render_dual_or_single_plots;
use crate::gui::status_bar::{render_status_bar, StatusBarState};
use crate::gui::top_menu::{render_top_menu, TopMenuActions};
use crate::model::page::{PageLayoutConfig, resolve_layout_config_path};
use crate::model::target::{TargetDeviceConfig, TargetDeviceRuntime};
use crate::network::receiver::NetworkReceiver;
use crate::processing::engine::{ProcessingEngine, ProcessingEngineShared};
use crate::simulator::target_sim::{TargetSimulator, TargetSimulatorConfig};
use crossbeam_channel::bounded;
use eframe::egui::{self, CentralPanel, SidePanel, TopBottomPanel};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct MeasurementStudioApp {
    pub processing_shared: ProcessingEngineShared,
    pub pages: Vec<PageLayoutConfig>,
    pub current_page: usize,
    pub targets: Vec<TargetDeviceConfig>,
    pub config_path: PathBuf,
    pub targets_config_path: PathBuf,

    // GUI component states
    pub menu_actions: TopMenuActions,
    pub conn_modal: ConnectionModalState,
    pub cfg_modal: ConfigModalState,
    pub sftp_modal: SftpModalState,
    pub sys_modal: SystemModalState,
    pub select_modal: ChannelSelectModalState,
    pub y_scale_modal: YScaleModalState,
    pub status_bar_state: StatusBarState,

    pub save_notification: Option<(String, Instant)>,

    // CRC-16 검증 On/Off 옵션 (원격 패킷 무결성 검증 토글)
    pub verify_crc: std::sync::Arc<std::sync::atomic::AtomicBool>,

    // Network & Engine handles
    _receiver: NetworkReceiver,
    pub simulator: TargetSimulator,
}

impl MeasurementStudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // [한글 폰트 설정] Windows 시스템 한글 폰트(맑은 고딕 / 굴림 등) 등록
        setup_korean_fonts(&cc.egui_ctx);

        // 레이아웃 영구 저장 파일 경로 자동 해석
        let config_path = resolve_layout_config_path();
        // 저장된 페이지 레이아웃 불러오기 (없으면 기본값 생성)
        let pages = PageLayoutConfig::load_or_default(&config_path);

        // Channel for measurement packets (bounded buffer for zero allocation streaming)
        let (packet_tx, packet_rx) = bounded(4096);

        // CRC-16 검증 옵션 (기본 True)
        let verify_crc = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

        // 타겟 설정 파일 경로 해석 및 마지막 설정 로드 (없으면 기본값 생성)
        let targets_config_path = crate::model::target::resolve_target_config_path();
        let targets = TargetDeviceConfig::load_or_default(&targets_config_path);

        // 등록된 타겟들이 사용하는 총 채널 수 계산 (기본 50, 최소 1)
        let total_channels = targets
            .iter()
            .map(|t| t.channel_offset + t.channel_count)
            .max()
            .unwrap_or(50)
            .max(1);

        // 1. Initialize Processing Engine (타겟 설정에 맞춘 정확한 채널 수로 초기화)
        let (engine, processing_shared) = ProcessingEngine::new(total_channels, 5000);
        engine.start(packet_rx);

        // 2. Initialize Network Receiver (UDP on 8000, TCP on 8001)
        let mut receiver = NetworkReceiver::new(packet_tx, std::sync::Arc::clone(&verify_crc));
        if let Err(e) = receiver.start("0.0.0.0:8000", "0.0.0.0:8001") {
            eprintln!("Warning: Failed to bind network ports: {}", e);
        }

        // 3. Register loaded Target Devices
        {
            let mut guard = processing_shared.targets.write().unwrap();
            for target in &targets {
                guard.insert(
                    target.device_id,
                    TargetDeviceRuntime::new(target.clone()),
                );
            }
        }

        // 4. Initialize Built-in Simulator (기본 정지 상태 유지 및 저장된 전송주기/프로토콜 적용)
        let mut sim_cfg = TargetSimulatorConfig::default();
        if let Some(first_target) = targets.first() {
            sim_cfg.interval_ms = first_target.packet_interval_ms as u64;
            sim_cfg.transport = first_target.transport;
        }
        let simulator = TargetSimulator::new(sim_cfg);

        Self {
            processing_shared,
            pages,
            current_page: 0,
            targets,
            config_path,
            targets_config_path,
            menu_actions: TopMenuActions::default(),
            conn_modal: ConnectionModalState::default(),
            cfg_modal: ConfigModalState::default(),
            sftp_modal: SftpModalState::default(),
            sys_modal: SystemModalState::default(),
            select_modal: ChannelSelectModalState::default(),
            y_scale_modal: YScaleModalState::default(),
            status_bar_state: StatusBarState::default(),
            save_notification: None,
            verify_crc,
            _receiver: receiver,
            simulator,
        }
    }

    /// 페이지 레이아웃 및 타겟 설정 변경 시 파일에 실시간 영구 자동 저장
    pub fn save_layout(&mut self) {
        if let Err(e) = PageLayoutConfig::save_to_file(&self.config_path, &self.pages) {
            eprintln!("Failed to save layout: {}", e);
        }
        if let Err(e) = TargetDeviceConfig::save_to_file(&self.targets_config_path, &self.targets) {
            eprintln!("Failed to save targets config: {}", e);
        }
    }
}

/// Windows 시스템 폰트를 로드하여 egui에 한글 폰트 등록
fn setup_korean_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let font_candidates = [
        "C:\\Windows\\Fonts\\malgun.ttf",
        "C:\\Windows\\Fonts\\malgunbd.ttf",
        "C:\\Windows\\Fonts\\gulim.ttc",
        "C:\\Windows\\Fonts\\batang.ttc",
    ];

    let mut font_loaded = false;
    for path in &font_candidates {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                "korean_font".to_owned(),
                egui::FontData::from_owned(font_data),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "korean_font".to_owned());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "korean_font".to_owned());

            font_loaded = true;
            break;
        }
    }

    if font_loaded {
        ctx.set_fonts(fonts);
    }
}

impl eframe::App for MeasurementStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 타겟 장치들의 채널 범위(시작CH + 채널수)에 맞춰 채널 용량 실시간 동기화
        let needed_channels = self.targets.iter().map(|t| t.channel_offset + t.channel_count).max().unwrap_or(50).max(1);
        self.processing_shared.sync_channel_capacity(needed_channels);

        // Request smooth continuous repaint (~60 FPS) for real-time measurement waveforms
        ctx.request_repaint_after(Duration::from_millis(16));

        // 1. Top Menu Bar
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            render_top_menu(
                ui,
                &mut self.menu_actions,
                &mut self.conn_modal,
                &mut self.cfg_modal,
                &mut self.sftp_modal,
                &mut self.sys_modal,
                &self.targets,
            );

            // 저장 알림 메시지 (3초간 표시)
            if let Some((ref msg, time)) = self.save_notification {
                if time.elapsed().as_secs() < 3 {
                    ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(100, 240, 100)));
                } else {
                    self.save_notification = None;
                }
            }
        });

        // 2. Bottom Status Bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            render_status_bar(
                ui,
                &mut self.status_bar_state,
                &self.targets,
                &self.processing_shared.targets,
                &self.processing_shared.total_rx_packets,
                &self.processing_shared.total_lost_packets,
                &self.menu_actions,
            );
        });

        let mut layout_changed = false;

        // 3. Bottom Page Selector Tabs (10-Page interface)
        TopBottomPanel::bottom("page_bar").show(ctx, |ui| {
            render_page_tabs(ui, &mut self.current_page, &self.pages);
        });

        // 4. Right Side Panel: Channel Lists (Page 1: 2 Lists, Page 2~10: 1 List)
        SidePanel::right("channel_side_panel")
            .resizable(true)
            .default_width(380.0)
            .show(ctx, |ui| {
                if self.current_page < self.pages.len() {
                    let page_idx = self.current_page;
                    let page = &mut self.pages[page_idx];
                    let is_dual = page.is_dual_plot();

                    render_channel_panels(
                        ui,
                        page_idx,
                        is_dual,
                        &self.processing_shared.channels,
                        &mut page.top_channels,
                        &mut page.bottom_channels,
                        &mut page.top_graph_config,
                        &mut page.bottom_graph_config,
                        &mut self.select_modal,
                        &mut layout_changed,
                    );
                }
            });

        // 5. Central Panel: Dual Plots (Page 1) or Single Plot (Page 2~10)
        CentralPanel::default().show(ctx, |ui| {
            if self.current_page < self.pages.len() {
                let page = &mut self.pages[self.current_page];
                let is_dual = page.is_dual_plot();
                render_dual_or_single_plots(
                    ui,
                    is_dual,
                    &mut page.title,
                    &page.top_channels,
                    &page.bottom_channels,
                    &mut page.top_graph_config,
                    &mut page.bottom_graph_config,
                    &self.processing_shared.channels,
                    &self.processing_shared.ring_buffers,
                    &mut self.y_scale_modal,
                    &mut layout_changed,
                );
            }
        });

        // 6. Modals
        render_connection_modal(
            ctx,
            &mut self.conn_modal,
            &mut self.targets,
            &self.processing_shared.targets,
            &self.verify_crc,
        );
        render_config_modal(
            ctx,
            &mut self.cfg_modal,
            &self.processing_shared.channels,
            &self.targets,
        );
        render_sftp_modal(
            ctx,
            &mut self.sftp_modal,
            &mut self.targets,
            &self.targets_config_path,
        );
        render_system_modal(
            ctx,
            &mut self.sys_modal,
            &self.simulator,
            &mut self.targets,
            &self.verify_crc,
            &self.targets_config_path,
            &mut self.pages,
            &self.config_path,
        );

        // 7. Channel Select Modal
        let mut modal_apply_args = None;
        render_channel_select_modal(
            ctx,
            &mut self.select_modal,
            &self.processing_shared.channels,
            |page_idx, target, selected| {
                modal_apply_args = Some((page_idx, target, selected));
            },
        );

        if let Some((page_idx, target, selected)) = modal_apply_args {
            if page_idx < self.pages.len() {
                match target {
                    ChannelSelectTarget::TopGraph | ChannelSelectTarget::SingleGraph => {
                        self.pages[page_idx].top_channels = selected;
                    }
                    ChannelSelectTarget::BottomGraph => {
                        self.pages[page_idx].bottom_channels = selected;
                    }
                }
                layout_changed = true;
            }
        }

        // 8. Y-Scale Settings Modal
        let current_page_idx = self.current_page;
        let page_ref = &mut self.pages[current_page_idx];
        render_y_scale_modal(
            ctx,
            &mut self.y_scale_modal,
            &self.processing_shared.channels,
            |limits| {
                for &(ch_idx, min, max) in limits {
                    page_ref.top_graph_config.set_channel_y_limit(ch_idx, min, max);
                    page_ref.bottom_graph_config.set_channel_y_limit(ch_idx, min, max);
                }
                layout_changed = true;
            },
        );

        // 레이아웃 변경 시 자동 영구 저장
        if layout_changed {
            self.save_layout();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_layout();
    }
}
