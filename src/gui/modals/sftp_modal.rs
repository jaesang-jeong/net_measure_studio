use crate::gui::modals::config_modal::open_path_in_explorer;
use crate::model::target::TargetDeviceConfig;
use crate::network::file_transfer::{
    FileTransferManager, FileTransferProgress, FileTransferStatus, RemoteFileInfo,
};
use eframe::egui::{self, Color32, ProgressBar, RichText, Window};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct SftpModalState {
    pub is_open: bool,
    pub selected_target_idx: usize,
    pub show_password: bool,
    pub remote_files: Vec<RemoteFileInfo>,
    pub selected_files: Vec<bool>,
    pub delete_remote_after: bool,
    pub transfer_manager: FileTransferManager,
    pub progress: Arc<Mutex<Option<FileTransferProgress>>>,
    pub status_msg: Option<(String, bool)>, // (message, is_error)

    // 파일 목록 비동기 새로고침 상태
    pub is_refreshing_files: bool,
    pub file_refresh_result: Arc<Mutex<Option<Result<Vec<RemoteFileInfo>, String>>>>,

    // 입력값 리셋 방지 및 영구 저장을 위한 편집 버퍼
    pub buffer_loaded_idx: Option<usize>,
    pub edit_ip: String,
    pub edit_port: String,
    pub edit_user: String,
    pub edit_password: String,
    pub edit_remote_dir: String,
    pub edit_local_dir: String,
}

impl SftpModalState {
    /// 타겟 장비의 저장된 SFTP 설정을 입력 버퍼로 즉시 로드
    pub fn load_target_to_buffer(&mut self, target: &TargetDeviceConfig, target_idx: usize) {
        self.edit_ip = target.get_sftp_ip().to_string();
        self.edit_port = target.sftp_port.to_string();
        self.edit_user = target.sftp_user.clone();
        self.edit_password = target.sftp_password.clone();
        self.edit_remote_dir = target.remote_dir.clone();
        self.edit_local_dir = target.local_dir.clone();
        self.buffer_loaded_idx = Some(target_idx);
        self.remote_files.clear();
        self.selected_files.clear();
        self.status_msg = None;
    }

    /// 현재 편집 버퍼의 값을 대상 타겟 구조체로 동기화 (Data IP는 보존하고 SFTP 전용 IP만 업데이트)
    pub fn sync_buffer_to_target(&self, target: &mut TargetDeviceConfig) {
        if let Ok(ip) = self.edit_ip.trim().parse::<std::net::IpAddr>() {
            target.sftp_ip = Some(ip);
        }
        if let Ok(p) = self.edit_port.trim().parse::<u16>() {
            target.sftp_port = p;
        }
        target.sftp_user = self.edit_user.clone();
        target.sftp_password = self.edit_password.clone();
        target.remote_dir = self.edit_remote_dir.clone();
        target.local_dir = self.edit_local_dir.clone();
    }
}

impl Default for SftpModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_target_idx: 0,
            show_password: false,
            remote_files: Vec::new(),
            selected_files: Vec::new(),
            delete_remote_after: false,
            transfer_manager: FileTransferManager::new(),
            progress: Arc::new(Mutex::new(None)),
            status_msg: None,
            is_refreshing_files: false,
            file_refresh_result: Arc::new(Mutex::new(None)),
            buffer_loaded_idx: None,
            edit_ip: String::new(),
            edit_port: String::new(),
            edit_user: String::new(),
            edit_password: String::new(),
            edit_remote_dir: String::new(),
            edit_local_dir: String::new(),
        }
    }
}

/// Windows 시스템 표준 폴더 선택 대화상자 (SHBrowseForFolderW)
#[cfg(target_os = "windows")]
pub fn prompt_folder_dialog(title: &str) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct BROWSEINFOW {
        hwndOwner: *mut std::ffi::c_void,
        pidlRoot: *mut std::ffi::c_void,
        pszDisplayName: *mut u16,
        lpszTitle: *const u16,
        ulFlags: u32,
        lpfn: *mut std::ffi::c_void,
        lParam: isize,
        iImage: i32,
    }

    #[link(name = "shell32")]
    #[link(name = "ole32")]
    extern "system" {
        fn SHBrowseForFolderW(lpbi: *mut BROWSEINFOW) -> *mut std::ffi::c_void;
        fn SHGetPathFromIDListW(pidl: *const std::ffi::c_void, pszPath: *mut u16) -> i32;
        fn CoTaskMemFree(pv: *mut std::ffi::c_void);
    }

    let title_utf16: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let mut bi: BROWSEINFOW = unsafe { std::mem::zeroed() };
    bi.lpszTitle = title_utf16.as_ptr();
    // BIF_RETURNONLYFSDIRS (0x0001) | BIF_NEWDIALOGSTYLE (0x0040)
    bi.ulFlags = 0x0001 | 0x0040;

    let pidl = unsafe { SHBrowseForFolderW(&mut bi) };
    if !pidl.is_null() {
        let mut path_buf = [0u16; 1024];
        let success = unsafe { SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr()) };
        unsafe { CoTaskMemFree(pidl) };
        if success != 0 {
            let len = path_buf.iter().position(|&c| c == 0).unwrap_or(path_buf.len());
            let os_str = std::ffi::OsString::from_wide(&path_buf[..len]);
            return Some(PathBuf::from(os_str));
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn prompt_folder_dialog(_title: &str) -> Option<PathBuf> {
    None
}

pub fn render_sftp_modal(
    ctx: &egui::Context,
    state: &mut SftpModalState,
    targets: &mut [TargetDeviceConfig],
    targets_config_path: &Path,
) {
    if !state.is_open {
        return;
    }

    Window::new("저장 파일 회수 (SFTP File Transfer)")
        .collapsible(false)
        .resizable([true, true])
        .default_size([880.0, 720.0])
        .min_width(820.0)
        .show(ctx, |ui| {
            ui.heading("타겟 장비 저장 데이터 SFTP 일괄 다운로드");
            ui.add_space(2.0);
            ui.label(
                RichText::new("원격 폴더의 모든 하위 디렉토리 계층 구조를 그대로 유지하여 호스트 PC 로컬 폴더로 안전하게 회수하고 무결성을 검증합니다.")
                    .color(Color32::from_rgb(100, 180, 255))
                    .small(),
            );
            ui.add_space(6.0);

            if targets.is_empty() {
                ui.label(RichText::new("등록된 타겟 장치가 없습니다. 먼저 [📡 연결관리]에서 장치를 등록하세요.").color(Color32::YELLOW));
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
                return;
            }

            // 비동기 파일 목록 조회 완료 결과 반영
            let refresh_res = {
                let mut lock = state.file_refresh_result.lock().unwrap();
                lock.take()
            };
            if let Some(res) = refresh_res {
                state.is_refreshing_files = false;
                match res {
                    Ok(files) => {
                        let count = files.len();
                        state.remote_files = files;
                        state.selected_files = vec![true; count];
                        state.status_msg = Some((
                            format!("✔ 원격 경로에서 총 {}개의 계측 파일 목록을 불러왔습니다.", count),
                            false,
                        ));
                    }
                    Err(err) => {
                        state.status_msg = Some((format!("❌ 파일 목록 조회 실패: {}", err), true));
                    }
                }
            }

            if state.selected_target_idx >= targets.len() {
                state.selected_target_idx = 0;
            }

            // 편집 버퍼 동기화: 대상 타겟이 아직 로드되지 않은 경우 버퍼 채움
            if state.buffer_loaded_idx != Some(state.selected_target_idx) {
                state.load_target_to_buffer(&targets[state.selected_target_idx], state.selected_target_idx);
            }

            let mut targets_changed = false;
            let mut switched_target = None;

            // 1. [상단 헤더 영역] 타겟 장치 선택 및 [우측 상단] 일괄/개별 다운로드 버튼 배치
            let is_busy = state.transfer_manager.is_busy();
            ui.horizontal(|ui| {
                ui.strong("다운로드 대상 타겟:");
                let current_target = &targets[state.selected_target_idx];
                let current_target_name = format!(
                    "#{} {} (SFTP: {})",
                    current_target.device_id,
                    current_target.name,
                    current_target.get_sftp_ip()
                );

                egui::ComboBox::from_id_salt("sftp_target_selector")
                    .selected_text(current_target_name)
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        for (idx, t) in targets.iter().enumerate() {
                            let label = format!("#{} {} (SFTP: {})", t.device_id, t.name, t.get_sftp_ip());
                            let is_selected = state.selected_target_idx == idx;
                            if ui.selectable_label(is_selected, label).clicked() {
                                if !is_selected {
                                    switched_target = Some(idx);
                                }
                            }
                        }
                    });

                if let Some(new_idx) = switched_target {
                    if state.buffer_loaded_idx == Some(state.selected_target_idx) {
                        state.sync_buffer_to_target(&mut targets[state.selected_target_idx]);
                    }
                    state.selected_target_idx = new_idx;
                    state.load_target_to_buffer(&targets[new_idx], new_idx);
                    targets_changed = true;
                }

                // [우측 상단 배치]: 모든 타겟 일괄 다운로드 및 선택 타겟 다운로드
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 1) 모든 타겟 일괄 다운로드 버튼 (가장 우측)
                    let all_btn_text = format!("📦 모든 타겟({}개) 일괄 다운로드", targets.len());
                    let all_btn = egui::Button::new(
                        RichText::new(all_btn_text)
                            .strong()
                            .color(if is_busy { Color32::GRAY } else { Color32::from_rgb(0, 240, 160) }),
                    );

                    if ui.add_enabled(!is_busy, all_btn)
                        .on_hover_text("등록된 모든 타겟 장치의 원격 폴더 파일들을 각각의 로컬 저장 폴더로 일괄 다운로드합니다.")
                        .clicked()
                    {
                        state.sync_buffer_to_target(&mut targets[state.selected_target_idx]);
                        let _ = TargetDeviceConfig::save_to_file(targets_config_path, targets);
                        state.status_msg = Some((
                            format!("모든 타겟(총 {}개 장치) 파일 일괄 다운로드를 시작합니다...", targets.len()),
                            false,
                        ));

                        let targets_copy = targets.to_vec();
                        let files_to_get = state.remote_files.clone();
                        let delete_remote = state.delete_remote_after;
                        let prog_arc = Arc::clone(&state.progress);

                        state.transfer_manager.download_all_targets_background(
                            targets_copy,
                            files_to_get,
                            delete_remote,
                            move |p| {
                                *prog_arc.lock().unwrap() = Some(p);
                            },
                        );
                    }

                    ui.add_space(6.0);

                    // 2) 선택된 타겟 다운로드 버튼
                    let target_now = &targets[state.selected_target_idx];
                    let single_btn_text = format!("⬇ [{}] 다운로드", target_now.name);
                    let single_btn = egui::Button::new(
                        RichText::new(single_btn_text)
                            .strong()
                            .color(if is_busy { Color32::GRAY } else { Color32::from_rgb(0, 220, 255) }),
                    );

                    if ui.add_enabled(!is_busy, single_btn)
                        .on_hover_text("현재 선택된 타겟의 원격 파일들을 호스트 PC 로컬 폴더로 다운로드합니다.")
                        .clicked()
                    {
                        state.sync_buffer_to_target(&mut targets[state.selected_target_idx]);
                        let _ = TargetDeviceConfig::save_to_file(targets_config_path, targets);
                        let target_now = &targets[state.selected_target_idx];
                        let target_name = target_now.name.clone();
                        let target_ip = target_now.get_sftp_ip();
                        let sftp_port = target_now.sftp_port;
                        let sftp_user = target_now.sftp_user.clone();
                        let sftp_pass = target_now.sftp_password.clone();
                        let remote_dir = target_now.remote_dir.clone();
                        let local_dir = PathBuf::from(&target_now.local_dir);

                        let mut files_to_get = Vec::new();
                        for (f_idx, f) in state.remote_files.iter().enumerate() {
                            if state.selected_files.get(f_idx).copied().unwrap_or(true) {
                                files_to_get.push(f.clone());
                            }
                        }

                        let delete_remote = state.delete_remote_after;
                        let prog_arc = Arc::clone(&state.progress);

                        state.status_msg = Some((
                            format!("[{}] 다운로드를 시작합니다 (대상: {}개 파일)...", target_name, files_to_get.len()),
                            false,
                        ));

                        state.transfer_manager.download_directory_background(
                            target_name,
                            target_ip,
                            sftp_port,
                            sftp_user,
                            sftp_pass,
                            remote_dir,
                            local_dir,
                            files_to_get,
                            delete_remote,
                            move |p| {
                                *prog_arc.lock().unwrap() = Some(p);
                            },
                        );
                    }
                });
            });

            ui.add_space(6.0);

            // 2. 타겟 접속 정보 및 경로 설정 박스
            {
                let target = &mut targets[state.selected_target_idx];
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(RichText::new("🔐 타겟 접속 정보 및 경로 설정").color(Color32::from_rgb(100, 220, 255)));
                        ui.label(RichText::new(format!("(연결관리 Data IP: {})", target.ip)).small().color(Color32::from_rgb(180, 200, 220)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("💾 설정 저장").on_hover_text("현재 입력된 접속정보와 경로를 targets_config.json에 영구 저장합니다.").clicked() {
                                state.sync_buffer_to_target(target);
                                targets_changed = true;
                                state.status_msg = Some(("✔ SFTP 설정 정보가 성공적으로 영구 저장되었습니다.".to_string(), false));
                            }
                        });
                    });
                    ui.add_space(4.0);

                    // Row 1: SFTP IP, Data IP 불러오기 버튼, SFTP Port, ID, Password
                    ui.horizontal(|ui| {
                        ui.label("SFTP IP:");
                        if ui.add_sized([110.0, 20.0], egui::TextEdit::singleline(&mut state.edit_ip)).changed() {
                            targets_changed = true;
                        }

                        if ui.button("📥 Data IP에서 불러오기")
                            .on_hover_text(format!("연결관리에서 설정한 타겟 Data IP({})를 SFTP IP로 가져옵니다.", target.ip))
                            .clicked()
                        {
                            state.edit_ip = target.ip.to_string();
                            targets_changed = true;
                            state.status_msg = Some((
                                format!("✔ 타겟 #{}의 Data IP({})를 SFTP IP로 불러왔습니다.", target.device_id, target.ip),
                                false,
                            ));
                        }

                        ui.add_space(8.0);
                        ui.label("SFTP 포트:");
                        if ui.add_sized([50.0, 20.0], egui::TextEdit::singleline(&mut state.edit_port)).changed() {
                            targets_changed = true;
                        }

                        ui.add_space(8.0);
                        ui.label("계정(ID):");
                        if ui.add_sized([85.0, 20.0], egui::TextEdit::singleline(&mut state.edit_user)).changed() {
                            targets_changed = true;
                        }

                        ui.add_space(8.0);
                        ui.label("비밀번호(PW):");
                        let pw_edit = egui::TextEdit::singleline(&mut state.edit_password)
                            .password(!state.show_password);
                        if ui.add_sized([95.0, 20.0], pw_edit).changed() {
                            targets_changed = true;
                        }

                        let eye_icon = if state.show_password { "👁 숨김" } else { "👁 보기" };
                        if ui.button(eye_icon).on_hover_text("비밀번호 표시/숨기기 토글").clicked() {
                            state.show_password = !state.show_password;
                        }
                    });

                    ui.add_space(6.0);

                    // Row 2: 원격 경로(폴더)
                    ui.horizontal(|ui| {
                        ui.label("🌐 원격 경로 (타겟 폴더):");
                        if ui.add_sized([380.0, 20.0], egui::TextEdit::singleline(&mut state.edit_remote_dir)).changed() {
                            targets_changed = true;
                        }
                        ui.label(RichText::new("예: /home/admin/data, /data/meas").small().color(Color32::GRAY));
                    });

                    ui.add_space(6.0);

                    // Row 3: 로컬 저장 경로(폴더)
                    ui.horizontal(|ui| {
                        ui.label("💻 로컬 경로 (호스트 저장):");
                        if ui.add_sized([380.0, 20.0], egui::TextEdit::singleline(&mut state.edit_local_dir)).changed() {
                            targets_changed = true;
                        }

                        if ui.button("🔍 폴더 선택").on_hover_text("로컬 저장 폴더를 마우스로 선택합니다.").clicked() {
                            let initial_title = format!("{} 로컬 저장 폴더 선택", target.name);
                            if let Some(folder) = prompt_folder_dialog(&initial_title) {
                                state.edit_local_dir = folder.to_string_lossy().to_string();
                                targets_changed = true;
                            }
                        }

                        if ui.button("📂 폴더 열기").on_hover_text("선택된 로컬 저장 폴더를 Windows 탐색기에서 엽니다.").clicked() {
                            let p = Path::new(&state.edit_local_dir);
                            let _ = open_path_in_explorer(p);
                        }
                    });
                });

                // 버퍼값을 현재 타겟 구조체로 동기화
                if targets_changed {
                    state.sync_buffer_to_target(target);
                }
            }

            // 설정 변경 시 targets_config.json에 즉시 영구 저장
            if targets_changed {
                let _ = TargetDeviceConfig::save_to_file(targets_config_path, targets);
            }

            ui.add_space(8.0);

            // 3. 파일 목록 제어 및 작업 옵션
            let target_ref = &targets[state.selected_target_idx];
            ui.horizontal(|ui| {
                let refresh_lbl = if state.is_refreshing_files {
                    "⏳ 목록 조회 중..."
                } else {
                    "🔄 파일 목록 새로고침"
                };

                let refresh_btn = egui::Button::new(
                    RichText::new(refresh_lbl)
                        .strong()
                        .color(if state.is_refreshing_files { Color32::GRAY } else { Color32::from_rgb(100, 200, 255) }),
                );

                let is_refreshing = state.is_refreshing_files;
                if ui.add_enabled(!is_refreshing, refresh_btn)
                    .on_hover_text("원격 타겟의 지정된 경로에서 최신 계측 파일 목록을 조회합니다.")
                    .clicked()
                {
                    let cur_ip: std::net::IpAddr = state.edit_ip.trim().parse().unwrap_or_else(|_| target_ref.get_sftp_ip());
                    let cur_port: u16 = state.edit_port.trim().parse().unwrap_or(target_ref.sftp_port);
                    let cur_user = state.edit_user.clone();
                    let cur_pass = state.edit_password.clone();
                    let cur_remote = state.edit_remote_dir.clone();

                    state.is_refreshing_files = true;
                    state.status_msg = Some((
                        format!("원격 타겟({}:{}) 파일 목록을 새로고침하는 중입니다...", cur_ip, cur_port),
                        false,
                    ));

                    let result_arc = Arc::clone(&state.file_refresh_result);
                    std::thread::spawn(move || {
                        let res = FileTransferManager::fetch_remote_file_list(
                            cur_ip,
                            cur_port,
                            &cur_user,
                            &cur_pass,
                            &cur_remote,
                        );
                        *result_arc.lock().unwrap() = Some(res);
                    });
                }

                if !state.remote_files.is_empty() {
                    ui.add_space(8.0);
                    if ui.button("✔ 전체 선택").clicked() {
                        state.selected_files.fill(true);
                    }
                    if ui.button("✖ 전체 해제").clicked() {
                        state.selected_files.fill(false);
                    }
                }

                ui.add_space(12.0);
                ui.checkbox(
                    &mut state.delete_remote_after,
                    "다운로드 완료 및 무결성 검증 후 타겟 원격 파일 정리(삭제)",
                );
            });

            ui.separator();

            // 4. 다운로드 진행 상태 및 진행률
            let progress_opt = { state.progress.lock().unwrap().clone() };
            if let Some(ref prog) = progress_opt {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(format!("전체 진행: {} / {} 파일", prog.current_file_index, prog.total_files));
                        if !prog.filename.is_empty() {
                            ui.label(format!("| 현재 파일: {}", prog.filename));
                        }
                    });

                    let ratio = if prog.total_bytes > 0 {
                        (prog.transferred_bytes as f32 / prog.total_bytes as f32).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    ui.add(ProgressBar::new(ratio).show_percentage());

                    match &prog.status {
                        FileTransferStatus::Downloading => {
                            ui.label(RichText::new("⬇ 다운로드 진행 중...").color(Color32::from_rgb(100, 180, 255)));
                        }
                        FileTransferStatus::Verifying => {
                            ui.label(RichText::new("🛡 SHA256 해시 무결성 검증 중...").color(Color32::YELLOW));
                        }
                        FileTransferStatus::Completed => {
                            if let Some(ref summary) = prog.summary_message {
                                ui.label(RichText::new(summary).strong().color(Color32::from_rgb(100, 240, 100)));
                            } else {
                                ui.label(RichText::new("✔ 파일 전송 및 검증 완료").color(Color32::from_rgb(100, 240, 100)));
                            }
                        }
                        FileTransferStatus::Failed(err) => {
                            ui.label(RichText::new(format!("❌ 다운로드 실패: {}", err)).color(Color32::RED));
                        }
                        _ => {}
                    }
                });
                ui.add_space(4.0);
            }

            if let Some((ref msg, is_err)) = state.status_msg {
                let color = if is_err { Color32::RED } else { Color32::from_rgb(100, 240, 100) };
                ui.label(RichText::new(msg).color(color));
                ui.add_space(4.0);
            }

            // 5. 파일 목록 그리드 테이블
            let table_height = (ui.available_height() - 45.0).max(180.0);
            egui::ScrollArea::vertical()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if state.remote_files.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(30.0);
                            ui.label(RichText::new("원격 계측 파일 목록이 비어 있거나 아직 조회되지 않았습니다.").color(Color32::GRAY));
                            ui.label(RichText::new("위의 [🔄 파일 목록 새로고침] 버튼을 눌러 원격 타겟의 계측 파일을 확인하세요.").small().color(Color32::from_rgb(100, 180, 255)));
                            ui.add_space(30.0);
                        });
                    } else {
                        egui::Grid::new("sftp_file_grid_v2")
                            .striped(true)
                            .min_col_width(70.0)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.strong("선택");
                                ui.strong("파일명");
                                ui.strong("크기 (Bytes)");
                                ui.strong("수정시각");
                                ui.strong("SHA256 체크섬");
                                ui.end_row();

                                for (idx, file) in state.remote_files.iter().enumerate() {
                                    if idx >= state.selected_files.len() {
                                        state.selected_files.resize(idx + 1, true);
                                    }

                                    ui.checkbox(&mut state.selected_files[idx], "");
                                    ui.label(&file.filename).on_hover_text(format!("원격 상대 경로: {}\n크기: {} Bytes", file.filename, file.size_bytes));
                                    ui.label(format!("{:.2} KB", file.size_bytes as f64 / 1024.0));
                                    ui.label(&file.modified_time);
                                    let hash_preview = if file.sha256.is_empty() {
                                        "Auto-verify".to_string()
                                    } else {
                                        format!("{}...", &file.sha256[0..8.min(file.sha256.len())])
                                    };
                                    ui.label(hash_preview);
                                    ui.end_row();
                                }
                            });
                    }
                });

            let remaining = (ui.available_height() - 32.0).max(0.0);
            if remaining > 0.0 {
                ui.add_space(remaining);
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("닫기").clicked() {
                        let _ = TargetDeviceConfig::save_to_file(targets_config_path, targets);
                        state.is_open = false;
                    }
                });
            });
        });
}
