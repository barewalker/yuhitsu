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

// プロジェクトビュー用のディレクトリエントリ。再帰的に子を持つ可能性がある。
// 全ファイル表示。隠しファイル(`.` 始まり)と定番ノイズフォルダだけ除外する。
#[derive(Serialize)]
struct DirEntry {
    name: String,
    path: String,
    is_dir: bool,
    children: Option<Vec<DirEntry>>,
}

// ビルド成果物・パッケージマネージャ管理ディレクトリ・IDE 個別キャッシュなど、
// 文書プロジェクト用途では雑音にしかならないフォルダ。`.` 始まりは別途除外。
const NOISE_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "__pycache__",
    ".svelte-kit",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
    ".idea",
    ".vscode",
];
const PROJECT_VIEW_MAX_DEPTH: usize = 8;

fn read_dir_recursive(path: &PathBuf, depth: usize) -> Result<Vec<DirEntry>, String> {
    let entries = fs::read_dir(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut dirs: Vec<DirEntry> = Vec::new();
    let mut files: Vec<DirEntry> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // 隠しファイル / フォルダ
        if name.starts_with('.') {
            continue;
        }
        let entry_path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            if NOISE_DIRS.contains(&name.as_str()) {
                continue;
            }
            // 深さ制限を超えたら子を読まない(空ディレクトリ表示)
            let children = if depth + 1 < PROJECT_VIEW_MAX_DEPTH {
                read_dir_recursive(&entry_path, depth + 1).ok()
            } else {
                Some(Vec::new())
            };
            dirs.push(DirEntry {
                name,
                path: entry_path.to_string_lossy().into_owned(),
                is_dir: true,
                children,
            });
        } else if metadata.is_file() {
            files.push(DirEntry {
                name,
                path: entry_path.to_string_lossy().into_owned(),
                is_dir: false,
                children: None,
            });
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let mut combined: Vec<DirEntry> = dirs;
    combined.extend(files);
    Ok(combined)
}

#[tauri::command]
fn list_directory(path: String) -> Result<DirEntry, String> {
    let buf = PathBuf::from(&path);
    if !buf.is_dir() {
        return Err(format!("not a directory: {}", buf.display()));
    }
    let name = buf
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let children = read_dir_recursive(&buf, 0)?;
    Ok(DirEntry {
        name,
        path,
        is_dir: true,
        children: Some(children),
    })
}

// 開発時のフロント側ログを Rust の stderr に流すための bridge。
// `pnpm tauri dev` のログに `[js]` プレフィックス付きで出るので、
// Claude や開発者が WebView の DevTools を開かずに状況把握できる。
// 製品リリース前に削除するか、release ビルドでは no-op にする。
#[tauri::command]
fn dev_log(message: String) {
    eprintln!("[js] {message}");
}

// Typst の `--root` に渡すプロジェクトルート。
// Linux/macOS は `/`、Windows は入力パスのドライブ。tinymist は cwd を
// 起点に root を相対化することがあり、cwd と root が食い違うと "entry
// file must be in the root directory" で弾かれるため、subprocess 起動
// 時に cwd もここで返す値に合わせる(`spawn_root_dir` の方を参照)。
fn filesystem_root_for(path: &str) -> String {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        "/".to_string()
    }
    #[cfg(target_os = "windows")]
    {
        use std::path::Component;
        PathBuf::from(path)
            .components()
            .next()
            .and_then(|c| match c {
                Component::Prefix(p) => {
                    Some(format!("{}\\", p.as_os_str().to_string_lossy()))
                }
                _ => None,
            })
            .unwrap_or_else(|| "C:\\".to_string())
    }
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
    // Typst は絶対パス参照(`#image("/abs/...")`)を `--root` 起点で解決するため、
    // 未指定だと入力ファイルの親ディレクトリが root になり、ホーム配下の画像など
    // 文書外の絶対パスが読めない。Yuhitsu はローカル GUI なので、ファイルシステム
    // ルートを root として渡す(セキュリティモデルは緩むが、ユーザ自身のファイルを
    // 自分のエディタで読むだけなので許容)。Windows は入力パスのドライブを起点に。
    let root = filesystem_root_for(&path);
    let child = Command::new("tinymist")
        // tinymist が `--root` を内部で相対化する際の起点を揃える(cwd と
        // root を一致させないと entry が root 外と判定されることがある)。
        .current_dir(&root)
        .args([
            "preview",
            "--no-open",
            "--data-plane-host",
            "127.0.0.1:23625",
            "--control-plane-host",
            "127.0.0.1:23626",
            "--root",
            &root,
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
    // preview と同じく filesystem ルートを `--root` に渡す。文書外の絶対パスを
    // 読みたい(例: ホーム配下のスクショ画像)用途に合わせる。
    let root = filesystem_root_for(&input);
    let result = Command::new("tinymist")
        // start_preview と同じく cwd を root に揃える
        .current_dir(&root)
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
            list_directory,
            start_preview,
            stop_preview,
            export_pdf,
            lsp_start,
            lsp_send,
            lsp_stop,
            dev_log
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
