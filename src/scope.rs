use std::collections::VecDeque;

/// Ring buffer storing the latest waveform samples for visualization.
pub struct ScopeBuffer {
    samples: VecDeque<f32>,
    capacity: usize,
}

impl ScopeBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn record(&mut self, block: &[f32]) {
        for &sample in block {
            if self.samples.len() == self.capacity {
                self.samples.pop_front();
            }
            self.samples.push_back(sample);
        }
    }

    pub fn snapshot(&self) -> Vec<f32> {
        self.samples.iter().copied().collect()
    }
}
