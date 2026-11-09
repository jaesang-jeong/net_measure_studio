use crate::model::channel::ChannelState;
use crate::model::target::TargetDeviceConfig;
use crate::network::control_client::ControlClient;
use crate::protocol::commands::ChannelConfigItem;
use eframe::egui::{self, Color32, RichText, Window};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// 실행 파일 기준 안전한 작업 디렉터리 경로 반환 (System32 권한 거부 방지)
pub fn get_app_base_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            // bin/ 폴더 내부에서 실행된 경우 프로젝트 루트로 한 단계 상위 디렉터리 지정
            if parent.file_name().map(|n| n == "bin").unwrap_or(false) {
                if let Some(project_root) = parent.parent() {
                    return project_root.to_path_buf();
                }
            }
            return parent.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 경로가 드라이브 루트(예: C:\, D:\ 등) 바로 아래인지 확인 (Windows NTFS 보안상 관리자 권한 없이 파일 생성 금지 구역)
pub fn is_drive_root(path: &Path) -> bool {
    let mut comps = path.components();
    match (comps.next(), comps.next(), comps.next()) {
        (Some(std::path::Component::Prefix(_)), Some(std::path::Component::RootDir), Some(std::path::Component::Normal(_))) => {
            comps.next().is_none()
        }
        _ => false,
    }
}

/// 입력된 파일 경로를 안전한 절대 경로로 해석 (쌍따옴표 및 앞뒤 공백 제거)
pub fn resolve_csv_path(input_path: &str) -> PathBuf {
    let trimmed = input_path.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return get_app_base_dir().join("channel_config.csv");
    }
    let p = Path::new(trimmed);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        get_app_base_dir().join(trimmed)
    }
}

/// Windows 표준 GetSaveFileNameW 대화상자를 호출하여 CSV 저장/선택 위치를 마우스로 지정
#[cfg(target_os = "windows")]
pub fn prompt_save_file(default_name: &str, initial_dir: Option<&Path>) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStrExt;

    let mut file_buf = [0u16; 1024];
    let default_encoded: Vec<u16> = Path::new(default_name)
        .as_os_str()
        .encode_wide()
        .collect();
    let copy_len = default_encoded.len().min(file_buf.len() - 1);
    file_buf[..copy_len].copy_from_slice(&default_encoded[..copy_len]);

    let filter: Vec<u16> = "CSV 파일 (*.csv)\0*.csv\0모든 파일 (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title: Vec<u16> = "CSV 설정 파일 위치 선택\0".encode_utf16().collect();
    let def_ext: Vec<u16> = "csv\0".encode_utf16().collect();

    let init_dir_wide: Option<Vec<u16>> = initial_dir.map(|p| {
        p.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
    });

    #[repr(C)]
    #[allow(non_snake_case)]
    struct OPENFILENAMEW {
        lStructSize: u32,
        hwndOwner: *mut std::ffi::c_void,
        hInstance: *mut std::ffi::c_void,
        lpstrFilter: *const u16,
        lpstrCustomFilter: *mut u16,
        nMaxCustFilter: u32,
        nFilterIndex: u32,
        lpstrFile: *mut u16,
        nMaxFile: u32,
        lpstrFileTitle: *mut u16,
        nMaxFileTitle: u32,
        lpstrInitialDir: *const u16,
        lpstrTitle: *const u16,
        Flags: u32,
        nFileOffset: u16,
        nFileExtension: u16,
        lpstrDefExt: *const u16,
        lCustData: usize,
        lpfnHook: *mut std::ffi::c_void,
        lpTemplateName: *const u16,
        pvReserved: *mut std::ffi::c_void,
        dwReserved: u32,
        FlagsEx: u32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    }

    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file_buf.as_mut_ptr();
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.lpstrTitle = title.as_ptr();
    ofn.lpstrDefExt = def_ext.as_ptr();
    if let Some(ref dir) = init_dir_wide {
        ofn.lpstrInitialDir = dir.as_ptr();
    }
    // OFN_OVERWRITEPROMPT (0x00000002) | OFN_NOCHANGEDIR (0x00000008)
    ofn.Flags = 0x00000002 | 0x00000008;

    let res = unsafe { GetSaveFileNameW(&mut ofn) };
    if res != 0 {
        let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
        use std::os::windows::ffi::OsStringExt;
        let os_str = std::ffi::OsString::from_wide(&file_buf[..len]);
        Some(PathBuf::from(os_str))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
pub fn prompt_save_file(_default_name: &str, _initial_dir: Option<&Path>) -> Option<PathBuf> {
    None
}

/// Windows ShellExecuteW API를 사용하여 안전하게 폴더 탐색기 열기 (0xc0000142 DLL init 오류 원천 차단)
#[cfg(target_os = "windows")]
pub fn open_path_in_explorer(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    let folder = if path.is_dir() {
        path.to_path_buf()
    } else if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
            parent.to_path_buf()
        } else {
            get_app_base_dir()
        }
    } else {
        get_app_base_dir()
    };

    let folder_display = folder.display().to_string();
    let wide_folder: Vec<u16> = folder.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let wide_op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut std::ffi::c_void,
            lpOperation: *const u16,
            lpFile: *const u16,
            lpParameters: *const u16,
            lpDirectory: *const u16,
            nShowCmd: i32,
        ) -> isize;
    }

    let ret = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide_op.as_ptr(),
            wide_folder.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        )
    };

    // Windows 규칙: ShellExecute 반환값이 32보다 크면 정상 성공
    if ret > 32 {
        Ok(())
    } else {
        Err(format!("탐색기 열기 실패 (ShellExecute 오류 코드: {}, 대상: {})", ret, folder_display))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn open_path_in_explorer(path: &Path) -> Result<(), String> {
    let folder = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
    let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
    Ok(())
}

/// 타겟 장치별 기본 CSV 파일 절대 경로 생성
fn default_csv_path_for_target(target_name: Option<&str>) -> String {
    let file_name = match target_name {
        Some(name) => format!("channel_config_{}.csv", name.to_lowercase().replace(' ', "_")),
        None => "channel_config_all.csv".to_string(),
    };
    resolve_csv_path(&file_name).to_string_lossy().to_string()
}

pub struct ConfigModalState {
    pub is_open: bool,
    pub status_message: Option<(String, bool)>, // (msg, is_error)
    pub search_query: String,
    pub selected_target_idx: usize,
    pub csv_path: String,
}

impl Default for ConfigModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            status_message: None,
            search_query: String::new(),
            selected_target_idx: 0,
            csv_path: default_csv_path_for_target(Some("target-station-01")),
        }
    }
}

/// CSV로 채널 설정 내보내기 (UTF-8 BOM 포함, 엑셀 호환 및 폴더 자동 생성)
fn export_channels_to_csv(path_str: &str, channels: &[ChannelConfigItem]) -> Result<(usize, PathBuf), String> {
    let resolved = resolve_csv_path(path_str);

    // 상위 디렉터리가 없으면 자동 생성
    if let Some(parent) = resolved.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("디렉터리 생성 실패 [{}]: {}", parent.display(), e)
            })?;
        }
    }

    let is_root = is_drive_root(&resolved);

    let mut file = File::create(&resolved).map_err(|e| {
        let hint = if is_root {
            "\n💡 [안내] Windows 보안 정책상 드라이브 루트(D:\\, C:\\ 등)에는 관리자 권한 없이 파일을 직접 생성할 수 없습니다. 하위 폴더(예: D:\\설정\\, 바탕화면, 문서 등)를 지정하거나 [🔍 찾아보기]를 클릭해 저장 위치를 선택해주세요."
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            "\n💡 [안내] 엑셀 등 다른 프로그램에서 이 CSV 파일이 열려있는지 확인 후 닫아주세요."
        } else {
            ""
        };
        format!("파일 생성 실패 [{}]: {}{}", resolved.display(), e, hint)
    })?;

    // UTF-8 BOM
    file.write_all(&[0xEF, 0xBB, 0xBF]).map_err(|e| format!("BOM 기록 실패: {}", e))?;

    writeln!(file, "CH,Name,Unit,SensorType,Gain,Offset,WarnLow,WarnHigh,AlarmLow,AlarmHigh")
        .map_err(|e| format!("헤더 작성 실패: {}", e))?;

    for ch in channels {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{}",
            ch.channel_index + 1,
            ch.name.replace(',', " "),
            ch.unit.replace(',', " "),
            ch.sensor_type.replace(',', " "),
            ch.gain,
            ch.offset,
            ch.warn_low,
            ch.warn_high,
            ch.alarm_low,
            ch.alarm_high
        ).map_err(|e| format!("데이터 작성 실패: {}", e))?;
    }
    file.flush().map_err(|e| format!("저장 완료 flush 실패: {}", e))?;

    Ok((channels.len(), resolved))
}

/// 채널 상태에 설정 적용 및 스케일된 값, 알람 상태 즉시 갱신
fn apply_config_to_channel(ch: &mut ChannelState, mut item: ChannelConfigItem, target_idx: usize) {
    item.channel_index = target_idx;
    ch.config = item;
    ch.scaled_value = ch.raw_value * ch.config.gain + ch.config.offset;
    let val = ch.scaled_value;
    if val <= ch.config.alarm_low || val >= ch.config.alarm_high {
        ch.alarm_level = crate::model::channel::ChannelAlarmLevel::Alarm;
    } else if val <= ch.config.warn_low || val >= ch.config.warn_high {
        ch.alarm_level = crate::model::channel::ChannelAlarmLevel::Warning;
    } else {
        ch.alarm_level = crate::model::channel::ChannelAlarmLevel::Normal;
    }
}

/// CSV에서 채널 설정 불러오기 (엑셀 인용부호 및 UTF-8 BOM 지원)
fn import_channels_from_csv(path_str: &str) -> Result<(Vec<(usize, ChannelConfigItem)>, PathBuf), String> {
    let resolved = resolve_csv_path(path_str);
    let file = File::open(&resolved)
        .map_err(|e| {
            let is_root = is_drive_root(&resolved);
            let hint = if is_root && e.kind() == std::io::ErrorKind::NotFound {
                "\n💡 [안내] 드라이브 루트(D:\\, C:\\)에 해당 파일이 존재하지 않습니다. [🔍 찾아보기]를 눌러 저장된 파일을 선택해주세요."
            } else {
                ""
            };
            format!("파일 열기 실패 [{}]: {}{}", resolved.display(), e, hint)
        })?;
    let reader = BufReader::new(file);

    let mut items = Vec::new();

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| format!("파일 읽기 오류: {}", e))?;
        let line = line.trim_start_matches('\u{FEFF}').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<String> = line
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim().to_string())
            .collect();

        // 헤더 행 건너뛰기
        if parts.first().map(|s| s.eq_ignore_ascii_case("ch") || s.eq_ignore_ascii_case("channel") || s.starts_with("채널")).unwrap_or(false) {
            continue;
        }

        if parts.len() < 2 {
            continue;
        }

        // 채널 번호 파싱 (예: "1", "001", "CH001", "CH1", "#1" 모두 유연하게 지원)
        let clean_digits: String = parts[0].chars().filter(|c| c.is_ascii_digit()).collect();
        let ch_num: usize = clean_digits.parse().unwrap_or(items.len() + 1);
        let ch_idx = if ch_num > 0 { ch_num - 1 } else { items.len() };

        let name = parts[1].clone();
        let unit = parts.get(2).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| "V".to_string());
        let sensor_type = parts.get(3).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| "Voltage".to_string());
        let gain: f32 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let offset: f32 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let warn_low: f32 = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(-10.0);
        let warn_high: f32 = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(10.0);
        let alarm_low: f32 = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(-12.0);
        let alarm_high: f32 = parts.get(9).and_then(|s| s.parse().ok()).unwrap_or(12.0);

        items.push((ch_idx, ChannelConfigItem {
            channel_index: ch_idx,
            name,
            unit,
            sensor_type,
            gain,
            offset,
            warn_low,
            warn_high,
            alarm_low,
            alarm_high,
        }));
    }

    if items.is_empty() {
        return Err("유효한 채널 데이터 행을 찾을 수 없습니다.".to_string());
    }

    Ok((items, resolved))
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

    // 기본 경로가 완전히 비어있는 경우에만 기본 절대 경로로 초기화
    if state.csv_path.trim().is_empty() {
        let target_name = targets.get(state.selected_target_idx).map(|t| t.name.as_str());
        state.csv_path = default_csv_path_for_target(target_name);
    }

    Window::new("채널 파라미터 설정 및 장치 동기화 (Channel Configuration)")
        .collapsible(false)
        .resizable([true, true])
        .default_size([1180.0, 750.0])
        .min_width(950.0)
        .show(ctx, |ui| {
            // 1. Title & Search
            ui.horizontal(|ui| {
                ui.heading("채널별 스케일링(Y = aX + b) 및 임계값 설정");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.search_query).desired_width(160.0));
                    ui.label("검색:");
                });
            });
            ui.add_space(4.0);

            // 2. Target Device Selection Tabs (장치별 분리)
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("🎯 대상 장치:").strong());
                for (idx, target) in targets.iter().enumerate() {
                    let is_selected = state.selected_target_idx == idx;
                    let ch_start = target.channel_offset + 1;
                    let ch_end = target.channel_offset + target.channel_count;
                    let btn_text = format!(
                        "{} (CH{:03}~CH{:03}, {}ch)",
                        target.name,
                        ch_start,
                        ch_end,
                        target.channel_count
                    );
                    if ui.selectable_label(is_selected, btn_text).clicked() {
                        state.selected_target_idx = idx;
                        state.csv_path = default_csv_path_for_target(Some(&target.name));
                        state.status_message = None;
                    }
                }

                let all_idx = targets.len();
                let current_total_ch = channels_arc.read().unwrap().len();
                let all_btn_text = format!("전체 채널 (CH001~CH{:03}, {}ch)", current_total_ch, current_total_ch);
                if ui.selectable_label(state.selected_target_idx == all_idx, all_btn_text).clicked() {
                    state.selected_target_idx = all_idx;
                    state.csv_path = default_csv_path_for_target(None);
                    state.status_message = None;
                }
            });
            ui.add_space(4.0);

            // 3. Action buttons: Deploy / Fetch per Target
            ui.horizontal(|ui| {
                let is_specific_target = state.selected_target_idx < targets.len();

                // Deploy Button
                let deploy_btn_text = if is_specific_target {
                    format!("⬆ {}로 설정 배포 (CMD_CFG_SET)", targets[state.selected_target_idx].name)
                } else {
                    "⬆ 모든 등록 장치로 순차 배포 (CMD_CFG_SET)".to_string()
                };

                if ui.button(deploy_btn_text).clicked() {
                    let targets_to_deploy: Vec<&TargetDeviceConfig> = if is_specific_target {
                        vec![&targets[state.selected_target_idx]]
                    } else {
                        targets.iter().collect()
                    };

                    let mut success_devices = Vec::new();
                    let mut fail_devices = Vec::new();

                    for target in targets_to_deploy {
                        let addr_str = format!("{}:{}", target.ip, target.control_port);
                        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                            let ch_list: Vec<ChannelConfigItem> = {
                                let guard = channels_arc.read().unwrap();
                                let start = target.channel_offset;
                                let end = (start + target.channel_count).min(guard.len());
                                if start < guard.len() {
                                    guard[start..end]
                                        .iter()
                                        .enumerate()
                                        .map(|(i, c)| {
                                            let mut item = c.config.clone();
                                            item.channel_index = i;
                                            item
                                        })
                                        .collect()
                                } else {
                                    Vec::new()
                                }
                            };

                            match ControlClient::set_config(addr, ch_list) {
                                Ok(_) => success_devices.push(target.name.clone()),
                                Err(e) => fail_devices.push(format!("{}: {}", target.name, e)),
                            }
                        }
                    }

                    if fail_devices.is_empty() && !success_devices.is_empty() {
                        state.status_message = Some((
                            format!("[{}] 설정 배포 완료!", success_devices.join(", ")),
                            false,
                        ));
                    } else if !fail_devices.is_empty() {
                        state.status_message = Some((
                            format!("배포 오류: {}", fail_devices.join("; ")),
                            true,
                        ));
                    }
                }

                // Fetch Button
                let fetch_btn_text = if is_specific_target {
                    format!("⬇ {}에서 가져오기 (CMD_CFG_GET)", targets[state.selected_target_idx].name)
                } else {
                    "⬇ 모든 등록 장치에서 순차 가져오기 (CMD_CFG_GET)".to_string()
                };

                if ui.button(fetch_btn_text).clicked() {
                    let targets_to_fetch: Vec<&TargetDeviceConfig> = if is_specific_target {
                        vec![&targets[state.selected_target_idx]]
                    } else {
                        targets.iter().collect()
                    };

                    let mut total_updated = 0;
                    let mut fail_devices = Vec::new();

                    for target in targets_to_fetch {
                        let addr_str = format!("{}:{}", target.ip, target.control_port);
                        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                            match ControlClient::get_config(addr) {
                                Ok(remote_configs) => {
                                    let mut guard = channels_arc.write().unwrap();
                                    let ch_start = target.channel_offset;
                                    let ch_end = (ch_start + target.channel_count).min(guard.len());

                                    for (i, item) in remote_configs.into_iter().enumerate() {
                                        let target_ch_idx = if item.channel_index < (ch_end.saturating_sub(ch_start)) {
                                            ch_start + item.channel_index
                                        } else if item.channel_index >= ch_start && item.channel_index < ch_end {
                                            item.channel_index
                                        } else {
                                            ch_start + i
                                        };

                                        if target_ch_idx < guard.len() {
                                            guard[target_ch_idx].config = item;
                                            guard[target_ch_idx].config.channel_index = target_ch_idx;
                                            total_updated += 1;
                                        }
                                    }
                                }
                                Err(e) => {
                                    fail_devices.push(format!("{}: {}", target.name, e));
                                }
                            }
                        }
                    }

                    if fail_devices.is_empty() {
                        state.status_message = Some((
                            format!("총 {}개 채널 설정 동기화 완료!", total_updated),
                            false,
                        ));
                    } else {
                        state.status_message = Some((
                            format!("가져오기 실패: {}", fail_devices.join("; ")),
                            true,
                        ));
                    }
                }

                // 4. Status Message
                if let Some((ref msg, is_err)) = state.status_message {
                    let color = if is_err {
                        Color32::from_rgb(255, 100, 100)
                    } else {
                        Color32::from_rgb(100, 240, 100)
                    };
                    ui.label(RichText::new(msg).color(color));
                }
            });

            // 5. CSV Import / Export Controls (찾아보기 대화상자, 경로 해석 및 안전한 폴더 열기)
            // 5. CSV Import / Export Controls (선택된 타겟 또는 전체 채널 격리 적용)
            ui.horizontal(|ui| {
                ui.label(RichText::new("📁 CSV 설정 파일:").strong());
                ui.add(egui::TextEdit::singleline(&mut state.csv_path).desired_width(280.0));

                if ui.button("🔍 찾아보기").on_hover_text("Windows 파일 대화상자를 열어 저장하거나 불러올 CSV 파일 위치를 선택합니다.").clicked() {
                    let current_path = resolve_csv_path(&state.csv_path);
                    let initial_dir = current_path.parent();
                    let file_name = current_path.file_name().and_then(|n| n.to_str()).unwrap_or("channel_config.csv");
                    if let Some(selected) = prompt_save_file(file_name, initial_dir) {
                        state.csv_path = selected.to_string_lossy().to_string();
                    }
                }

                let is_specific_target = state.selected_target_idx < targets.len();
                let target_opt = if is_specific_target {
                    Some(&targets[state.selected_target_idx])
                } else {
                    None
                };

                // 타겟별 / 전체 채널 구분 버튼 문구
                let export_btn_text = match target_opt {
                    Some(t) => format!("📤 [{}] CSV 내보내기 ({}ch)", t.name, t.channel_count),
                    None => format!("📤 [전체 채널] CSV 내보내기 ({}ch)", channels_arc.read().unwrap().len()),
                };

                if ui.button(export_btn_text).clicked() {
                    let channels_to_export: Vec<ChannelConfigItem> = {
                        let guard = channels_arc.read().unwrap();
                        if let Some(target) = target_opt {
                            let start = target.channel_offset;
                            let end = (start + target.channel_count).min(guard.len());
                            if start < guard.len() {
                                guard[start..end].iter().map(|c| c.config.clone()).collect()
                            } else {
                                Vec::new()
                            }
                        } else {
                            guard.iter().map(|c| c.config.clone()).collect()
                        }
                    };

                    match export_channels_to_csv(&state.csv_path, &channels_to_export) {
                        Ok((count, saved_path)) => {
                            let target_name = target_opt.map(|t| t.name.as_str()).unwrap_or("전체 채널");
                            state.status_message = Some((
                                format!("[CSV 저장 완료] [{}] {}개 채널 설정 저장: {}", target_name, count, saved_path.display()),
                                false,
                            ));
                        }
                        Err(e) => {
                            state.status_message = Some((
                                format!("[CSV 저장 실패] {}", e),
                                true,
                            ));
                        }
                    }
                }

                let import_btn_text = match target_opt {
                    Some(t) => format!("📥 [{}] CSV 불러오기", t.name),
                    None => "📥 [전체 채널] CSV 불러오기".to_string(),
                };

                if ui.button(import_btn_text).clicked() {
                    match import_channels_from_csv(&state.csv_path) {
                        Ok((imported, loaded_path)) => {
                            let mut guard = channels_arc.write().unwrap();
                            let mut updated_count = 0;

                            if let Some(target) = target_opt {
                                // [선택된 타겟의 채널에만 격리하여 반영]
                                let start = target.channel_offset;
                                let count = target.channel_count;
                                let end = start + count;

                                if guard.len() < end {
                                    for i in guard.len()..end {
                                        guard.push(ChannelState::new(ChannelConfigItem::default_for(i)));
                                    }
                                }

                                for (row_idx, (raw_idx, item)) in imported.into_iter().enumerate() {
                                    let ch_num = raw_idx + 1;
                                    // 1) 만약 글로벌 채널 번호(start+1..=end)에 해당하면 그대로 매핑
                                    // 2) 로컬 채널 번호(1..=count)에 해당하면 start + (ch_num - 1) 매핑
                                    // 3) 그 외의 경우 행 순서(row_idx < count)대로 타겟 채널(start + row_idx)에 순차 매핑!
                                    let target_idx = if ch_num >= (start + 1) && ch_num <= end {
                                        ch_num - 1
                                    } else if ch_num >= 1 && ch_num <= count {
                                        start + (ch_num - 1)
                                    } else if row_idx < count {
                                        start + row_idx
                                    } else {
                                        continue;
                                    };

                                    if target_idx < guard.len() {
                                        apply_config_to_channel(&mut guard[target_idx], item, target_idx);
                                        updated_count += 1;
                                    }
                                }

                                state.status_message = Some((
                                    format!("[CSV 적용 완료] [{}] {}개 채널 설정 로드: {}", target.name, updated_count, loaded_path.display()),
                                    false,
                                ));
                            } else {
                                // [전체 채널 일괄 반영]
                                let needed = imported.iter().map(|(idx, _)| idx + 1).max().unwrap_or(0).max(imported.len());
                                if guard.len() < needed {
                                    for i in guard.len()..needed {
                                        guard.push(ChannelState::new(ChannelConfigItem::default_for(i)));
                                    }
                                }

                                for (row_idx, (idx, item)) in imported.into_iter().enumerate() {
                                    let target_idx = if idx < guard.len() {
                                        idx
                                    } else if row_idx < guard.len() {
                                        row_idx
                                    } else {
                                        continue;
                                    };
                                    apply_config_to_channel(&mut guard[target_idx], item, target_idx);
                                    updated_count += 1;
                                }

                                state.status_message = Some((
                                    format!("[CSV 적용 완료] [전체 채널] {}개 채널 설정 로드: {}", updated_count, loaded_path.display()),
                                    false,
                                ));
                            }
                        }
                        Err(e) => {
                            state.status_message = Some((
                                format!("[CSV 불러오기 실패] {}", e),
                                true,
                            ));
                        }
                    }
                }

                if ui.button("📂 폴더 열기").on_hover_text("CSV 파일이 위치한 폴더를 Windows 탐색기에서 엽니다.").clicked() {
                    let resolved = resolve_csv_path(&state.csv_path);
                    match open_path_in_explorer(&resolved) {
                        Ok(_) => {
                            let folder_name = resolved.parent().unwrap_or(&resolved).display().to_string();
                            state.status_message = Some((
                                format!("폴더 열기 성공: {}", folder_name),
                                false,
                            ));
                        }
                        Err(e) => {
                            state.status_message = Some((e, true));
                        }
                    }
                }
            });

            ui.separator();

            // 6. Channel Grid Table (장치별 슬라이스 범위 적용)
            let mut channels = channels_arc.write().unwrap();
            let query = state.search_query.to_lowercase();

            let (slice_start, slice_end) = if state.selected_target_idx < targets.len() {
                let target = &targets[state.selected_target_idx];
                let start = target.channel_offset;
                let end = (start + target.channel_count).min(channels.len());
                (start, end)
            } else {
                (0, channels.len())
            };

            let table_height = (ui.available_height() - 45.0).max(250.0);

            egui::ScrollArea::both()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("channels_config_grid_v5")
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

                            if slice_start < channels.len() {
                                for ch in channels[slice_start..slice_end].iter_mut() {
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
                });
            });
        });
}
