use crate::model::target::DataTransportProtocol;
use crate::protocol::commands::{ChannelConfigItem, ControlCommand, ControlResponse};
use crate::protocol::packet::MeasurementPacket;
use rand::Rng;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct TargetSimulatorConfig {
    pub device_id: u16,
    pub host_ip: String,
    pub host_udp_port: u16,
    pub host_tcp_port: u16,
    pub listen_control_port: u16,
    pub channel_count: usize,
    pub transport: DataTransportProtocol,
    pub interval_ms: u64,
}

impl Default for TargetSimulatorConfig {
    fn default() -> Self {
        Self {
            device_id: 1,
            host_ip: "127.0.0.1".to_string(),
            host_udp_port: 8000,
            host_tcp_port: 8001,
            listen_control_port: 9000,
            channel_count: 100,
            transport: DataTransportProtocol::Udp,
            interval_ms: 100,
        }
    }
}

pub struct TargetSimulator {
    config: Arc<Mutex<TargetSimulatorConfig>>,
    is_running: Arc<AtomicBool>,
    is_recording: Arc<AtomicBool>,
    active_session: Arc<Mutex<Option<String>>>,
    channels: Arc<Mutex<Vec<ChannelConfigItem>>>,
}

impl TargetSimulator {
    pub fn new(config: TargetSimulatorConfig) -> Self {
        let channel_count = config.channel_count;
        let mut ch_list = Vec::with_capacity(channel_count);
        for i in 0..channel_count {
            ch_list.push(ChannelConfigItem::default_for(i));
        }

        Self {
            config: Arc::new(Mutex::new(config)),
            is_running: Arc::new(AtomicBool::new(false)),
            is_recording: Arc::new(AtomicBool::new(false)),
            active_session: Arc::new(Mutex::new(None)),
            channels: Arc::new(Mutex::new(ch_list)),
        }
    }

    pub fn set_transport(&self, transport: DataTransportProtocol) {
        if let Ok(mut cfg) = self.config.lock() {
            cfg.transport = transport;
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn start(&self) {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return; // Already running
        }

        let is_running_data = Arc::clone(&self.is_running);
        let config_data = Arc::clone(&self.config);
        let is_recording_flag = Arc::clone(&self.is_recording);

        // 1. Data streaming thread (UDP or TCP based on config)
        thread::spawn(move || {
            let mut sequence: u32 = 0;
            let start_time = Instant::now();
            let mut tcp_stream: Option<TcpStream> = None;

            while is_running_data.load(Ordering::Relaxed) {
                let (device_id, host_ip, udp_port, tcp_port, ch_count, transport, interval_ms) = {
                    let cfg = config_data.lock().unwrap();
                    (
                        cfg.device_id,
                        cfg.host_ip.clone(),
                        cfg.host_udp_port,
                        cfg.host_tcp_port,
                        cfg.channel_count,
                        cfg.transport,
                        cfg.interval_ms,
                    )
                };

                let t = start_time.elapsed().as_secs_f64();
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                // Synthesize 100 channels with realistic sensor signals
                let mut data = Vec::with_capacity(ch_count);
                let mut rng = rand::thread_rng();

                for ch in 0..ch_count {
                    let noise = rng.gen_range(-0.1f32..0.1f32);
                    let val = match ch % 5 {
                        0 => 5.0 * (t * 2.0 * std::f64::consts::PI * 0.5 + ch as f64).sin() as f32 + noise,
                        1 => 10.0 + 3.0 * (t * 0.2).sin() as f32 + (ch as f32 * 0.1) + noise,
                        2 => 45.0 + 10.0 * (t * 0.05).sin() as f32 + (ch as f32 * 0.2) + noise * 0.5,
                        3 => 25.0 + 15.0 * (t * 1.5).cos() as f32 + noise * 0.8,
                        _ => 2000.0 + 300.0 * (t * 0.8).sin() as f32 + noise * 5.0,
                    };
                    data.push(val);
                }

                let mut status_flags = 0u16;
                if is_recording_flag.load(Ordering::Relaxed) {
                    status_flags |= 0x0002; // Bit 1: Target Recording Active
                }

                let packet = MeasurementPacket::new(device_id, sequence, now_ms, status_flags, data);
                let bytes = packet.to_bytes();
                sequence = sequence.wrapping_add(1);

                match transport {
                    DataTransportProtocol::Udp => {
                        let target_addr = format!("{}:{}", host_ip, udp_port);
                        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
                            let _ = socket.send_to(&bytes, &target_addr);
                        }
                    }
                    DataTransportProtocol::Tcp => {
                        let target_addr = format!("{}:{}", host_ip, tcp_port);
                        let mut needs_reconnect = false;

                        if let Some(ref mut s) = tcp_stream {
                            if let Err(_) = s.write_all(&bytes) {
                                needs_reconnect = true;
                            }
                        } else {
                            needs_reconnect = true;
                        }

                        if needs_reconnect {
                            match TcpStream::connect_timeout(
                                &target_addr.parse().unwrap(),
                                Duration::from_millis(200),
                            ) {
                                Ok(mut s) => {
                                    let _ = s.write_all(&bytes);
                                    tcp_stream = Some(s);
                                }
                                Err(_) => {
                                    tcp_stream = None;
                                }
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(interval_ms));
            }
        });

        // 2. Control command listener thread (Target Listen on 9000)
        let is_running_ctrl = Arc::clone(&self.is_running);
        let config_ctrl = Arc::clone(&self.config);
        let channels_ctrl = Arc::clone(&self.channels);
        let is_recording_ctrl = Arc::clone(&self.is_recording);
        let active_session_ctrl = Arc::clone(&self.active_session);

        thread::spawn(move || {
            let listen_port = {
                let cfg = config_ctrl.lock().unwrap();
                cfg.listen_control_port
            };

            let listener = match TcpListener::bind(format!("0.0.0.0:{}", listen_port)) {
                Ok(l) => {
                    let _ = l.set_nonblocking(true);
                    l
                }
                Err(e) => {
                    eprintln!("Failed to bind simulator control port {}: {}", listen_port, e);
                    return;
                }
            };

            let start_time = Instant::now();

            while is_running_ctrl.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _addr)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        let mut line = String::new();

                        if let Ok(_) = reader.read_line(&mut line) {
                            if let Ok(cmd) = ControlCommand::from_json_str(&line) {
                                let response = match cmd {
                                    ControlCommand::RecStart { session_id, .. } => {
                                        is_recording_ctrl.store(true, Ordering::SeqCst);
                                        *active_session_ctrl.lock().unwrap() = Some(session_id.clone());
                                        ControlResponse::Ack {
                                            cmd: "CMD_REC_START".to_string(),
                                            success: true,
                                            message: format!("Recording started for session {}", session_id),
                                        }
                                    }
                                    ControlCommand::RecStop { session_id } => {
                                        is_recording_ctrl.store(false, Ordering::SeqCst);
                                        *active_session_ctrl.lock().unwrap() = None;
                                        ControlResponse::Ack {
                                            cmd: "CMD_REC_STOP".to_string(),
                                            success: true,
                                            message: format!("Recording stopped for session {}", session_id),
                                        }
                                    }
                                    ControlCommand::CfgSet { channels: new_channels } => {
                                        let mut guard = channels_ctrl.lock().unwrap();
                                        *guard = new_channels;
                                        ControlResponse::Ack {
                                            cmd: "CMD_CFG_SET".to_string(),
                                            success: true,
                                            message: "Parameters applied successfully".to_string(),
                                        }
                                    }
                                    ControlCommand::CfgGet => {
                                        let guard = channels_ctrl.lock().unwrap();
                                        ControlResponse::CfgData {
                                            channels: guard.clone(),
                                        }
                                    }
                                    ControlCommand::Ping => {
                                        let cfg = config_ctrl.lock().unwrap();
                                        ControlResponse::Pong {
                                            device_id: cfg.device_id,
                                            is_recording: is_recording_ctrl.load(Ordering::Relaxed),
                                            active_session: active_session_ctrl.lock().unwrap().clone(),
                                            uptime_secs: start_time.elapsed().as_secs(),
                                        }
                                    }
                                };

                                if let Ok(resp_json) = response.to_json_line() {
                                    let _ = stream.write_all(resp_json.as_bytes());
                                    let _ = stream.flush();
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}
