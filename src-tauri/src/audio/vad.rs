use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use crate::utils::error::{AudioError, AudioResult};
use super::types::VADResult;

/// Voice Activity Detection using energy-based detection with smoothing
/// Optimized for <5ms processing time as per PRD requirements
pub struct VoiceActivityDetector {
    /// Sample rate for time calculations
    sample_rate: u32,
    
    /// Energy threshold for speech detection (0.0 - 1.0)
    energy_threshold: f32,
    
    /// Minimum energy threshold (noise floor)
    noise_floor: f32,
    
    /// Frame size in samples
    frame_size: usize,
    
    /// Smoothing window for energy levels
    energy_window: VecDeque<f32>,
    
    /// Window size for smoothing (in frames)
    window_size: usize,
    
    /// Speech state smoothing
    speech_history: VecDeque<bool>,
    
    /// Minimum consecutive speech frames
    min_speech_frames: usize,
    
    /// Minimum consecutive silence frames
    min_silence_frames: usize,
    
    /// Current speech state
    is_speech: bool,
    
    /// Adaptive noise level estimation
    noise_level: f32,
    
    /// Noise estimation alpha (learning rate)
    noise_alpha: f32,
    
    /// Statistics for monitoring
    total_frames: u64,
    speech_frames: u64,
}

impl VoiceActivityDetector {
    /// Create new VAD with configurable sensitivity
    pub fn new(
        sample_rate: u32, 
        energy_threshold: f32,
        frame_duration: Duration,
    ) -> AudioResult<Self> {
        let frame_size = (sample_rate as f64 * frame_duration.as_secs_f64()) as usize;
        
        if frame_size == 0 {
            return Err(AudioError::VADError("Frame size cannot be zero".to_string()));
        }
        
        // Smoothing window size (5 frames for ~50ms smoothing at 10ms frames)
        let window_size = 5;
        let speech_history_size = 3; // 3 frames for speech state smoothing
        
        debug!("🎙️  Initializing VAD: {}Hz, {:.3}s frames, {:.2} threshold", 
               sample_rate, frame_duration.as_secs_f64(), energy_threshold);
        
        Ok(Self {
            sample_rate,
            energy_threshold,
            noise_floor: 0.01, // Initial noise floor
            frame_size,
            energy_window: VecDeque::with_capacity(window_size),
            window_size,
            speech_history: VecDeque::with_capacity(speech_history_size),
            min_speech_frames: 2,  // Minimum 2 frames to confirm speech
            min_silence_frames: 3, // Minimum 3 frames to confirm silence
            is_speech: false,
            noise_level: 0.01,
            noise_alpha: 0.1, // Slow adaptation to noise changes
            total_frames: 0,
            speech_frames: 0,
        })
    }
    
    /// Process audio frame and detect speech activity
    /// Must complete in <5ms as per PRD requirements
    pub fn process_frame(&mut self, audio_data: &[f32]) -> AudioResult<VADResult> {
        let start_time = Instant::now();
        
        if audio_data.is_empty() {
            return Ok(VADResult {
                is_speech: false,
                confidence: 0.0,
                energy_level: 0.0,
                processing_time_us: start_time.elapsed().as_micros() as u64,
            });
        }
        
        // Calculate RMS energy
        let energy = self.calculate_rms_energy(audio_data);
        
        // Update noise level estimation (adaptive)
        if !self.is_speech {
            self.noise_level = self.noise_level * (1.0 - self.noise_alpha) + energy * self.noise_alpha;
        }
        
        // Add energy to smoothing window
        self.energy_window.push_back(energy);
        if self.energy_window.len() > self.window_size {
            self.energy_window.pop_front();
        }
        
        // Calculate smoothed energy
        let smoothed_energy = self.energy_window.iter().sum::<f32>() / self.energy_window.len() as f32;
        
        // Dynamic threshold based on noise level
        let dynamic_threshold = (self.noise_level * 2.0).max(self.energy_threshold).max(self.noise_floor);
        
        // Speech detection with hysteresis
        let raw_speech = smoothed_energy > dynamic_threshold;
        
        // Add to speech history for smoothing
        self.speech_history.push_back(raw_speech);
        if self.speech_history.len() > self.min_speech_frames.max(self.min_silence_frames) {
            self.speech_history.pop_front();
        }
        
        // State machine for speech detection with hysteresis
        let new_speech_state = self.determine_speech_state();
        self.is_speech = new_speech_state;
        
        // Calculate confidence based on energy ratio
        let confidence = if smoothed_energy > dynamic_threshold {
            ((smoothed_energy - dynamic_threshold) / dynamic_threshold).min(1.0)
        } else {
            0.0
        };
        
        // Update statistics
        self.total_frames += 1;
        if new_speech_state {
            self.speech_frames += 1;
        }
        
        let processing_time = start_time.elapsed();
        
        // Performance warning
        if processing_time > Duration::from_millis(5) {
            warn!("⚠️  VAD processing too slow: {:?}", processing_time);
        }
        
        Ok(VADResult {
            is_speech: new_speech_state,
            confidence,
            energy_level: smoothed_energy,
            processing_time_us: processing_time.as_micros() as u64,
        })
    }
    
    /// Calculate RMS energy of audio frame (optimized for speed)
    fn calculate_rms_energy(&self, audio_data: &[f32]) -> f32 {
        if audio_data.is_empty() {
            return 0.0;
        }
        
        // Fast RMS calculation using SIMD-friendly operations
        let sum_squares: f32 = audio_data.iter()
            .map(|&sample| sample * sample)
            .sum();
        
        (sum_squares / audio_data.len() as f32).sqrt()
    }
    
    /// Determine speech state using hysteresis and smoothing
    fn determine_speech_state(&self) -> bool {
        if self.speech_history.len() < self.min_speech_frames {
            return self.is_speech; // Keep current state if not enough history
        }
        
        let recent_speech_count = self.speech_history.iter()
            .take(self.min_speech_frames)
            .filter(|&&speech| speech)
            .count();
        
        let recent_silence_count = self.speech_history.iter()
            .take(self.min_silence_frames)
            .filter(|&&speech| !speech)
            .count();
        
        if !self.is_speech {
            // Currently silent - need min_speech_frames consecutive speech to activate
            recent_speech_count >= self.min_speech_frames
        } else {
            // Currently speaking - need min_silence_frames consecutive silence to deactivate
            !(recent_silence_count >= self.min_silence_frames)
        }
    }
    
    /// Update VAD sensitivity (0.0 - 1.0)
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.energy_threshold = sensitivity.clamp(0.0, 1.0);
        debug!("🎛️  VAD sensitivity updated: {:.2}", self.energy_threshold);
    }
    
    /// Get current VAD statistics
    pub fn get_stats(&self) -> VADStats {
        let speech_ratio = if self.total_frames > 0 {
            self.speech_frames as f32 / self.total_frames as f32
        } else {
            0.0
        };
        
        VADStats {
            total_frames: self.total_frames,
            speech_frames: self.speech_frames,
            speech_ratio,
            current_noise_level: self.noise_level,
            current_threshold: self.energy_threshold,
            is_speech: self.is_speech,
            avg_processing_time_us: 0, // Would need to track this separately
        }
    }
    
    /// Reset VAD state (useful for new audio sessions)
    pub fn reset(&mut self) {
        self.energy_window.clear();
        self.speech_history.clear();
        self.is_speech = false;
        self.noise_level = 0.01;
        self.total_frames = 0;
        self.speech_frames = 0;
        
        debug!("🔄 VAD state reset");
    }
    
    /// Configure VAD parameters for different environments
    pub fn configure_for_environment(&mut self, environment: VADEnvironment) {
        match environment {
            VADEnvironment::Quiet => {
                self.energy_threshold = 0.3;
                self.min_speech_frames = 1;
                self.min_silence_frames = 2;
                self.noise_alpha = 0.05; // Slow noise adaptation
            },
            VADEnvironment::Normal => {
                self.energy_threshold = 0.5;
                self.min_speech_frames = 2;
                self.min_silence_frames = 3;
                self.noise_alpha = 0.1; // Normal noise adaptation
            },
            VADEnvironment::Noisy => {
                self.energy_threshold = 0.7;
                self.min_speech_frames = 3;
                self.min_silence_frames = 4;
                self.noise_alpha = 0.2; // Fast noise adaptation
            },
        }
        
        debug!("🌍 VAD configured for {:?} environment", environment);
    }
}

/// VAD statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct VADStats {
    pub total_frames: u64,
    pub speech_frames: u64,
    pub speech_ratio: f32,
    pub current_noise_level: f32,
    pub current_threshold: f32,
    pub is_speech: bool,
    pub avg_processing_time_us: u64,
}

/// Environment presets for VAD configuration
#[derive(Debug, Clone)]
pub enum VADEnvironment {
    Quiet,  // Library, office
    Normal, // Home, normal room
    Noisy,  // Cafe, street, open office
}
