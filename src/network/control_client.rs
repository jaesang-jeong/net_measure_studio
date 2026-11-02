use crate::protocol::commands::{ChannelConfigItem, ControlCommand, ControlResponse};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub struct ControlClient;

impl ControlClient {
    pub fn send_command(
        addr: SocketAddr,
        cmd: ControlCommand,
        timeout: Duration,
    ) -> Result<ControlResponse, String> {
        let stream = TcpStream::connect_timeout(&addr, timeout)
            .map_err(|e| format!("Failed to connect to control port {}: {}", addr, e))?;

        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| format!("Set read timeout error: {}", e))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| format!("Set write timeout error: {}", e))?;

        let mut reader = BufReader::new(
            stream
                .try_clone()
                .map_err(|e| format!("Clone stream error: {}", e))?,
        );
        let mut writer = stream;

        let json_line = cmd
            .to_json_line()
            .map_err(|e| format!("Serialize error: {}", e))?;
        writer
            .write_all(json_line.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        writer.flush().map_err(|e| format!("Flush error: {}", e))?;

        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| format!("Read response error: {}", e))?;

        ControlResponse::from_json_str(&response_line)
            .map_err(|e| format!("Parse response error: {}", e))
    }

    pub fn start_recording(
        addr: SocketAddr,
        session_id: String,
        prefix: String,
        interval_ms: u32,
    ) -> Result<bool, String> {
        let cmd = ControlCommand::RecStart {
            session_id,
            file_prefix: prefix,
            interval_ms,
        };
        match Self::send_command(addr, cmd, Duration::from_secs(3))? {
            ControlResponse::Ack { success, message, .. } => {
                if success {
                    Ok(true)
                } else {
                    Err(format!("Target rejected REC_START: {}", message))
                }
            }
            other => Err(format!("Unexpected response: {:?}", other)),
        }
    }

    pub fn stop_recording(addr: SocketAddr, session_id: String) -> Result<bool, String> {
        let cmd = ControlCommand::RecStop { session_id };
        match Self::send_command(addr, cmd, Duration::from_secs(3))? {
            ControlResponse::Ack { success, message, .. } => {
                if success {
                    Ok(true)
                } else {
                    Err(format!("Target rejected REC_STOP: {}", message))
                }
            }
            other => Err(format!("Unexpected response: {:?}", other)),
        }
    }

    pub fn get_config(addr: SocketAddr) -> Result<Vec<ChannelConfigItem>, String> {
        let cmd = ControlCommand::CfgGet;
        match Self::send_command(addr, cmd, Duration::from_secs(3))? {
            ControlResponse::CfgData { channels } => Ok(channels),
            ControlResponse::Ack { message, .. } => Err(format!("Target error: {}", message)),
            other => Err(format!("Unexpected response: {:?}", other)),
        }
    }

    pub fn set_config(addr: SocketAddr, channels: Vec<ChannelConfigItem>) -> Result<bool, String> {
        let cmd = ControlCommand::CfgSet { channels };
        match Self::send_command(addr, cmd, Duration::from_secs(3))? {
            ControlResponse::Ack { success, message, .. } => {
                if success {
                    Ok(true)
                } else {
                    Err(format!("Target rejected CFG_SET: {}", message))
                }
            }
            other => Err(format!("Unexpected response: {:?}", other)),
        }
    }
}
