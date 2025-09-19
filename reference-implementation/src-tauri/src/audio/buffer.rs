use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use crate::utils::error::{AudioError, AudioResult};
use super::types::AudioFrame;

/// Thread-safe circular buffer for 30-second rolling audio window
/// Optimized for high-performance real-time audio processing
pub struct CircularBuffer {
    /// Raw audio data storage
    data: Arc<Mutex<Vec<f32>>>,
    
    /// Current write position (atomic for lock-free access)
    write_pos: AtomicUsize,
    
    /// Current read position (atomic for lock-free access)  
    read_pos: AtomicUsize,
    
    /// Buffer capacity in samples
    capacity: usize,
    
    /// Current number of valid samples
    size: AtomicUsize,
    
    /// Whether buffer is full (started overwriting)
    is_full: AtomicBool,
    
    /// Buffer creation timestamp
    created_at: Instant,
    
    /// Sample rate for time calculations
    sample_rate: u32,
}

impl CircularBuffer {
    /// Create new circular buffer with specified duration
    /// Duration should be 30 seconds as per PRD requirements
    pub fn new(sample_rate: u32, duration: Duration) -> AudioResult<Self> {
        let capacity = (sample_rate as f64 * duration.as_secs_f64()) as usize;
        
        if capacity == 0 {
            return Err(AudioError::CaptureInitFailed(
                "Buffer capacity cannot be zero".to_string()
            ));
        }
        
        debug!("🔊 Creating circular buffer: {} samples, {:.1}s duration", 
               capacity, duration.as_secs_f64());
        
        Ok(Self {
            data: Arc::new(Mutex::new(vec![0.0; capacity])),
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            capacity,
            size: AtomicUsize::new(0),
            is_full: AtomicBool::new(false),
            created_at: Instant::now(),
            sample_rate,
        })
    }
    
    /// Write audio frame to buffer (thread-safe)
    /// Returns number of samples written
    pub fn write_frame(&self, frame: &AudioFrame) -> AudioResult<usize> {
        let start_time = Instant::now();
        
        if frame.data.is_empty() {
            return Ok(0);
        }
        
        let mut data = self.data.lock().map_err(|_| {
            AudioError::CaptureInitFailed("Buffer mutex poisoned".to_string())
        })?;
        
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let samples_to_write = frame.data.len().min(self.capacity);
        
        // Handle wrap-around in circular buffer
        if write_pos + samples_to_write <= self.capacity {
            // No wrap-around needed
            data[write_pos..write_pos + samples_to_write]
                .copy_from_slice(&frame.data[..samples_to_write]);
        } else {
            // Split write across buffer boundary
            let first_chunk = self.capacity - write_pos;
            let second_chunk = samples_to_write - first_chunk;
            
            data[write_pos..].copy_from_slice(&frame.data[..first_chunk]);
            data[..second_chunk].copy_from_slice(&frame.data[first_chunk..samples_to_write]);
        }
        
        // Update atomic counters
        let new_write_pos = (write_pos + samples_to_write) % self.capacity;
        self.write_pos.store(new_write_pos, Ordering::Release);
        
        let current_size = self.size.load(Ordering::Acquire);
        if current_size + samples_to_write >= self.capacity {
            self.is_full.store(true, Ordering::Release);
            self.size.store(self.capacity, Ordering::Release);
        } else {
            self.size.store(current_size + samples_to_write, Ordering::Release);
        }
        
        let write_time = start_time.elapsed();
        if write_time > Duration::from_micros(100) {
            warn!("⚠️  Slow buffer write: {:?}", write_time);
        }
        
        Ok(samples_to_write)
    }
    
    /// Read recent audio data for STT processing
    /// Returns samples from last 'duration' seconds
    pub fn read_recent(&self, duration: Duration) -> AudioResult<Vec<f32>> {
        let samples_needed = (self.sample_rate as f64 * duration.as_secs_f64()) as usize;
        let samples_needed = samples_needed.min(self.size.load(Ordering::Acquire));
        
        if samples_needed == 0 {
            return Ok(Vec::new());
        }
        
        let data = self.data.lock().map_err(|_| {
            AudioError::CaptureInitFailed("Buffer mutex poisoned".to_string())
        })?;
        
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let mut result = Vec::with_capacity(samples_needed);
        
        // Calculate read start position
        let read_start = if write_pos >= samples_needed {
            write_pos - samples_needed
        } else if self.is_full.load(Ordering::Acquire) {
            self.capacity - (samples_needed - write_pos)
        } else {
            0
        };
        
        // Handle wrap-around in circular buffer
        if read_start + samples_needed <= self.capacity {
            // No wrap-around
            result.extend_from_slice(&data[read_start..read_start + samples_needed]);
        } else {
            // Handle wrap-around
            let first_chunk = self.capacity - read_start;
            let second_chunk = samples_needed - first_chunk;
            
            result.extend_from_slice(&data[read_start..]);
            result.extend_from_slice(&data[..second_chunk]);
        }
        
        Ok(result)
    }
    
    /// Get current buffer statistics
    pub fn get_stats(&self) -> BufferStats {
        let size = self.size.load(Ordering::Acquire);
        let usage_percent = (size as f32 / self.capacity as f32) * 100.0;
        let duration_stored = Duration::from_secs_f64(size as f64 / self.sample_rate as f64);
        
        BufferStats {
            capacity: self.capacity,
            current_size: size,
            usage_percent,
            is_full: self.is_full.load(Ordering::Acquire),
            duration_stored,
            write_position: self.write_pos.load(Ordering::Acquire),
            read_position: self.read_pos.load(Ordering::Acquire),
        }
    }
    
    /// Clear buffer contents (for reset)
    pub fn clear(&self) -> AudioResult<()> {
        let mut data = self.data.lock().map_err(|_| {
            AudioError::CaptureInitFailed("Buffer mutex poisoned".to_string())
        })?;
        
        data.fill(0.0);
        self.write_pos.store(0, Ordering::Release);
        self.read_pos.store(0, Ordering::Release);
        self.size.store(0, Ordering::Release);
        self.is_full.store(false, Ordering::Release);
        
        debug!("🧹 Buffer cleared");
        Ok(())
    }
    
    /// Check if buffer has enough data for processing
    pub fn has_sufficient_data(&self, min_duration: Duration) -> bool {
        let min_samples = (self.sample_rate as f64 * min_duration.as_secs_f64()) as usize;
        self.size.load(Ordering::Acquire) >= min_samples
    }
}

/// Buffer statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub capacity: usize,
    pub current_size: usize,
    pub usage_percent: f32,
    pub is_full: bool,
    pub duration_stored: Duration,
    pub write_position: usize,
    pub read_position: usize,
}

// Thread safety implementation
unsafe impl Send for CircularBuffer {}
unsafe impl Sync for CircularBuffer {}
