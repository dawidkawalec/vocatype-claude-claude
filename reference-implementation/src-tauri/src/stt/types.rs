use serde::{Deserialize, Serialize};

/// STT (Speech-to-Text) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STTConfig {
    /// Whisper model to use (tiny, base, small, medium, large)
    pub model_size: WhisperModelSize,
    
    /// Language code (e.g., "en", "pl", "auto" for auto-detection)
    pub language: String,
    
    /// Enable automatic punctuation
    pub enable_punctuation: bool,
    
    /// Maximum audio duration to process (seconds)
    pub max_duration: u32,
    
    /// Temperature for sampling (0.0 = deterministic)
    pub temperature: f32,
    
    /// Beam size for beam search
    pub beam_size: usize,
}

impl Default for STTConfig {
    fn default() -> Self {
        Self {
            model_size: WhisperModelSize::Base, // Good balance of speed/accuracy
            language: "auto".to_string(),       // Auto-detect language
            enable_punctuation: true,
            max_duration: 30,                   // Max 30 seconds
            temperature: 0.0,                   // Deterministic
            beam_size: 5,
        }
    }
}

/// Available Whisper model sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WhisperModelSize {
    Tiny,   // ~39 MB, fastest
    Base,   // ~74 MB, good balance
    Small,  // ~244 MB, better accuracy
    Medium, // ~769 MB, high accuracy
    Large,  // ~1550 MB, highest accuracy
}

impl WhisperModelSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            WhisperModelSize::Tiny => "tiny",
            WhisperModelSize::Base => "base", 
            WhisperModelSize::Small => "small",
            WhisperModelSize::Medium => "medium",
            WhisperModelSize::Large => "large",
        }
    }
    
    pub fn model_filename(&self) -> String {
        format!("ggml-{}.bin", self.as_str())
    }
    
    /// Estimated processing speed (rough multiplier of real-time)
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            WhisperModelSize::Tiny => 32.0,   // Very fast
            WhisperModelSize::Base => 16.0,   // Fast  
            WhisperModelSize::Small => 6.0,   // Medium
            WhisperModelSize::Medium => 3.0,  // Slower
            WhisperModelSize::Large => 1.0,   // Slowest
        }
    }
}

/// STT transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Transcribed text
    pub text: String,
    
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    
    /// Detected language (if auto-detection was used)
    pub detected_language: Option<String>,
    
    /// Individual word segments with timestamps
    pub segments: Vec<TranscriptionSegment>,
    
    /// Audio duration that was processed
    pub audio_duration_ms: u64,
}

/// Individual transcription segment with timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Start time in milliseconds
    pub start_ms: u64,
    
    /// End time in milliseconds  
    pub end_ms: u64,
    
    /// Text content of this segment
    pub text: String,
    
    /// Confidence score for this segment
    pub confidence: f32,
}

/// STT processing statistics for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STTStats {
    /// Total transcriptions processed
    pub total_transcriptions: u64,
    
    /// Average processing time (ms)
    pub avg_processing_time_ms: f64,
    
    /// Fastest processing time (ms)
    pub min_processing_time_ms: u64,
    
    /// Slowest processing time (ms) 
    pub max_processing_time_ms: u64,
    
    /// Current model being used
    pub current_model: WhisperModelSize,
    
    /// Model load time (ms)
    pub model_load_time_ms: u64,
    
    /// Memory usage estimate (MB)
    pub memory_usage_mb: f32,
}
