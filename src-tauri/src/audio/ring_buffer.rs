//! Fixed-capacity audio ring buffer.
//!
//! Holds up to N seconds of 16 kHz mono audio. On overflow the oldest samples
//! are overwritten (records keep the most recent window rather than panicking).

/// Simple overwrite-on-overflow ring buffer of `f32` samples.
pub struct AudioRingBuffer {
    buffer: Vec<f32>,
    capacity: usize,
    head: usize,
    len: usize,
}

impl AudioRingBuffer {
    /// Allocate a buffer holding `seconds * sample_rate` samples.
    pub fn with_duration(sample_rate: u32, seconds: u32) -> Self {
        let capacity = (sample_rate as usize) * (seconds as usize);
        Self::with_capacity(capacity.max(1))
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            capacity,
            head: 0,
            len: 0,
        }
    }

    /// Push samples, overwriting the oldest if capacity is exceeded.
    pub fn push_slice(&mut self, samples: &[f32]) {
        for &s in samples {
            self.buffer[self.head] = s;
            self.head = (self.head + 1) % self.capacity;
            if self.len < self.capacity {
                self.len += 1;
            }
        }
    }

    /// Number of samples currently stored.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Copy stored samples in chronological order.
    pub fn to_vec(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.len);
        if self.len < self.capacity {
            // Not yet wrapped: data is [0, head).
            out.extend_from_slice(&self.buffer[..self.len]);
        } else {
            // Wrapped: oldest sample is at `head`.
            out.extend_from_slice(&self.buffer[self.head..]);
            out.extend_from_slice(&self.buffer[..self.head]);
        }
        out
    }

    /// Reset to empty without reallocating.
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_in_order_without_wrap() {
        let mut rb = AudioRingBuffer::with_capacity(4);
        rb.push_slice(&[1.0, 2.0, 3.0]);
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.to_vec(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn overwrites_oldest_on_overflow() {
        let mut rb = AudioRingBuffer::with_capacity(3);
        rb.push_slice(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.to_vec(), vec![3.0, 4.0, 5.0]);
    }

    #[test]
    fn clear_resets() {
        let mut rb = AudioRingBuffer::with_capacity(3);
        rb.push_slice(&[1.0, 2.0]);
        rb.clear();
        assert!(rb.is_empty());
        assert_eq!(rb.to_vec(), Vec::<f32>::new());
    }
}
