use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State};

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

#[derive(Default)]
struct PreviewState {
    // 現在動いている tinymist preview の subprocess を抱える
    child: Mutex<Option<Child>>,
}

impl PreviewState {
    fn kill_existing(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[tauri::command]
fn start_preview(path: String, state: State<'_, PreviewState>) -> Result<(), String> {
    state.kill_existing();
    let child = Command::new("tinymist")
        .args([
            "preview",
            "--no-open",
            "--data-plane-host",
            "127.0.0.1:23625",
            "--control-plane-host",
            "127.0.0.1:23626",
            &path,
        ])
        // dev 中は stderr をそのまま流したいので inherit、stdout は閉じておく
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn tinymist: {}", e))?;
    let mut guard = state
        .child
        .lock()
        .map_err(|e| format!("failed to lock preview state: {}", e))?;
    *guard = Some(child);
    Ok(())
}

#[tauri::command]
fn stop_preview(state: State<'_, PreviewState>) -> Result<(), String> {
    state.kill_existing();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(PreviewState::default())
        .invoke_handler(tauri::generate_handler![
            open_file,
            save_file,
            start_preview,
            stop_preview
        ])
        .on_window_event(|window, event| {
            // ウィンドウが閉じられた時(終了確認後の destroy 含む)に preview を必ず止める
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<PreviewState>() {
                    state.kill_existing();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
