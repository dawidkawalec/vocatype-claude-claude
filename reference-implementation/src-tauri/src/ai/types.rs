use serde::{Deserialize, Serialize};

/// AI processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// Gemini API key
    pub api_key: String,
    
    /// Model to use (gemini-2.0-flash-exp, gemini-1.5-flash, etc.)
    pub model: String,
    
    /// Temperature for generation (0.0 - 2.0)
    pub temperature: f32,
    
    /// Maximum tokens to generate
    pub max_tokens: u32,
    
    /// Enable streaming responses
    pub stream: bool,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(), // Will be set from config
            model: "gemini-2.0-flash-exp".to_string(), // Latest Gemini 2.5 Flash
            temperature: 0.3, // Slightly creative but mostly deterministic
            max_tokens: 1000,
            stream: true,
            timeout_seconds: 30,
        }
    }
}

/// AI processing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequest {
    /// Text to process
    pub text: String,
    
    /// Processing type/instruction
    pub instruction: AIInstruction,
    
    /// Additional context
    pub context: Option<String>,
}

/// Types of AI processing instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIInstruction {
    /// Improve text (grammar, clarity, style)
    Improve,
    
    /// Summarize text
    Summarize,
    
    /// Translate to specified language
    Translate { target_language: String },
    
    /// Custom instruction
    Custom { instruction: String },
    
    /// Auto-detect best action based on content
    Auto,
}

impl AIInstruction {
    pub fn to_prompt(&self, text: &str) -> String {
        match self {
            AIInstruction::Improve => {
                format!("Please improve the following text by fixing grammar, enhancing clarity, and improving style while preserving the original meaning:\n\n{}", text)
            },
            AIInstruction::Summarize => {
                format!("Please provide a concise summary of the following text:\n\n{}", text)
            },
            AIInstruction::Translate { target_language } => {
                format!("Please translate the following text to {}:\n\n{}", target_language, text)
            },
            AIInstruction::Custom { instruction } => {
                format!("{}:\n\n{}", instruction, text)
            },
            AIInstruction::Auto => {
                format!("Please analyze the following text and perform the most appropriate action (improve, summarize, translate, or other helpful processing):\n\n{}", text)
            },
        }
    }
}

/// AI processing response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    /// Processed text
    pub text: String,
    
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    
    /// Confidence/quality score (0.0 - 1.0)
    pub confidence: f32,
    
    /// Token usage statistics
    pub token_usage: TokenUsage,
    
    /// Whether response was streamed
    pub was_streamed: bool,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens
    pub prompt_tokens: u32,
    
    /// Output tokens generated
    pub completion_tokens: u32,
    
    /// Total tokens used
    pub total_tokens: u32,
}

/// AI processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIStats {
    /// Total requests processed
    pub total_requests: u64,
    
    /// Average processing time (ms)
    pub avg_processing_time_ms: f64,
    
    /// Fastest processing time (ms)
    pub min_processing_time_ms: u64,
    
    /// Slowest processing time (ms)
    pub max_processing_time_ms: u64,
    
    /// Total tokens used
    pub total_tokens_used: u64,
    
    /// Current model being used
    pub current_model: String,
    
    /// API rate limit info
    pub rate_limit_remaining: Option<u32>,
    
    /// Last request time
    pub last_request_time: Option<String>,
}
