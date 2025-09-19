use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::system::{HotkeyManager, ClipboardManager};

/// Response for system commands
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemCommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> SystemCommandResponse<T> {
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

/// Register global hotkeys (Cmd+Shift+V for macOS)
#[tauri::command]
pub async fn register_hotkeys() -> Result<SystemCommandResponse<bool>, String> {
    info!("⌨️  Registering global hotkeys");
    
    match HotkeyManager::new() {
        Ok(mut manager) => {
            // Set callback for hotkey activation
            manager.set_hotkey_callback(|| {
                info!("🔥 VocaType hotkey activated!");
                println!("🎤 VocaType activated via hotkey - starting audio capture...");
            });
            
            // Register main VocaType hotkey
            match manager.register_main_hotkey() {
                Ok(()) => {
                    // Start listening for events
                    match manager.start_listening().await {
                        Ok(()) => {
                            info!("✅ Global hotkeys registered and listening");
                            Ok(SystemCommandResponse::success(true))
                        },
                        Err(e) => {
                            error!("Failed to start hotkey listening: {:?}", e);
                            Ok(SystemCommandResponse::error(format!("Failed to start listening: {}", e)))
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to register hotkeys: {:?}", e);
                    Ok(SystemCommandResponse::error(format!("Failed to register: {}", e)))
                }
            }
        },
        Err(e) => {
            error!("Failed to create hotkey manager: {:?}", e);
            Ok(SystemCommandResponse::error(format!("Failed to initialize: {}", e)))
        }
    }
}

/// Get currently selected text from system
#[tauri::command] 
pub async fn get_selected_text() -> Result<SystemCommandResponse<String>, String> {
    info!("📋 Getting selected text");
    
    match ClipboardManager::get_selected_text().await {
        Ok(text) => {
            info!("✅ Retrieved selected text: {} chars", text.len());
            Ok(SystemCommandResponse::success(text))
        },
        Err(e) => {
            error!("Failed to get selected text: {:?}", e);
            Ok(SystemCommandResponse::error(format!("Failed to get selection: {}", e)))
        }
    }
}

/// Copy text to clipboard
#[tauri::command]
pub async fn copy_to_clipboard(text: String) -> Result<SystemCommandResponse<bool>, String> {
    info!("📋 Copying text to clipboard: {} chars", text.len());
    
    match ClipboardManager::copy_text(&text).await {
        Ok(()) => {
            info!("✅ Text copied to clipboard successfully");
            Ok(SystemCommandResponse::success(true))
        },
        Err(e) => {
            error!("Failed to copy to clipboard: {:?}", e);
            Ok(SystemCommandResponse::error(format!("Copy failed: {}", e)))
        }
    }
}
