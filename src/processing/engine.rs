use crate::model::channel::ChannelState;
use crate::model::target::{TargetDeviceConfig, TargetDeviceRuntime};
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

impl ProcessingEngineShared {
    /// 필요 채널 수(예: 200채널 등)에 맞춰 channels 및 ring_buffers를 실시간 동적 확장
    #[allow(dead_code)]
    pub fn ensure_channel_capacity(&self, needed_count: usize) {
        self.sync_channel_capacity(needed_count);
    }

    /// 타겟 장치들의 총 채널 수에 맞춰 channels 및 ring_buffers 크기를 정확히 동기화 (확장 및 축소)
    pub fn sync_channel_capacity(&self, needed_count: usize) {
        if needed_count == 0 {
            return;
        }
        let mut ch_guard = self.channels.write().unwrap();
        let mut rb_guard = self.ring_buffers.write().unwrap();
        let current = ch_guard.len();
        if needed_count > current {
            for i in current..needed_count {
                let cfg = crate::protocol::commands::ChannelConfigItem::default_for(i);
                ch_guard.push(ChannelState::new(cfg));
                rb_guard.push(ChannelRingBuffer::new(5000));
            }
        } else if needed_count < current {
            ch_guard.truncate(needed_count);
            rb_guard.truncate(needed_count);
        }
    }
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

                            // Update target device statistics (미등록 장치도 패킷 유입 시 즉시 런타임 자동 생성하여 수신 처리)
                            let (target_offset, target_limit, target_lost) = {
                                let target = tgt_guard.entry(pkt.device_id).or_insert_with(|| {
                                    let cfg = TargetDeviceConfig::new_default(
                                        pkt.device_id,
                                        &format!("Target-{:02}", pkt.device_id),
                                        "0.0.0.0".parse().unwrap(),
                                    );
                                    TargetDeviceRuntime::new(cfg)
                                });
                                target.record_packet(pkt.sequence);
                                (
                                    target.config.channel_offset,
                                    target.config.channel_count,
                                    target.total_packets_lost,
                                )
                            };
                            total_lost.store(target_lost, Ordering::Relaxed);
                            total_rx.fetch_add(1, Ordering::Relaxed);

                            // Update channel values and ring buffers (타겟별 channel_offset 및 channel_count 한도 적용하여 채널 충돌 방지 및 필요 시 동적 확장)
                            let needed_capacity = target_offset + pkt.data.len().min(target_limit);
                            if needed_capacity > ch_guard.len() {
                                for idx in ch_guard.len()..needed_capacity {
                                    let cfg = crate::protocol::commands::ChannelConfigItem::default_for(idx);
                                    ch_guard.push(ChannelState::new(cfg));
                                    rb_guard.push(ChannelRingBuffer::new(5000));
                                }
                            }
                            let ch_len = ch_guard.len();
                            for (i, &raw_val) in pkt.data.iter().take(target_limit).enumerate() {
                                let ch_idx = target_offset + i;
                                if ch_idx < ch_len {
                                    ch_guard[ch_idx].update_value(raw_val, pkt.timestamp);
                                    let scaled = ch_guard[ch_idx].scaled_value;
                                    rb_guard[ch_idx].push(rel_time, scaled);
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
