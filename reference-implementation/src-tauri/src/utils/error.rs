use thiserror::Error;

/// Main application error types with detailed error information
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),
    
    #[error("STT error: {0}")]
    STT(#[from] STTError),
    
    #[error("AI processing error: {0}")]
    AI(#[from] AIError),
    
    #[error("System integration error: {0}")]
    System(#[from] SystemError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
}

/// Audio-specific error types
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("No input device available")]
    NoInputDevice,
    
    #[error("Device not found: {device_name}")]
    DeviceNotFound { device_name: String },
    
    #[error("Audio format not supported: {details}")]
    UnsupportedFormat { details: String },
    
    #[error("Stream error: {0}")]
    StreamError(String),
    
    #[error("Processing timeout: {timeout_ms}ms")]
    ProcessingTimeout { timeout_ms: u64 },
    
    #[error("Buffer overflow - audio processing too slow")]
    BufferOverflow,
    
    #[error("VAD processing failed: {0}")]
    VADError(String),
    
    #[error("Audio capture initialization failed: {0}")]
    CaptureInitFailed(String),
}

/// Speech-to-Text error types
#[derive(Debug, Error)]
pub enum STTError {
    #[error("Whisper model not found")]
    ModelNotFound,
    
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    
    #[error("Model loading timeout")]
    ModelLoadTimeout,
    
    #[error("Audio preprocessing failed: {0}")]
    PreprocessingFailed(String),
}

/// AI processing error types
#[derive(Debug, Error)]
pub enum AIError {
    #[error("API request failed: {status_code}")]
    APIRequestFailed { status_code: u16 },
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Response parsing failed: {0}")]
    ResponseParsingFailed(String),
    
    #[error("Streaming connection lost")]
    StreamingConnectionLost,
    
    #[error("Invalid API key")]
    InvalidAPIKey,
}

/// System integration error types
#[derive(Debug, Error)]
pub enum SystemError {
    #[error("Hotkey registration failed: {key_combination}")]
    HotkeyRegistrationFailed { key_combination: String },
    
    #[error("Clipboard access failed")]
    ClipboardAccessFailed,
    
    #[error("Text selection failed")]
    TextSelectionFailed,
    
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    #[error("Platform not supported")]
    PlatformNotSupported,
}

// Type aliases for cleaner code
pub type AppResult<T> = Result<T, AppError>;
pub type AudioResult<T> = Result<T, AudioError>;
pub type STTResult<T> = Result<T, STTError>;
pub type AIResult<T> = Result<T, AIError>;
pub type SystemResult<T> = Result<T, SystemError>;

// Serde serialization for frontend communication
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
