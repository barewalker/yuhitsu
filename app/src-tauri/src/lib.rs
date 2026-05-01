use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command as TokioCommand};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex as AsyncMutex;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as WsMessage;

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

// preview のホスト・ポート。data plane(HTML / SVG ストリーム)と
// control plane(エディタ ↔ preview の双方向制御)で別ポートが要る。
const PREVIEW_DATA_PLANE: &str = "127.0.0.1:23625";
const PREVIEW_CONTROL_PLANE: &str = "127.0.0.1:23626";
// data / control の両 TCP が listen 状態になるまで何 ms 待つか。
// 通常は数百 ms で起動するが、Windows / 低速マシンの余裕も見て 5 秒。
const PREVIEW_BOOT_TIMEOUT_MS: u64 = 5000;

#[derive(Default)]
struct PreviewState {
    // 現在動いている tinymist preview の subprocess を抱える
    child: Mutex<Option<Child>>,
    // control plane WebSocket への送信ハンドル。preview を起動して接続が
    // 張れたタイミングで Some になり、kill / 切断時に None に戻す。
    // 送信は内部 spawn したタスクが順次実行するので、ここはチャネル送信端のみ。
    control_tx: AsyncMutex<Option<UnboundedSender<String>>>,
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

    async fn clear_control(&self) {
        // tx を drop すると spawn 済みの送信タスクの recv が None を返して
        // ループを抜け、内部の WebSocket sink も自然に閉じる。
        let mut guard = self.control_tx.lock().await;
        *guard = None;
    }
}

// data plane の TCP listen を polling で待つ。control plane の方は
// listen していても accept ループの準備未完了で WS upgrade が
// `Connection reset by peer` で弾かれることがあるため、ここでは確認せず、
// 後段の `connect_control_plane_with_retry` で実 WS 接続まで含めて待つ。
async fn probe_data_plane_ready() -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(PREVIEW_BOOT_TIMEOUT_MS);
    loop {
        if tokio::net::TcpStream::connect(PREVIEW_DATA_PLANE).await.is_ok() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "preview data plane did not start in {} ms",
                PREVIEW_BOOT_TIMEOUT_MS
            ));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// control plane WebSocket(`ws://127.0.0.1:23626/`)に接続し、送信用の
// チャネルを PreviewState に格納する。
//
// 注意: tinymist は Origin ヘッダの spoofing 対策で `http://<host>:<port>` 形式
// しか受け付けないが、`localhost` / `127.0.0.1` だけは任意ポートを許容する
// 例外がある(crates/tinymist/src/tool/preview/http.rs::is_valid_origin)。
// それでも明示的に control plane 自身の origin を付けておく。
//
// 受信側は将来 `editorScrollTo` / `compileStatus` を扱うため、Tauri event
// `preview:control` にそのまま流す。今は drain でも構わないが、土台として
// 入れておく。
async fn connect_control_plane(
    app: AppHandle,
    state: &State<'_, PreviewState>,
) -> Result<(), String> {
    let url = format!("ws://{}/", PREVIEW_CONTROL_PLANE);
    // tinymist 側は listen はしていても accept ループの準備が整っておらず、
    // WS upgrade を送った瞬間に TCP を reset することがある(`Connection
    // reset by peer`)。一定時間内で retry して、初回起動時のレースを吸収。
    let deadline = tokio::time::Instant::now() + Duration::from_millis(PREVIEW_BOOT_TIMEOUT_MS);
    let ws_stream = loop {
        let mut req = url
            .clone()
            .into_client_request()
            .map_err(|e| format!("failed to build ws request: {}", e))?;
        req.headers_mut().insert(
            "Origin",
            format!("http://{}", PREVIEW_CONTROL_PLANE)
                .parse()
                .map_err(|e| format!("failed to build Origin header: {}", e))?,
        );
        match tokio_tungstenite::connect_async(req).await {
            Ok((ws, _resp)) => break ws,
            Err(e) => {
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "failed to connect preview control plane after {} ms: {}",
                        PREVIEW_BOOT_TIMEOUT_MS, e
                    ));
                }
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
        }
    };
    let (mut sink, mut stream) = ws_stream.split();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    {
        let mut guard = state.control_tx.lock().await;
        *guard = Some(tx);
    }

    // 送信タスク
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
        // recv が None(tx drop)、または送信失敗で抜ける。残りのバッファは
        // 捨てて構わない。
        let _ = sink.close().await;
    });

    // 受信タスク。土台として preview:control イベントに流すだけ。
    // 同期スクロールやクリックジャンプは将来 ここで dispatch する。
    let app_emit = app.clone();
    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(WsMessage::Text(t)) => {
                    if app_emit.emit("preview:control", t.to_string()).is_err() {
                        break;
                    }
                }
                Ok(WsMessage::Close(_)) => break,
                Err(e) => {
                    eprintln!("[preview] control plane recv error: {e}");
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn start_preview(
    app: AppHandle,
    path: String,
    state: State<'_, PreviewState>,
) -> Result<(), String> {
    state.kill_existing();
    state.clear_control().await;
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
            PREVIEW_DATA_PLANE,
            "--control-plane-host",
            PREVIEW_CONTROL_PLANE,
            "--root",
            &root,
            &path,
        ])
        // dev 中は stderr をそのまま流したいので inherit、stdout は閉じておく
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn tinymist: {}", e))?;
    {
        let mut guard = state
            .child
            .lock()
            .map_err(|e| format!("failed to lock preview state: {}", e))?;
        *guard = Some(child);
    }

    // data plane が listen するまで待つ。control plane の WS 接続自体は
    // connect_control_plane の中で retry ループ付きで張る。
    probe_data_plane_ready().await?;
    connect_control_plane(app, &state).await?;
    Ok(())
}

#[tauri::command]
async fn stop_preview(state: State<'_, PreviewState>) -> Result<(), String> {
    state.kill_existing();
    state.clear_control().await;
    Ok(())
}

// 編集中の未保存バッファを preview に注入するエントリポイント。
// フロントは onChange を debounce してこれを呼ぶ。
//
// `path` は memory file としてのキー。ディスク上のパスでもよいし、
// 無題タブ用の仮想パスでもよい(現状は path 持ちタブのみ呼び出し)。
//
// preview 未起動・未接続の場合は no-op で返す(エラーにしない:タブが
// .typ じゃない時に保険として呼ばれても落ちないように)。
#[tauri::command]
async fn preview_update_memory(
    state: State<'_, PreviewState>,
    path: String,
    content: String,
) -> Result<(), String> {
    let mut files = serde_json::Map::new();
    files.insert(path, serde_json::Value::String(content));
    let msg = serde_json::json!({
        "event": "updateMemoryFiles",
        "files": serde_json::Value::Object(files),
    })
    .to_string();
    let guard = state.control_tx.lock().await;
    let Some(tx) = guard.as_ref() else {
        return Ok(());
    };
    tx.send(msg)
        .map_err(|e| format!("preview control plane send failed: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn preview_remove_memory(
    state: State<'_, PreviewState>,
    paths: Vec<String>,
) -> Result<(), String> {
    let msg = serde_json::json!({
        "event": "removeMemoryFiles",
        "files": paths,
    })
    .to_string();
    let guard = state.control_tx.lock().await;
    let Some(tx) = guard.as_ref() else {
        return Ok(());
    };
    tx.send(msg)
        .map_err(|e| format!("preview control plane send failed: {}", e))?;
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

// tauri-plugin-store が保存している settings.json の絶対パスを返す。
// フロント側の「設定を開く」コマンドが、このパスをそのまま open_file に
// 渡してタブで編集する用途。
//
// 注意: tauri-plugin-store v2 は `app_data_dir()`(Linux で
// $XDG_DATA_HOME = ~/.local/share/<bundle-id>/)に保存する。
// `app_config_dir()`(~/.config/<bundle-id>/)とは別物で、間違えると
// 空ファイルを別の場所に作ってしまうので app_data_dir を使う。
//
// 初回起動などでファイルが未作成の時はディレクトリ + 空 JSON を作成して
// 「開けない」状態を回避する。
// ============================================================
// 無題タブ用の仮想 .typ パス管理
// ------------------------------------------------------------
// `tinymist preview` / LSP / memory file 注入は実在ファイルパスを要求するが、
// ユーザが「無題タブ」で書いている時はディスク上に対応するファイルが無い。
// preview / LSP を有効にするため、`<app_cache_dir>/untitled/<tab_id>.typ`
// に空ファイルを作っておき、そこを内部 path として扱う。
// 保存・タブ閉じ・アプリ終了時に掃除する。
// ============================================================

fn untitled_dir_for(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("failed to resolve cache_dir: {}", e))?
        .join("untitled");
    Ok(dir)
}

#[tauri::command]
fn prepare_untitled_path(app: AppHandle, tab_id: String) -> Result<String, String> {
    let dir = untitled_dir_for(&app)?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create untitled dir {}: {}", dir.display(), e))?;
    let path = dir.join(format!("{}.typ", tab_id));
    if !path.exists() {
        fs::write(&path, "")
            .map_err(|e| format!("failed to seed untitled file: {}", e))?;
    }
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn cleanup_untitled_path(path: String) -> Result<(), String> {
    let buf = PathBuf::from(&path);
    if buf.exists() {
        fs::remove_file(&buf)
            .map_err(|e| format!("failed to remove untitled file {}: {}", buf.display(), e))?;
    }
    Ok(())
}

// アプリ起動時に呼ぶ。前回 crash で残った無題タブの仮想ファイルを丸ごと
// 掃除する。中に入っている内容は全部「閉じたタブ」の残骸なので、保存
// された情報は無い前提。
fn cleanup_untitled_dir(app: &AppHandle) {
    let Ok(dir) = untitled_dir_for(app) else { return };
    if !dir.exists() {
        return;
    }
    let _ = fs::remove_dir_all(&dir);
}

// 起動時に、前回 Yuhitsu インスタンスが SIGKILL 等で異常終了した時に
// 孤児として残った tinymist preview / lsp の子プロセスを掃除する。
//
// 背景: Tauri dev は再ビルド時に Yuhitsu をシグナルで強制終了するため、
// `on_window_event(Destroyed)` の cleanup が走らないことがある。残った
// tinymist preview が 127.0.0.1:23625 を保持していると、新インスタンス
// 起動時に AddrInUse で panic する(`Result::unwrap()` on Err
// `AddrInUse` at preview/http.rs)。
//
// 誤爆の懸念: Yuhitsu 以外で `tinymist preview` を使っているケースは
// 想定しないが、もし他用途で立てているなら起動時に巻き添えで死ぬ。
// 通常の利用では問題にならない。
fn kill_lingering_tinymist() {
    #[cfg(unix)]
    {
        // pkill -f "tinymist preview" / "tinymist lsp" は子プロセス
        // (preview の subprocess of subprocess は無し)を一掃する。
        // 終了コードは握り潰し(該当プロセス無しで非ゼロが返るため)。
        let _ = Command::new("pkill")
            .args(["-f", "tinymist preview"])
            .status();
        let _ = Command::new("pkill")
            .args(["-f", "tinymist lsp"])
            .status();
    }
}

#[tauri::command]
fn get_settings_path(app: AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app_data_dir: {}", e))?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create data dir {}: {}", dir.display(), e))?;
    let path = dir.join("settings.json");
    if !path.exists() {
        fs::write(&path, "{}\n")
            .map_err(|e| format!("failed to seed settings.json: {}", e))?;
    }
    Ok(path.to_string_lossy().into_owned())
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
            preview_update_memory,
            preview_remove_memory,
            export_pdf,
            lsp_start,
            lsp_send,
            lsp_stop,
            dev_log,
            get_settings_path,
            prepare_untitled_path,
            cleanup_untitled_path
        ])
        .setup(|app| {
            // dev での強制終了などで孤児になった tinymist プロセスを
            // 先に始末する。残ったままだと新 preview の bind が
            // AddrInUse で panic する。
            kill_lingering_tinymist();
            // 前回 crash で残った無題タブ用仮想ファイルを掃除する。
            // タブ ID は起動毎に振り直されるため、過去の残骸は意味を持たない。
            cleanup_untitled_dir(&app.handle());
            Ok(())
        })
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
