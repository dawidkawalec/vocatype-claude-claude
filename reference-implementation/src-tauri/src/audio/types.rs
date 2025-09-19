use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Audio configuration with performance targets from PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Sample rate - fixed at 16kHz as per PRD requirements
    pub sample_rate: u32,
    
    /// Number of channels - mono for STT processing
    pub channels: u16,
    
    /// Buffer size in samples - tuned for <10ms latency
    pub buffer_size: usize,
    
    /// Circular buffer duration - 30 seconds rolling window
    pub buffer_duration: Duration,
    
    /// VAD sensitivity threshold (0.0 - 1.0)
    pub vad_threshold: f32,
    
    /// VAD frame size in ms
    pub vad_frame_size: u64,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,  // 16kHz as required by PRD
            channels: 1,         // Mono for better STT performance
            buffer_size: 160,    // ~10ms at 16kHz for low latency
            buffer_duration: Duration::from_secs(30), // 30s rolling window
            vad_threshold: 0.5,  // Configurable sensitivity
            vad_frame_size: 10,  // 10ms frames for <5ms processing
        }
    }
}

/// Audio device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
    pub channels: u16,
    pub sample_rate: u32,
}

/// Audio frame for processing
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// Raw audio data as f32 samples
    pub data: Vec<f32>,
    
    /// Timestamp when frame was captured
    pub timestamp: std::time::Instant,
    
    /// Sample rate for this frame
    pub sample_rate: u32,
    
    /// Energy level for this frame (for VAD and visualization)
    pub energy_level: f32,
    
    /// Whether VAD detected speech in this frame
    pub is_speech: bool,
}

impl AudioFrame {
    /// Calculate RMS energy level for audio visualization
    pub fn calculate_energy(&mut self) {
        if self.data.is_empty() {
            self.energy_level = 0.0;
            return;
        }
        
        let sum_squares: f32 = self.data.iter().map(|&sample| sample * sample).sum();
        self.energy_level = (sum_squares / self.data.len() as f32).sqrt();
    }
    
    /// Convert to 16-bit PCM for STT processing
    pub fn to_pcm16(&self) -> Vec<i16> {
        self.data
            .iter()
            .map(|&sample| (sample * 32767.0).max(-32768.0).min(32767.0) as i16)
            .collect()
    }
}

/// Audio capture statistics for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStats {
    /// Current audio level (0.0 - 1.0)
    pub current_level: f32,
    
    /// Peak level in current session
    pub peak_level: f32,
    
    /// Average level over last second
    pub average_level: f32,
    
    /// Total frames processed
    pub frames_processed: u64,
    
    /// Frames dropped due to processing delays
    pub frames_dropped: u64,
    
    /// Current buffer usage percentage
    pub buffer_usage: f32,
    
    /// Processing latency in milliseconds
    pub processing_latency_ms: f32,
    
    /// VAD speech detection rate
    pub speech_detection_rate: f32,
}

impl Default for AudioStats {
    fn default() -> Self {
        Self {
            current_level: 0.0,
            peak_level: 0.0,
            average_level: 0.0,
            frames_processed: 0,
            frames_dropped: 0,
            buffer_usage: 0.0,
            processing_latency_ms: 0.0,
            speech_detection_rate: 0.0,
        }
    }
}

/// Audio capture state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureState {
    Stopped,
    Starting,
    Recording,
    Processing,
    Error,
}

/// VAD detection result
#[derive(Debug, Clone)]
pub struct VADResult {
    pub is_speech: bool,
    pub confidence: f32,
    pub energy_level: f32,
    pub processing_time_us: u64,
}
