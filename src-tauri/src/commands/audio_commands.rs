use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::audio::{AudioConfig, AudioStats, AudioDevice, CaptureState};

/// Response for audio commands with success/error handling
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioCommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> AudioCommandResponse<T> {
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

/// Start audio capture with specified configuration
#[tauri::command]
pub async fn start_audio_capture(
    device_name: Option<String>,
    _config: Option<AudioConfig>, // Prefixed with underscore since not yet implemented
) -> Result<AudioCommandResponse<bool>, String> {
    info!("🎬 Starting audio capture command with device: {:?}", device_name);
    
    // TODO: Implement audio capture startup
    // For now, return success placeholder
    Ok(AudioCommandResponse::success(true))
}

/// Stop audio capture
#[tauri::command]
pub async fn stop_audio_capture() -> Result<AudioCommandResponse<bool>, String> {
    info!("🛑 Stopping audio capture command");
    
    // TODO: Implement audio capture stop
    Ok(AudioCommandResponse::success(true))
}

/// Get current audio level for UI visualization
#[tauri::command]
pub fn get_audio_level() -> Result<AudioCommandResponse<f32>, String> {
    // TODO: Implement real audio level reading
    Ok(AudioCommandResponse::success(0.0))
}

/// Get comprehensive audio statistics
#[tauri::command]
pub fn get_audio_stats() -> Result<AudioCommandResponse<AudioStats>, String> {
    // TODO: Implement real audio statistics
    Ok(AudioCommandResponse::success(AudioStats::default()))
}

/// Get available audio input devices
#[tauri::command]
pub async fn get_audio_devices() -> Result<AudioCommandResponse<Vec<AudioDevice>>, String> {
    info!("🔍 Getting available audio devices");
    
    // Use the real cpal implementation for device enumeration
    match enumerate_audio_devices().await {
        Ok(devices) => {
            info!("Found {} audio devices", devices.len());
            Ok(AudioCommandResponse::success(devices))
        },
        Err(e) => {
            error!("Failed to get audio devices: {}", e);
            Ok(AudioCommandResponse::error(format!("Failed to get devices: {}", e)))
        }
    }
}

/// Internal function to enumerate audio devices using cpal
async fn enumerate_audio_devices() -> Result<Vec<AudioDevice>, String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    
    let host = cpal::default_host();
    let devices = host.input_devices()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?;
    
    let default_device = host.default_input_device();
    let default_name = default_device
        .as_ref()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();
    
    let mut audio_devices = Vec::new();
    
    for device in devices {
        let name = device.name()
            .map_err(|e| format!("Could not get device name: {}", e))?;
        
        // Get device capabilities  
        let supported_configs = device.supported_input_configs()
            .map_err(|e| format!("Could not get device configs: {}", e))?;
        
        // Find best matching config for our requirements (16kHz, mono)
        let mut best_sample_rate = 44100; // Default fallback
        let mut best_channels = 1;
        
        for config in supported_configs {
            // Prefer configs that support 16kHz
            let min_rate = config.min_sample_rate().0;
            let max_rate = config.max_sample_rate().0;
            
            if min_rate <= 16000 && max_rate >= 16000 {
                best_sample_rate = 16000;
                best_channels = config.channels().min(2); // Prefer mono/stereo
                break;
            } else {
                // Use closest available rate
                if (min_rate as i32 - 16000).abs() < (best_sample_rate as i32 - 16000).abs() {
                    best_sample_rate = min_rate;
                    best_channels = config.channels().min(2);
                }
            }
        }
        
        audio_devices.push(AudioDevice {
            name: name.clone(),
            is_default: name == default_name,
            channels: best_channels,
            sample_rate: best_sample_rate,
        });
    }
    
    // Ensure we have at least one device (add default if none found)
    if audio_devices.is_empty() {
        audio_devices.push(AudioDevice {
            name: "Default Input".to_string(),
            is_default: true,
            channels: 1,
            sample_rate: 16000,
        });
    }
    
    Ok(audio_devices)
}

/// Get recent audio data for STT processing
#[tauri::command]
pub fn get_recent_audio(duration_ms: u64) -> Result<AudioCommandResponse<Vec<f32>>, String> {
    info!("📊 Getting recent audio: {}ms", duration_ms);
    
    // TODO: Implement real audio data retrieval
    Ok(AudioCommandResponse::success(vec![0.0; (duration_ms * 16) as usize]))
}

/// Check if audio capture is currently recording
#[tauri::command]
pub fn is_recording() -> Result<AudioCommandResponse<bool>, String> {
    // TODO: Implement real recording state check
    Ok(AudioCommandResponse::success(false))
}

/// Get current capture state
#[tauri::command]
pub fn get_capture_state() -> Result<AudioCommandResponse<CaptureState>, String> {
    // TODO: Implement real state retrieval
    Ok(AudioCommandResponse::success(CaptureState::Stopped))
}

/// Configure audio capture settings
#[tauri::command]
pub fn configure_audio(config: AudioConfig) -> Result<AudioCommandResponse<bool>, String> {
    info!("⚙️  Configuring audio with: {:?}", config);
    
    // TODO: Implement real audio configuration
    Ok(AudioCommandResponse::success(true))
}
