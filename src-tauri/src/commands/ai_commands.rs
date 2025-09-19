use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::stt::{WhisperEngine, STTConfig, TranscriptionResult};
use crate::ai::{GeminiClient, AIConfig, AIRequest, AIResponse, AIInstruction};

// Remove shared state for now due to thread safety issues

/// Response for AI commands
#[derive(Debug, Serialize, Deserialize)]
pub struct AICommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> AICommandResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Process text with Gemini 2.5 Flash
#[tauri::command]
pub async fn process_with_gemini(
    text: String, 
    instruction: Option<String>,
    api_key: Option<String>,
) -> Result<AICommandResponse<AIResponse>, String> {
    info!("🤖 Processing text with Gemini: {} chars", text.len());
    
    if text.trim().is_empty() {
        return Ok(AICommandResponse::error("No text provided".to_string()));
    }
    
    // Create AI configuration
    let mut config = AIConfig::default();
    if let Some(key) = api_key {
        config.api_key = key;
    } else {
        // For demo, return mock response
        return Ok(AICommandResponse::success(AIResponse {
            text: format!("✨ [Mock AI Processing] Improved version: {}", text),
            processing_time_ms: 150,
            confidence: 0.9,
            token_usage: crate::ai::TokenUsage {
                prompt_tokens: text.len() as u32 / 4, // Rough estimate
                completion_tokens: 50,
                total_tokens: (text.len() as u32 / 4) + 50,
            },
            was_streamed: false,
        }));
    }
    
    // Create Gemini client
    let mut client = match GeminiClient::new(config) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create Gemini client: {:?}", e);
            return Ok(AICommandResponse::error(format!("Failed to initialize Gemini: {}", e)));
        }
    };
    
    // Determine instruction type
    let ai_instruction = match instruction.as_deref() {
        Some("improve") => AIInstruction::Improve,
        Some("summarize") => AIInstruction::Summarize,
        Some(custom) => AIInstruction::Custom { instruction: custom.to_string() },
        None => AIInstruction::Auto,
    };
    
    // Create request
    let request = AIRequest {
        text,
        instruction: ai_instruction,
        context: None,
    };
    
    // Process with Gemini
    match client.process_text(request).await {
        Ok(response) => {
            info!("✅ Gemini processing completed: {} chars output", response.text.len());
            Ok(AICommandResponse::success(response))
        },
        Err(e) => {
            error!("Gemini processing failed: {:?}", e);
            Ok(AICommandResponse::error(format!("AI processing failed: {}", e)))
        }
    }
}

/// Transcribe audio using local Whisper (mock implementation)
#[tauri::command]
pub async fn transcribe_audio(
    audio_data: Vec<f32>,
) -> Result<AICommandResponse<TranscriptionResult>, String> {
    let start_time = std::time::Instant::now();
    
    info!("🎤 Transcribing {} audio samples", audio_data.len());
    
    if audio_data.is_empty() {
        return Ok(AICommandResponse::error("No audio data provided".to_string()));
    }
    
    // Create temporary engine for transcription (stateless)
    let config = STTConfig::default();
    let mut engine = match WhisperEngine::new(config) {
        Ok(engine) => engine,
        Err(e) => {
            error!("Failed to create Whisper engine: {:?}", e);
            return Ok(AICommandResponse::error(format!("Failed to initialize Whisper: {}", e)));
        }
    };
    
    // Load model (mock)
    if let Err(e) = engine.load_model().await {
        error!("Failed to load Whisper model: {:?}", e);
        return Ok(AICommandResponse::error(format!("Failed to load Whisper model: {}", e)));
    }
    
    // Perform transcription
    match engine.transcribe(&audio_data).await {
        Ok(transcription) => {
            let total_time = start_time.elapsed();
            info!("✅ Transcription completed in {}ms: '{}'", 
                  total_time.as_millis(), 
                  transcription.text.chars().take(50).collect::<String>());
            
            Ok(AICommandResponse::success(transcription))
        },
        Err(e) => {
            error!("Transcription failed: {:?}", e);
            Ok(AICommandResponse::error(format!("Transcription failed: {}", e)))
        }
    }
}

/// Initialize Whisper engine with specific configuration (stateless)
#[tauri::command]
pub async fn initialize_whisper(
    config: STTConfig,
) -> Result<AICommandResponse<bool>, String> {
    info!("🔧 Initializing Whisper with model: {}", config.model_size.as_str());
    
    match WhisperEngine::new(config) {
        Ok(mut engine) => {
            // Load model 
            match engine.load_model().await {
                Ok(()) => {
                    info!("✅ Whisper engine initialized successfully");
                    Ok(AICommandResponse::success(true))
                },
                Err(e) => {
                    error!("Failed to load Whisper model: {:?}", e);
                    Ok(AICommandResponse::error(format!("Model load failed: {}", e)))
                }
            }
        },
        Err(e) => {
            error!("Failed to create Whisper engine: {:?}", e);
            Ok(AICommandResponse::error(format!("Engine creation failed: {}", e)))
        }
    }
}

/// Get Whisper engine statistics (mock)
#[tauri::command]
pub fn get_whisper_stats() -> Result<AICommandResponse<crate::stt::STTStats>, String> {
    // Return default stats since we're using stateless approach
    let stats = crate::stt::STTStats {
        total_transcriptions: 0,
        avg_processing_time_ms: 0.0,
        min_processing_time_ms: u64::MAX,
        max_processing_time_ms: 0,
        current_model: crate::stt::WhisperModelSize::Base,
        model_load_time_ms: 100,
        memory_usage_mb: 74.0,
    };
    
    Ok(AICommandResponse::success(stats))
}
