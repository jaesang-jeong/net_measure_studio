use crate::network::file_transfer::{
    FileTransferManager, FileTransferProgress, FileTransferStatus, RemoteFileInfo,
};
use eframe::egui::{self, Color32, ProgressBar, RichText, Window};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct SftpModalState {
    pub is_open: bool,
    pub remote_files: Vec<RemoteFileInfo>,
    pub selected_files: Vec<bool>,
    pub delete_remote_after: bool,
    pub transfer_manager: FileTransferManager,
    pub progress: Arc<Mutex<Option<FileTransferProgress>>>,
    pub download_dir: PathBuf,
}

impl Default for SftpModalState {
    fn default() -> Self {
        let default_files = vec![
            RemoteFileInfo {
                filename: "REC_20261102_140000.bin".to_string(),
                size_bytes: 512 * 1024, // 512 KB
                modified_time: "2026-11-02 14:05:22".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            },
            RemoteFileInfo {
                filename: "REC_20261102_143000.csv".to_string(),
                size_bytes: 1024 * 1024, // 1 MB
                modified_time: "2026-11-02 14:35:10".to_string(),
                sha256: "".to_string(),
            },
            RemoteFileInfo {
                filename: "LOG_TARGET_CH100.bin".to_string(),
                size_bytes: 2 * 1024 * 1024, // 2 MB
                modified_time: "2026-11-02 15:00:00".to_string(),
                sha256: "".to_string(),
            },
        ];

        let count = default_files.len();
        Self {
            is_open: false,
            remote_files: default_files,
            selected_files: vec![false; count],
            delete_remote_after: false,
            transfer_manager: FileTransferManager::new(),
            progress: Arc::new(Mutex::new(None)),
            download_dir: std::env::current_dir().unwrap_or_default().join("downloads"),
        }
    }
}

pub fn render_sftp_modal(ctx: &egui::Context, state: &mut SftpModalState) {
    if !state.is_open {
        return;
    }

    Window::new("저장 파일 회수 (SFTP File Transfer)")
        .collapsible(false)
        .resizable([false, true]) // 좌우 고정, 상하 조절 가능
        .min_width(800.0)
        .max_width(800.0)         // 내용에 맞게 좌우 크기 고정
        .default_height(650.0)     // 상하로 넉넉한 기본 높이
        .show(ctx, |ui| {
            ui.heading("타겟 저장장치 파일 다운로드 및 SHA256 무결성 검증");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(format!("저장 경로: {}", state.download_dir.display()));
                if ui.button("📁 폴더 생성/확인").clicked() {
                    let _ = std::fs::create_dir_all(&state.download_dir);
                }
            });

            ui.checkbox(
                &mut state.delete_remote_after,
                "다운로드 완료 및 무결성 검증 후 타겟 로컬 파일 정리(삭제)",
            );

            ui.separator();

            let file_table_height = (ui.available_height() - 150.0).max(180.0);

            // File Table
            egui::ScrollArea::vertical()
                .max_height(file_table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("sftp_file_grid")
                        .striped(true)
                        .min_col_width(80.0)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong("선택");
                            ui.strong("파일명");
                            ui.strong("크기 (Bytes)");
                            ui.strong("생성/수정시각");
                            ui.strong("SHA256");
                            ui.end_row();

                            for (idx, file) in state.remote_files.iter().enumerate() {
                                if idx >= state.selected_files.len() {
                                    state.selected_files.resize(idx + 1, false);
                                }

                                ui.checkbox(&mut state.selected_files[idx], "");
                                ui.label(&file.filename);
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
                });

            ui.separator();

            // Progress Bar & Status
            let progress_opt = { state.progress.lock().unwrap().clone() };
            if let Some(prog) = progress_opt {
                ui.label(format!("파일: {} (전송 중)", prog.filename));
                let ratio = if prog.total_bytes > 0 {
                    prog.transferred_bytes as f32 / prog.total_bytes as f32
                } else {
                    0.0
                };
                ui.add(ProgressBar::new(ratio).show_percentage());

                match prog.status {
                    FileTransferStatus::Downloading => {
                        ui.label(RichText::new("다운로드 진행 중...").color(Color32::from_rgb(100, 180, 255)));
                    }
                    FileTransferStatus::Verifying => {
                        ui.label(RichText::new("SHA256 해시 무결성 검증 중...").color(Color32::YELLOW));
                    }
                    FileTransferStatus::Completed => {
                        let msg = if prog.hash_verified {
                            "✔ 다운로드 완료 및 SHA256 해시 검증 성공!"
                        } else {
                            "✔ 다운로드 완료 (해시 불일치 경고)"
                        };
                        ui.label(RichText::new(msg).color(Color32::from_rgb(100, 240, 100)));
                    }
                    FileTransferStatus::Failed(err) => {
                        ui.label(RichText::new(format!("❌ 다운로드 실패: {}", err)).color(Color32::RED));
                    }
                    _ => {}
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let is_busy = state.transfer_manager.is_busy();
                if ui
                    .add_enabled(!is_busy, egui::Button::new("⬇ 선택 파일 백그라운드 다운로드"))
                    .clicked()
                {
                    let _ = std::fs::create_dir_all(&state.download_dir);
                    for (idx, &selected) in state.selected_files.iter().enumerate() {
                        if selected && idx < state.remote_files.len() {
                            let file_info = state.remote_files[idx].clone();
                            let prog_arc = Arc::clone(&state.progress);
                            state.transfer_manager.download_file_background(
                                file_info,
                                state.download_dir.clone(),
                                state.delete_remote_after,
                                move |p| {
                                    *prog_arc.lock().unwrap() = Some(p);
                                },
                            );
                            break;
                        }
                    }
                }

                if ui.button("새로고침").clicked() {
                    // Refresh file list
                }

                if ui.button("닫기").clicked() {
                    state.is_open = false;
                }
            });
        });
}
