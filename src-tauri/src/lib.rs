// Tauri commands for MyMusic DAW
// These commands are callable from the React frontend

/// Get DAW engine information
#[tauri::command]
fn get_engine_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": "MyMusic DAW",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "audio_engine": "CPAL",
        "sample_rate": 44100,
        "buffer_size": 512
    }))
}

/// Play a test beep (simple sine wave)
#[tauri::command]
fn play_test_beep() -> Result<String, String> {
    // For now, just return a success message
    // Later we'll connect this to the actual audio engine
    Ok("Test beep played!".to_string())
}

/// Get list of available waveforms
#[tauri::command]
fn get_waveforms() -> Result<Vec<String>, String> {
    Ok(vec![
        "Sine".to_string(),
        "Square".to_string(),
        "Saw".to_string(),
        "Triangle".to_string(),
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        get_engine_info,
        play_test_beep,
        get_waveforms,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
