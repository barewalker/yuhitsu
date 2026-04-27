use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command as TokioCommand};
use tokio::sync::Mutex as AsyncMutex;

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

#[tauri::command]
fn export_pdf(input: String, output: String) -> Result<(), String> {
    let input_path = PathBuf::from(&input);
    // Typst の絶対パス参照(/foo.png のような書き方)はプロジェクトルートからの解決
    // となるため、root は入力ファイルの親ディレクトリを既定値にしておく。
    let root = input_path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());
    let result = Command::new("tinymist")
        .args(["compile", "--root", &root, &input, &output])
        .output()
        .map_err(|e| format!("failed to spawn tinymist compile: {}", e))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("compile failed:\n{}", stderr.trim()));
    }
    Ok(())
}

// ============================================================
// LSP subprocess(tinymist lsp)管理
// ------------------------------------------------------------
// CodeMirror の @codemirror/lsp-client は JSON 文字列のみを扱う Transport
// 抽象を求めるため、LSP の Content-Length ヘッダの組み立て / 剥離は
// このバックエンド側で完結させる。
// ============================================================

#[derive(Default)]
struct LspState {
    // tinymist lsp の subprocess。child は ID とハンドル両方を抱える。
    child: AsyncMutex<Option<tokio::process::Child>>,
    stdin: AsyncMutex<Option<ChildStdin>>,
}

impl LspState {
    async fn kill_existing(&self) {
        // stdin は先に閉じる(Drop で自然に flush される)。
        {
            let mut guard = self.stdin.lock().await;
            *guard = None;
        }
        let mut guard = self.child.lock().await;
        if let Some(mut child) = guard.take() {
            let _ = child.kill().await;
        }
    }
}

#[tauri::command]
async fn lsp_start(app: AppHandle, state: State<'_, LspState>) -> Result<(), String> {
    state.kill_existing().await;

    let mut child = TokioCommand::new("tinymist")
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn tinymist lsp: {}", e))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to capture tinymist stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture tinymist stdout".to_string())?;

    {
        let mut guard = state.stdin.lock().await;
        *guard = Some(stdin);
    }
    {
        let mut guard = state.child.lock().await;
        *guard = Some(child);
    }

    // stdout 側を非同期に読みながら、LSP の Content-Length ヘッダを解釈し、
    // JSON 本体だけを取り出して "lsp:message" イベントとして emit するタスク。
    let app_emit = app.clone();
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut header_line = String::new();
        loop {
            let mut content_length: Option<usize> = None;
            // ヘッダ群を読む(空行で終わる)
            loop {
                header_line.clear();
                let n = match reader.read_line(&mut header_line).await {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("[lsp] header read error: {e}");
                        return;
                    }
                };
                if n == 0 {
                    // EOF: tinymist が終了した
                    return;
                }
                let trimmed = header_line.trim_end_matches(|c| c == '\r' || c == '\n');
                if trimmed.is_empty() {
                    break;
                }
                if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                    if let Ok(v) = rest.trim().parse::<usize>() {
                        content_length = Some(v);
                    }
                }
                // 他のヘッダ(Content-Type など)は読み捨て
            }
            let len = match content_length {
                Some(l) => l,
                None => {
                    eprintln!("[lsp] message without Content-Length, dropping");
                    continue;
                }
            };
            let mut body = vec![0u8; len];
            if let Err(e) = reader.read_exact(&mut body).await {
                eprintln!("[lsp] body read error: {e}");
                return;
            }
            let json = match String::from_utf8(body) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[lsp] non-utf8 body: {e}");
                    continue;
                }
            };
            if let Err(e) = app_emit.emit("lsp:message", json) {
                eprintln!("[lsp] emit error: {e}");
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn lsp_send(state: State<'_, LspState>, message: String) -> Result<(), String> {
    let mut guard = state.stdin.lock().await;
    let stdin = guard
        .as_mut()
        .ok_or_else(|| "lsp not started".to_string())?;
    let header = format!("Content-Length: {}\r\n\r\n", message.len());
    stdin
        .write_all(header.as_bytes())
        .await
        .map_err(|e| format!("write header failed: {}", e))?;
    stdin
        .write_all(message.as_bytes())
        .await
        .map_err(|e| format!("write body failed: {}", e))?;
    stdin
        .flush()
        .await
        .map_err(|e| format!("flush failed: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn lsp_stop(state: State<'_, LspState>) -> Result<(), String> {
    state.kill_existing().await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(PreviewState::default())
        .manage(LspState::default())
        .invoke_handler(tauri::generate_handler![
            open_file,
            save_file,
            start_preview,
            stop_preview,
            export_pdf,
            lsp_start,
            lsp_send,
            lsp_stop
        ])
        .on_window_event(|window, event| {
            // ウィンドウが閉じられた時(終了確認後の destroy 含む)に
            // 抱えている subprocess を確実に止める。
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<PreviewState>() {
                    state.kill_existing();
                }
                if let Some(state) = window.app_handle().try_state::<LspState>() {
                    // async kill だが Destroyed では block_on で待つ
                    let state = state.inner();
                    tauri::async_runtime::block_on(state.kill_existing());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
