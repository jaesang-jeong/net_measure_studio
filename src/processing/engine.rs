use crate::model::channel::ChannelState;
use crate::model::target::TargetDeviceRuntime;
use crate::processing::ring_buffer::ChannelRingBuffer;
use crate::protocol::packet::MeasurementPacket;
use crossbeam_channel::Receiver;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Instant;

pub struct ProcessingEngineShared {
    pub channels: Arc<RwLock<Vec<ChannelState>>>,
    pub ring_buffers: Arc<RwLock<Vec<ChannelRingBuffer>>>,
    pub targets: Arc<RwLock<HashMap<u16, TargetDeviceRuntime>>>,
    pub total_rx_packets: Arc<AtomicU64>,
    pub total_lost_packets: Arc<AtomicU64>,
    pub start_time: Instant,
}

pub struct ProcessingEngine {
    pub shared: ProcessingEngineShared,
    is_running: Arc<AtomicBool>,
}

impl ProcessingEngine {
    pub fn new(
        channel_count: usize,
        ring_buffer_capacity: usize,
    ) -> (Self, ProcessingEngineShared) {
        let mut channels = Vec::with_capacity(channel_count);
        let mut ring_buffers = Vec::with_capacity(channel_count);

        for i in 0..channel_count {
            let cfg = crate::protocol::commands::ChannelConfigItem::default_for(i);
            channels.push(ChannelState::new(cfg));
            ring_buffers.push(ChannelRingBuffer::new(ring_buffer_capacity));
        }

        let shared = ProcessingEngineShared {
            channels: Arc::new(RwLock::new(channels)),
            ring_buffers: Arc::new(RwLock::new(ring_buffers)),
            targets: Arc::new(RwLock::new(HashMap::new())),
            total_rx_packets: Arc::new(AtomicU64::new(0)),
            total_lost_packets: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        };

        let shared_clone = ProcessingEngineShared {
            channels: Arc::clone(&shared.channels),
            ring_buffers: Arc::clone(&shared.ring_buffers),
            targets: Arc::clone(&shared.targets),
            total_rx_packets: Arc::clone(&shared.total_rx_packets),
            total_lost_packets: Arc::clone(&shared.total_lost_packets),
            start_time: shared.start_time,
        };

        (
            Self {
                shared,
                is_running: Arc::new(AtomicBool::new(false)),
            },
            shared_clone,
        )
    }

    pub fn start(&self, packet_rx: Receiver<MeasurementPacket>) {
        self.is_running.store(true, Ordering::SeqCst);
        let is_running = Arc::clone(&self.is_running);
        let channels_arc = Arc::clone(&self.shared.channels);
        let ring_buffers_arc = Arc::clone(&self.shared.ring_buffers);
        let targets_arc = Arc::clone(&self.shared.targets);
        let total_rx = Arc::clone(&self.shared.total_rx_packets);
        let total_lost = Arc::clone(&self.shared.total_lost_packets);
        let start_time = self.shared.start_time;

        thread::spawn(move || {
            let mut batch = Vec::with_capacity(64);

            while is_running.load(Ordering::Relaxed) {
                // Wait for at least one packet
                match packet_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(packet) => {
                        batch.push(packet);
                        // Drain up to 64 packets at once to minimize lock acquisition overhead
                        while let Ok(p) = packet_rx.try_recv() {
                            batch.push(p);
                            if batch.len() >= 64 {
                                break;
                            }
                        }

                        // Process batch
                        let mut ch_guard = channels_arc.write().unwrap();
                        let mut rb_guard = ring_buffers_arc.write().unwrap();
                        let mut tgt_guard = targets_arc.write().unwrap();

                        for pkt in batch.drain(..) {
                            let rel_time = start_time.elapsed().as_secs_f64();

                            // Update target device statistics
                            if let Some(target) = tgt_guard.get_mut(&pkt.device_id) {
                                target.record_packet(pkt.sequence);
                                total_lost.store(target.total_packets_lost, Ordering::Relaxed);
                            }
                            total_rx.fetch_add(1, Ordering::Relaxed);

                            // Update channel values and ring buffers
                            let ch_len = ch_guard.len();
                            for (idx, &raw_val) in pkt.data.iter().enumerate() {
                                if idx < ch_len {
                                    ch_guard[idx].update_value(raw_val, pkt.timestamp);
                                    let scaled = ch_guard[idx].scaled_value;
                                    rb_guard[idx].push(rel_time, scaled);
                                }
                            }
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        // Periodic status update for targets
                        if let Ok(mut tgt_guard) = targets_arc.try_write() {
                            for target in tgt_guard.values_mut() {
                                target.update_status();
                            }
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }
        });
    }

    #[allow(dead_code)]
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}
