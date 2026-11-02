use crate::protocol::commands::ChannelConfigItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelAlarmLevel {
    Normal,
    Warning,
    Alarm,
}

#[derive(Debug, Clone)]
pub struct ChannelState {
    pub config: ChannelConfigItem,
    pub raw_value: f32,
    pub scaled_value: f32,
    pub alarm_level: ChannelAlarmLevel,
    pub last_update_ms: u64,
}

impl ChannelState {
    pub fn new(config: ChannelConfigItem) -> Self {
        Self {
            config,
            raw_value: 0.0,
            scaled_value: 0.0,
            alarm_level: ChannelAlarmLevel::Normal,
            last_update_ms: 0,
        }
    }

    pub fn update_value(&mut self, raw: f32, timestamp: u64) {
        self.raw_value = raw;
        self.scaled_value = self.config.gain * raw + self.config.offset;
        self.last_update_ms = timestamp;

        // Check alarms
        let val = self.scaled_value;
        if val <= self.config.alarm_low || val >= self.config.alarm_high {
            self.alarm_level = ChannelAlarmLevel::Alarm;
        } else if val <= self.config.warn_low || val >= self.config.warn_high {
            self.alarm_level = ChannelAlarmLevel::Warning;
        } else {
            self.alarm_level = ChannelAlarmLevel::Normal;
        }
    }
}
