use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataTransportProtocol {
    Udp,
    Tcp,
}

impl Default for DataTransportProtocol {
    fn default() -> Self {
        DataTransportProtocol::Udp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Normal,   // 정상 수신 (< 300ms)
    Delayed,  // 지연 (300ms ~ 1000ms)
    Offline,  // 단절 (> 1000ms)
}

fn default_channel_count() -> usize {
    50
}

fn default_sftp_user() -> String {
    "admin".to_string()
}

fn default_remote_dir() -> String {
    "/home/admin/data".to_string()
}

fn default_local_dir() -> String {
    "downloads".to_string()
}

fn default_interval_ms() -> u32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDeviceConfig {
    pub device_id: u16,
    pub name: String,
    pub ip: IpAddr,
    pub data_port: u16,       // UDP / TCP data port
    pub control_port: u16,    // TCP control command port
    pub sftp_port: u16,       // SFTP file transfer port
    pub transport: DataTransportProtocol,
    #[serde(default)]
    pub channel_offset: usize, // 시작 채널 인덱스 (0 = CH001부터, 50 = CH051부터)
    #[serde(default = "default_channel_count")]
    pub channel_count: usize,  // 이 장치의 채널 수 (기본: 50)
    pub enabled: bool,
    #[serde(default)]
    pub sftp_ip: Option<IpAddr>, // SFTP 접속 전용 IP (None일 경우 계측 데이터 IP 사용)
    #[serde(default = "default_sftp_user")]
    pub sftp_user: String,
    #[serde(default)]
    pub sftp_password: String,
    #[serde(default = "default_remote_dir")]
    pub remote_dir: String,
    #[serde(default = "default_local_dir")]
    pub local_dir: String,
    #[serde(default = "default_interval_ms")]
    pub packet_interval_ms: u32,
}

pub const TARGETS_CONFIG_FILE: &str = "targets_config.json";

/// 설정 파일(targets_config.json) 경로 해석
pub fn resolve_target_config_path() -> std::path::PathBuf {
    let filename = TARGETS_CONFIG_FILE;
    let p = std::path::Path::new(filename);
    if p.exists() {
        return p.to_path_buf();
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let exe_adj = parent.join(filename);
            if exe_adj.exists() {
                return exe_adj;
            }
            if parent.file_name().map(|n| n == "bin").unwrap_or(false) {
                if let Some(root) = parent.parent() {
                    let root_adj = root.join(filename);
                    if root_adj.exists() {
                        return root_adj;
                    }
                }
            }
            return exe_adj;
        }
    }
    std::path::PathBuf::from(filename)
}

impl TargetDeviceConfig {
    /// SFTP 접속 전용 IP를 반환하며, 설정되지 않았을 경우 데이터 통신 IP를 반환합니다.
    pub fn get_sftp_ip(&self) -> IpAddr {
        self.sftp_ip.unwrap_or(self.ip)
    }

    pub fn new_default(device_id: u16, name: &str, ip: IpAddr) -> Self {
        let channel_count = 50;
        let channel_offset = (device_id.saturating_sub(1) as usize) * channel_count;
        Self {
            device_id,
            name: name.to_string(),
            ip,
            data_port: 8000,
            control_port: 9000,
            sftp_port: 22,
            transport: DataTransportProtocol::Udp,
            channel_offset,
            channel_count,
            enabled: true,
            sftp_ip: Some(ip),
            sftp_user: "admin".to_string(),
            sftp_password: "".to_string(),
            remote_dir: "/home/admin/data".to_string(),
            local_dir: format!("downloads/{}", name.to_lowercase().replace(' ', "_")),
            packet_interval_ms: 100,
        }
    }

    /// 파일에서 타겟 장치 목록 불러오기 (없으면 기본 타겟 1개 생성)
    pub fn load_or_default(path: &std::path::Path) -> Vec<Self> {
        if path.exists() {
            if let Ok(file) = std::fs::File::open(path) {
                let reader = std::io::BufReader::new(file);
                if let Ok(targets) = serde_json::from_reader::<_, Vec<Self>>(reader) {
                    if !targets.is_empty() {
                        return targets;
                    }
                }
            }
        }
        vec![Self::new_default(
            1,
            "Target-Station-01",
            "127.0.0.1".parse().unwrap(),
        )]
    }

    /// 타겟 장치 목록을 파일에 영구 저장
    pub fn save_to_file(path: &std::path::Path, targets: &[Self]) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, targets)?;
        Ok(())
    }
}

#[allow(dead_code)]
pub struct TargetDeviceRuntime {
    pub config: TargetDeviceConfig,
    pub status: ConnectionStatus,
    pub last_packet_time: Option<Instant>,
    pub total_packets_received: u64,
    pub total_packets_lost: u64,
    pub last_sequence: Option<u32>,
    pub loss_rate_percent: f32,
    pub is_recording: bool,
    pub recording_session: Option<String>,
}

impl TargetDeviceRuntime {
    pub fn new(config: TargetDeviceConfig) -> Self {
        Self {
            config,
            status: ConnectionStatus::Offline,
            last_packet_time: None,
            total_packets_received: 0,
            total_packets_lost: 0,
            last_sequence: None,
            loss_rate_percent: 0.0,
            is_recording: false,
            recording_session: None,
        }
    }

    pub fn record_packet(&mut self, sequence: u32) {
        self.total_packets_received += 1;
        self.last_packet_time = Some(Instant::now());
        self.status = ConnectionStatus::Normal;

        if let Some(prev_seq) = self.last_sequence {
            if sequence > prev_seq + 1 {
                let lost = (sequence - prev_seq - 1) as u64;
                self.total_packets_lost += lost;
            }
        }
        self.last_sequence = Some(sequence);

        let total = self.total_packets_received + self.total_packets_lost;
        if total > 0 {
            self.loss_rate_percent = (self.total_packets_lost as f32 / total as f32) * 100.0;
        }
    }

    pub fn update_status(&mut self) {
        if let Some(last) = self.last_packet_time {
            let elapsed = last.elapsed().as_millis();
            if elapsed < 300 {
                self.status = ConnectionStatus::Normal;
            } else if elapsed < 1200 {
                self.status = ConnectionStatus::Delayed;
            } else {
                self.status = ConnectionStatus::Offline;
            }
        } else {
            self.status = ConnectionStatus::Offline;
        }
    }
}
