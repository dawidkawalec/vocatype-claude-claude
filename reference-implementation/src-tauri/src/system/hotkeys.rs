use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tracing::{debug, info, warn};
use global_hotkey::{
    GlobalHotKeyManager, 
    HotKeyState, 
    hotkey::{HotKey, Modifiers, Code}
};

use crate::utils::error::{SystemError, SystemResult};

/// Global hotkey manager for VocaType
/// Handles Cmd+Shift+V (macOS) and Ctrl+Shift+V (Windows) combinations
pub struct HotkeyManager {
    /// Global hotkey manager instance
    manager: GlobalHotKeyManager,
    
    /// Currently registered hotkeys
    registered_hotkeys: Vec<HotKey>,
    
    /// Event callback for hotkey activation
    hotkey_callback: Option<Arc<dyn Fn() + Send + Sync>>,
    
    /// Whether hotkeys are currently active
    is_active: Arc<AtomicBool>,
}

impl HotkeyManager {
    /// Create new hotkey manager
    pub fn new() -> SystemResult<Self> {
        info!("⌨️  Initializing global hotkey manager");
        
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| SystemError::HotkeyRegistrationFailed { 
                key_combination: format!("Manager initialization failed: {}", e) 
            })?;
        
        Ok(Self {
            manager,
            registered_hotkeys: Vec::new(),
            hotkey_callback: None,
            is_active: Arc::new(AtomicBool::new(false)),
        })
    }
    
    /// Register the main VocaType hotkey (Cmd+Shift+V on macOS, Ctrl+Shift+V on Windows)
    pub fn register_main_hotkey(&mut self) -> SystemResult<()> {
        info!("🔗 Registering main VocaType hotkey");
        
        // Define key combination based on platform
        let hotkey = if cfg!(target_os = "macos") {
            // Cmd+Shift+V for macOS
            HotKey::new(
                Some(Modifiers::META | Modifiers::SHIFT),
                Code::KeyV
            )
        } else {
            // Ctrl+Shift+V for Windows/Linux
            HotKey::new(
                Some(Modifiers::CONTROL | Modifiers::SHIFT),
                Code::KeyV
            )
        };
        
        // Register the hotkey
        self.manager.register(hotkey)
            .map_err(|e| SystemError::HotkeyRegistrationFailed { 
                key_combination: format!("Failed to register Cmd+Shift+V: {}", e) 
            })?;
        
        self.registered_hotkeys.push(hotkey);
        self.is_active.store(true, Ordering::Release);
        
        let key_desc = if cfg!(target_os = "macos") { "⌘+Shift+V" } else { "Ctrl+Shift+V" };
        info!("✅ Hotkey registered: {}", key_desc);
        
        Ok(())
    }
    
    /// Set callback function for hotkey activation
    pub fn set_hotkey_callback<F>(&mut self, callback: F) 
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.hotkey_callback = Some(Arc::new(callback));
        debug!("🔧 Hotkey callback configured");
    }
    
    /// Start listening for hotkey events
    pub async fn start_listening(&self) -> SystemResult<()> {
        info!("👂 Starting hotkey event listening");
        
        if !self.is_active.load(Ordering::Acquire) {
            return Err(SystemError::HotkeyRegistrationFailed { 
                key_combination: "No hotkeys registered".to_string() 
            });
        }
        
        // Start event loop in background
        let callback = self.hotkey_callback.clone();
        let is_active = Arc::clone(&self.is_active);
        
        tokio::spawn(async move {
            use global_hotkey::GlobalHotKeyEvent;
            
            loop {
                if !is_active.load(Ordering::Acquire) {
                    break;
                }
                
                // Check for hotkey events
                if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                    match event.state {
                        HotKeyState::Pressed => {
                            debug!("🔥 Hotkey activated!");
                            
                            if let Some(ref callback_fn) = callback {
                                callback_fn();
                            }
                        },
                        HotKeyState::Released => {
                            // Optional: Handle key release
                        },
                    }
                }
                
                // Small delay to prevent excessive CPU usage
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            
            info!("👋 Hotkey listening stopped");
        });
        
        Ok(())
    }
    
    /// Stop listening for hotkey events
    pub fn stop_listening(&self) {
        info!("🛑 Stopping hotkey listening");
        self.is_active.store(false, Ordering::Release);
    }
    
    /// Unregister all hotkeys
    pub fn unregister_all(&mut self) -> SystemResult<()> {
        info!("🗑️  Unregistering all hotkeys");
        
        for hotkey in &self.registered_hotkeys {
            if let Err(e) = self.manager.unregister(*hotkey) {
                warn!("⚠️  Failed to unregister hotkey: {}", e);
            }
        }
        
        self.registered_hotkeys.clear();
        self.is_active.store(false, Ordering::Release);
        
        info!("✅ All hotkeys unregistered");
        Ok(())
    }
    
    /// Check if hotkeys are currently active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Acquire)
    }
    
    /// Get list of registered hotkey combinations
    pub fn get_registered_combinations(&self) -> Vec<String> {
        self.registered_hotkeys.iter()
            .map(|hotkey| self.hotkey_to_string(hotkey))
            .collect()
    }
    
    /// Convert hotkey to human-readable string
    fn hotkey_to_string(&self, hotkey: &HotKey) -> String {
        let mut parts = Vec::new();
        
        let modifiers = hotkey.mods;
        if !modifiers.is_empty() {
            if modifiers.contains(Modifiers::META) {
                parts.push(if cfg!(target_os = "macos") { "⌘" } else { "Meta" });
            }
            if modifiers.contains(Modifiers::CONTROL) {
                parts.push("Ctrl");
            }
            if modifiers.contains(Modifiers::ALT) {
                parts.push("Alt");
            }
            if modifiers.contains(Modifiers::SHIFT) {
                parts.push("Shift");
            }
        }
        
        // Add the main key
        let key_name = match hotkey.key {
            Code::KeyV => "V",
            Code::KeyA => "A", 
            Code::KeyS => "S",
            _ => "?",
        };
        parts.push(key_name);
        
        parts.join("+")
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        debug!("🗑️  Dropping HotkeyManager");
        let _ = self.unregister_all();
    }
}
