use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelConfigItem {
    pub channel_index: usize,
    pub name: String,
    pub unit: String,
    pub sensor_type: String,
    pub gain: f32,       // Y = gain * X + offset
    pub offset: f32,
    pub warn_low: f32,
    pub warn_high: f32,
    pub alarm_low: f32,
    pub alarm_high: f32,
}

impl ChannelConfigItem {
    pub fn default_for(index: usize) -> Self {
        let (name, unit, sensor_type, warn_low, warn_high, alarm_low, alarm_high) = match index % 5 {
            0 => (format!("CH{:03}_Voltage", index + 1), "V".to_string(), "Voltage".to_string(), -10.0, 10.0, -12.0, 12.0),
            1 => (format!("CH{:03}_Current", index + 1), "A".to_string(), "Current".to_string(), 0.0, 15.0, -1.0, 20.0),
            2 => (format!("CH{:03}_Temp", index + 1), "°C".to_string(), "Thermocouple".to_string(), -20.0, 80.0, -30.0, 100.0),
            3 => (format!("CH{:03}_Pressure", index + 1), "bar".to_string(), "Pressure".to_string(), 0.0, 50.0, -2.0, 60.0),
            _ => (format!("CH{:03}_RPM", index + 1), "rpm".to_string(), "Encoder".to_string(), 0.0, 3000.0, 0.0, 3500.0),
        };

        Self {
            channel_index: index,
            name,
            unit,
            sensor_type,
            gain: 1.0,
            offset: 0.0,
            warn_low,
            warn_high,
            alarm_low,
            alarm_high,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "cmd", content = "payload")]
pub enum ControlCommand {
    #[serde(rename = "CMD_REC_START")]
    RecStart {
        session_id: String,
        file_prefix: String,
        interval_ms: u32,
    },
    #[serde(rename = "CMD_REC_STOP")]
    RecStop {
        session_id: String,
    },
    #[serde(rename = "CMD_CFG_SET")]
    CfgSet {
        channels: Vec<ChannelConfigItem>,
    },
    #[serde(rename = "CMD_CFG_GET")]
    CfgGet,
    #[serde(rename = "CMD_PING")]
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "res", content = "payload")]
pub enum ControlResponse {
    #[serde(rename = "RES_ACK")]
    Ack {
        cmd: String,
        success: bool,
        message: String,
    },
    #[serde(rename = "RES_CFG_DATA")]
    CfgData {
        channels: Vec<ChannelConfigItem>,
    },
    #[serde(rename = "RES_PONG")]
    Pong {
        device_id: u16,
        is_recording: bool,
        active_session: Option<String>,
        uptime_secs: u64,
    },
}

impl ControlCommand {
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        Ok(s)
    }

    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s.trim())
    }
}

impl ControlResponse {
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        Ok(s)
    }

    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s.trim())
    }
}
