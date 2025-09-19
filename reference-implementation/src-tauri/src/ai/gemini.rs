use std::time::{Duration, Instant};
use tracing::{debug, info, warn, error};
use reqwest::Client;
use serde_json::{json, Value};
// use tokio_stream::StreamExt; // For future streaming implementation

use crate::utils::error::{AIError, AIResult};
use super::types::{AIConfig, AIRequest, AIResponse, AIStats, TokenUsage};

/// Gemini 2.5 Flash client with streaming support
pub struct GeminiClient {
    /// HTTP client with connection pooling
    client: Client,
    
    /// Configuration
    config: AIConfig,
    
    /// Base API URL
    base_url: String,
    
    /// Processing statistics
    stats: AIStats,
}

impl GeminiClient {
    /// Create new Gemini client
    pub fn new(config: AIConfig) -> AIResult<Self> {
        let start_time = Instant::now();
        
        if config.api_key.is_empty() {
            return Err(AIError::InvalidAPIKey);
        }
        
        info!("🤖 Initializing Gemini client with model: {}", config.model);
        
        // Create HTTP client with optimized settings for streaming
        let client = Client::builder()
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("VocaType/0.1.0")
            .build()
            .map_err(|e| AIError::ResponseParsingFailed(format!("HTTP client creation failed: {}", e)))?;
        
        let base_url = "https://generativelanguage.googleapis.com/v1beta/models".to_string();
        
        let init_time = start_time.elapsed();
        debug!("⚡ Gemini client initialized in {:?}", init_time);
        
        Ok(Self {
            client,
            config: config.clone(),
            base_url,
            stats: AIStats {
                total_requests: 0,
                avg_processing_time_ms: 0.0,
                min_processing_time_ms: u64::MAX,
                max_processing_time_ms: 0,
                total_tokens_used: 0,
                current_model: config.model.clone(),
                rate_limit_remaining: None,
                last_request_time: None,
            },
        })
    }
    
    /// Process text with Gemini (non-streaming)
    pub async fn process_text(&mut self, request: AIRequest) -> AIResult<AIResponse> {
        let start_time = Instant::now();
        
        info!("🧠 Processing text with Gemini: {} chars", request.text.len());
        
        let prompt = request.instruction.to_prompt(&request.text);
        
        // Prepare request payload
        let payload = json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": self.config.max_tokens,
                "candidateCount": 1,
            }
        });
        
        // Make API request
        let url = format!("{}/{}:generateContent?key={}", 
                         self.base_url, self.config.model, self.config.api_key);
        
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                error!("API request failed: {:?}", e);
                AIError::APIRequestFailed { status_code: 0 }
            })?;
        
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            error!("API returned error status: {}", status);
            return Err(AIError::APIRequestFailed { status_code: status });
        }
        
        // Parse response
        let response_json: Value = response.json().await
            .map_err(|e| AIError::ResponseParsingFailed(format!("JSON parsing failed: {}", e)))?;
        
        let processed_text = self.extract_text_from_response(&response_json)?;
        let token_usage = self.extract_token_usage(&response_json);
        
        let processing_time = start_time.elapsed();
        let processing_time_ms = processing_time.as_millis() as u64;
        
        // Update statistics
        self.update_stats(processing_time_ms, &token_usage);
        
        let response = AIResponse {
            text: processed_text,
            processing_time_ms,
            confidence: 0.9, // Gemini typically has high confidence
            token_usage,
            was_streamed: false,
        };
        
        // Performance check
        if processing_time_ms > 300 {
            warn!("⚠️  AI processing took {}ms (target: <300ms)", processing_time_ms);
        } else {
            debug!("⚡ AI processing completed in {}ms", processing_time_ms);
        }
        
        Ok(response)
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> &AIStats {
        &self.stats
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: AIConfig) -> AIResult<()> {
        info!("⚙️  Updating AI configuration");
        self.config = config;
        Ok(())
    }
    
    /// Extract text content from Gemini response
    fn extract_text_from_response(&self, response: &Value) -> AIResult<String> {
        let candidates = response["candidates"]
            .as_array()
            .ok_or_else(|| AIError::ResponseParsingFailed("No candidates in response".to_string()))?;
        
        if candidates.is_empty() {
            return Err(AIError::ResponseParsingFailed("Empty candidates array".to_string()));
        }
        
        let first_candidate = &candidates[0];
        let content = &first_candidate["content"];
        let parts = content["parts"]
            .as_array()
            .ok_or_else(|| AIError::ResponseParsingFailed("No parts in content".to_string()))?;
        
        if parts.is_empty() {
            return Err(AIError::ResponseParsingFailed("Empty parts array".to_string()));
        }
        
        let text = parts[0]["text"]
            .as_str()
            .ok_or_else(|| AIError::ResponseParsingFailed("No text in first part".to_string()))?;
        
        Ok(text.to_string())
    }
    
    /// Extract token usage from response (if available)
    fn extract_token_usage(&self, response: &Value) -> TokenUsage {
        let usage_metadata = &response["usageMetadata"];
        
        let prompt_tokens = usage_metadata["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;
        
        let completion_tokens = usage_metadata["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;
        
        TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
    
    /// Update processing statistics
    fn update_stats(&mut self, processing_time_ms: u64, token_usage: &TokenUsage) {
        self.stats.total_requests += 1;
        
        // Update min/max times
        self.stats.min_processing_time_ms = self.stats.min_processing_time_ms.min(processing_time_ms);
        self.stats.max_processing_time_ms = self.stats.max_processing_time_ms.max(processing_time_ms);
        
        // Update moving average
        let total = self.stats.total_requests as f64;
        self.stats.avg_processing_time_ms = 
            (self.stats.avg_processing_time_ms * (total - 1.0) + processing_time_ms as f64) / total;
        
        // Update token usage
        self.stats.total_tokens_used += token_usage.total_tokens as u64;
        
        // Update timestamp
        self.stats.last_request_time = Some(
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
        );
    }
}