// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Audio commands
            vocatype_temp_lib::commands::audio_commands::start_audio_capture,
            vocatype_temp_lib::commands::audio_commands::stop_audio_capture,
            vocatype_temp_lib::commands::audio_commands::get_audio_level,
            vocatype_temp_lib::commands::audio_commands::get_audio_stats,
            vocatype_temp_lib::commands::audio_commands::get_audio_devices,
            vocatype_temp_lib::commands::audio_commands::get_recent_audio,
            vocatype_temp_lib::commands::audio_commands::is_recording,
            vocatype_temp_lib::commands::audio_commands::get_capture_state,
            vocatype_temp_lib::commands::audio_commands::configure_audio,
            
            // System commands
            vocatype_temp_lib::commands::system_commands::register_hotkeys,
            vocatype_temp_lib::commands::system_commands::get_selected_text,
            vocatype_temp_lib::commands::system_commands::copy_to_clipboard,
            
            // AI processing commands
            vocatype_temp_lib::commands::ai_commands::process_with_gemini,
            vocatype_temp_lib::commands::ai_commands::transcribe_audio,
            vocatype_temp_lib::commands::ai_commands::initialize_whisper,
            vocatype_temp_lib::commands::ai_commands::get_whisper_stats,
        ])
        .setup(|_app| {
            // Initialize logging
            tracing_subscriber::fmt::init();
            tracing::info!("🚀 VocaType starting up...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
