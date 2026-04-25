use std::fs;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Serialize)]
struct FileDoc {
    path: String,
    content: String,
}

#[tauri::command]
fn open_file(path: String) -> Result<FileDoc, String> {
    let buf = PathBuf::from(&path);
    let content = fs::read_to_string(&buf)
        .map_err(|e| format!("failed to read {}: {}", buf.display(), e))?;
    Ok(FileDoc { path, content })
}

#[tauri::command]
fn save_file(path: String, content: String) -> Result<(), String> {
    let buf = PathBuf::from(&path);
    fs::write(&buf, content).map_err(|e| format!("failed to write {}: {}", buf.display(), e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![open_file, save_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
