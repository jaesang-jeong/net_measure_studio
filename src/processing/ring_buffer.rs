#[derive(Debug, Clone)]
pub struct TimeSeriesPoint {
    pub time_secs: f64,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct ChannelRingBuffer {
    capacity: usize,
    buffer: Vec<TimeSeriesPoint>,
    head: usize,
    count: usize,
}

impl ChannelRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: Vec::with_capacity(capacity),
            head: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, time_secs: f64, value: f32) {
        let point = TimeSeriesPoint { time_secs, value };
        if self.buffer.len() < self.capacity {
            self.buffer.push(point);
            self.count += 1;
        } else {
            self.buffer[self.head] = point;
            self.head = (self.head + 1) % self.capacity;
        }
    }

    /// Extract points in chronological order
    pub fn get_points(&self) -> Vec<[f64; 2]> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(self.buffer.len());
        if self.buffer.len() < self.capacity {
            for pt in &self.buffer {
                result.push([pt.time_secs, pt.value as f64]);
            }
        } else {
            // Read from head to end, then from 0 to head
            for i in self.head..self.capacity {
                let pt = &self.buffer[i];
                result.push([pt.time_secs, pt.value as f64]);
            }
            for i in 0..self.head {
                let pt = &self.buffer[i];
                result.push([pt.time_secs, pt.value as f64]);
            }
        }
        result
    }

    /// Decimate points using Min-Max algorithm for high-performance chart rendering
    #[allow(dead_code)]
    pub fn get_decimated_points(&self, max_display_points: usize) -> Vec<[f64; 2]> {
        let pts = self.get_points();
        if pts.len() <= max_display_points || max_display_points == 0 {
            return pts;
        }

        let bucket_size = pts.len() / (max_display_points / 2).max(1);
        if bucket_size <= 1 {
            return pts;
        }

        let mut decimated = Vec::with_capacity(max_display_points);
        for chunk in pts.chunks(bucket_size) {
            if chunk.is_empty() {
                continue;
            }
            let mut min_pt = chunk[0];
            let mut max_pt = chunk[0];

            for &pt in chunk {
                if pt[1] < min_pt[1] {
                    min_pt = pt;
                }
                if pt[1] > max_pt[1] {
                    max_pt = pt;
                }
            }

            // Keep time ordering between min and max in chunk
            if min_pt[0] <= max_pt[0] {
                decimated.push(min_pt);
                if min_pt[0] != max_pt[0] {
                    decimated.push(max_pt);
                }
            } else {
                decimated.push(max_pt);
                if min_pt[0] != max_pt[0] {
                    decimated.push(min_pt);
                }
            }
        }
        decimated
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.head = 0;
        self.count = 0;
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}
