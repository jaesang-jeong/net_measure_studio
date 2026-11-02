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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDeviceConfig {
    pub device_id: u16,
    pub name: String,
    pub ip: IpAddr,
    pub data_port: u16,       // UDP / TCP data port
    pub control_port: u16,    // TCP control command port
    pub sftp_port: u16,       // SFTP file transfer port
    pub transport: DataTransportProtocol,
    pub enabled: bool,
}

impl TargetDeviceConfig {
    pub fn new_default(device_id: u16, name: &str, ip: IpAddr) -> Self {
        Self {
            device_id,
            name: name.to_string(),
            ip,
            data_port: 8000,
            control_port: 9000,
            sftp_port: 22,
            transport: DataTransportProtocol::Udp,
            enabled: true,
        }
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
