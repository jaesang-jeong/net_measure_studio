use crate::protocol::packet::{extract_packet_from_stream_opt, MeasurementPacket};
use crossbeam_channel::Sender;
use std::io::Read;
use std::net::{TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct NetworkReceiver {
    packet_tx: Sender<MeasurementPacket>,
    is_running: Arc<AtomicBool>,
    verify_crc: Arc<AtomicBool>,
    udp_handle: Option<thread::JoinHandle<()>>,
    tcp_handle: Option<thread::JoinHandle<()>>,
}

impl NetworkReceiver {
    pub fn new(packet_tx: Sender<MeasurementPacket>, verify_crc: Arc<AtomicBool>) -> Self {
        Self {
            packet_tx,
            is_running: Arc::new(AtomicBool::new(false)),
            verify_crc,
            udp_handle: None,
            tcp_handle: None,
        }
    }

    /// Start both UDP (default: 8000) and TCP (default: 8001) measurement data receivers
    pub fn start(&mut self, udp_bind_addr: &str, tcp_bind_addr: &str) -> std::io::Result<()> {
        self.is_running.store(true, Ordering::SeqCst);

        // 1. Start UDP Receiver
        let udp_socket = UdpSocket::bind(udp_bind_addr)?;
        udp_socket.set_read_timeout(Some(Duration::from_millis(50)))?;
        let is_running_udp = Arc::clone(&self.is_running);
        let verify_crc_udp = Arc::clone(&self.verify_crc);
        let tx_udp = self.packet_tx.clone();

        let udp_handle = thread::spawn(move || {
            let mut buf = [0u8; 65535];
            while is_running_udp.load(Ordering::Relaxed) {
                match udp_socket.recv_from(&mut buf) {
                    Ok((amt, _src)) => {
                        let verify = verify_crc_udp.load(Ordering::Relaxed);
                        if let Ok(packet) = MeasurementPacket::from_bytes_opt(&buf[0..amt], verify) {
                            let _ = tx_udp.send(packet);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                        continue;
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });
        self.udp_handle = Some(udp_handle);

        // 2. Start TCP Receiver (Handles MTU-exceeding packets with stream framing)
        let tcp_listener = TcpListener::bind(tcp_bind_addr)?;
        tcp_listener.set_nonblocking(true)?;
        let is_running_tcp = Arc::clone(&self.is_running);
        let verify_crc_tcp = Arc::clone(&self.verify_crc);
        let tx_tcp = self.packet_tx.clone();

        let tcp_handle = thread::spawn(move || {
            let mut active_connections: Vec<(std::net::TcpStream, Vec<u8>)> = Vec::new();

            while is_running_tcp.load(Ordering::Relaxed) {
                // Accept incoming TCP connections
                match tcp_listener.accept() {
                    Ok((stream, _addr)) => {
                        let _ = stream.set_nonblocking(true);
                        active_connections.push((stream, Vec::with_capacity(8192)));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No pending connection
                    }
                    Err(_) => {}
                }

                // Read from active connections
                let mut temp_buf = [0u8; 8192];
                let mut disconnected = Vec::new();

                for (idx, (stream, buffer)) in active_connections.iter_mut().enumerate() {
                    loop {
                        match stream.read(&mut temp_buf) {
                            Ok(0) => {
                                // Connection closed by peer
                                disconnected.push(idx);
                                break;
                            }
                            Ok(amt) => {
                                buffer.extend_from_slice(&temp_buf[0..amt]);
                                let verify = verify_crc_tcp.load(Ordering::Relaxed);
                                // Extract any complete packets from stream
                                while let Ok(Some(packet)) = extract_packet_from_stream_opt(buffer, verify) {
                                    let _ = tx_tcp.send(packet);
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                                break;
                            }
                            Err(_) => {
                                disconnected.push(idx);
                                break;
                            }
                        }
                    }
                }

                // Remove disconnected clients in reverse order
                for idx in disconnected.into_iter().rev() {
                    if idx < active_connections.len() {
                        active_connections.swap_remove(idx);
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }
        });
        self.tcp_handle = Some(tcp_handle);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(h) = self.udp_handle.take() {
            let _ = h.join();
        }
        if let Some(h) = self.tcp_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for NetworkReceiver {
    fn drop(&mut self) {
        self.stop();
    }
}
