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

// ============================================================
// プロジェクトビューからのファイル操作(右クリックメニュー)
// ------------------------------------------------------------
// 共通方針:
//   - 既存パスを上書きしない(`exists` で先にチェック)
//   - 失敗時はエラー文字列をフロントへ。toast 等でユーザに見せる前提
//   - ディレクトリ作成は parents=true(create_dir_all)
// ============================================================

fn ensure_no_traversal(name: &str) -> Result<(), String> {
    // 単一の名前(ファイル名 / フォルダ名)に "/", "\\", ".." が含まれて
    // いると、親ディレクトリ脱出や別ディレクトリ書き込みになりうる。
    // フロントの input から来た値を保険でガード。
    if name.is_empty() {
        return Err("name must not be empty".into());
    }
    if name.contains('/') || name.contains('\\') || name == ".." || name == "." {
        return Err(format!("invalid name: {:?}", name));
    }
    Ok(())
}

#[tauri::command]
fn create_file(parent: String, name: String) -> Result<String, String> {
    ensure_no_traversal(&name)?;
    let path = PathBuf::from(&parent).join(&name);
    if path.exists() {
        return Err(format!("already exists: {}", path.display()));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|e| format!("failed to ensure parent dir: {}", e))?;
    }
    fs::write(&path, "")
        .map_err(|e| format!("failed to create file {}: {}", path.display(), e))?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn create_folder(parent: String, name: String) -> Result<String, String> {
    ensure_no_traversal(&name)?;
    let path = PathBuf::from(&parent).join(&name);
    if path.exists() {
        return Err(format!("already exists: {}", path.display()));
    }
    fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create folder {}: {}", path.display(), e))?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn rename_path(old_path: String, new_name: String) -> Result<String, String> {
    ensure_no_traversal(&new_name)?;
    let old = PathBuf::from(&old_path);
    let parent = old
        .parent()
        .ok_or_else(|| format!("no parent for {}", old.display()))?;
    let new = parent.join(&new_name);
    if new == old {
        // 名前変更なし(ユーザがそのまま Enter):成功扱いで何もしない
        return Ok(old_path);
    }
    if new.exists() {
        return Err(format!("already exists: {}", new.display()));
    }
    fs::rename(&old, &new)
        .map_err(|e| format!("failed to rename {} -> {}: {}", old.display(), new.display(), e))?;
    Ok(new.to_string_lossy().into_owned())
}

// ============================================================
// プロジェクトビュー: git status 連携
// ------------------------------------------------------------
// `git status --porcelain=v1 -z` の出力をパースして、ファイル毎の
// 状態を返す。git repo でない場合は空マップを返す(エラーにしない、
// プロジェクトビューは git 非依存で動かしたい)。
//
// porcelain=v1 のフォーマット:各レコードは "XY <path>\0" または
// "XY <new>\0<old>\0"(rename / copy 時)。XY は 2 文字の status code。
//   X = index 側(staged)
//   Y = work tree 側(unstaged)
// 1 文字目が " "(space)なら index 変更なし、? は untracked。
//
// フロント側で扱いやすいよう、絶対パス → status code 1 文字の
// マップに正規化する(M / A / ? / D / R / U)。
// ============================================================

#[derive(Serialize)]
struct GitStatus {
    is_repo: bool,
    // key: 絶対パス、value: 1 文字 status code
    entries: std::collections::HashMap<String, String>,
}

#[tauri::command]
fn git_status(folder: String) -> Result<GitStatus, String> {
    let mut result = GitStatus {
        is_repo: false,
        entries: std::collections::HashMap::new(),
    };
    let dir = PathBuf::from(&folder);
    if !dir.is_dir() {
        return Ok(result);
    }
    // git status --porcelain=v1 -z は NUL 区切り + 確定的フォーマット。
    let output = match Command::new("git")
        .current_dir(&dir)
        .args(["status", "--porcelain=v1", "-z"])
        .output()
    {
        Ok(o) => o,
        // git コマンドが無い環境はエラーにせず空マップ
        Err(_) => return Ok(result),
    };
    if !output.status.success() {
        // repo でない場合はここに来る(exit code 128 等)。errcode は捨てる
        return Ok(result);
    }
    result.is_repo = true;

    // -z 出力を NUL で分割。rename/copy 時は次レコードに old name が来るが
    // Yuhitsu は new path だけ見れば十分なので old は読み飛ばす。
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut iter = stdout.split('\0');
    while let Some(rec) = iter.next() {
        if rec.is_empty() {
            continue;
        }
        if rec.len() < 3 {
            continue;
        }
        let bytes = rec.as_bytes();
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        // " <path>" の path 部分は 3 バイト目以降
        let path = &rec[3..];
        // rename / copy('R' / 'C')は old name が次レコードに来るので読み飛ばす
        if x == 'R' || x == 'C' {
            let _ = iter.next();
        }
        // フロントが扱う 1 文字 status を決める。優先度:
        //   ?? (untracked) → "?"
        //   何かしら index 変更あり → X
        //   work tree 変更あり → Y
        // それ以外は最初の非空白を採用。
        let code = if x == '?' && y == '?' {
            '?'
        } else if x != ' ' && x != '?' {
            x
        } else if y != ' ' && y != '?' {
            y
        } else {
            x
        };
        let abs = dir.join(path);
        result
            .entries
            .insert(abs.to_string_lossy().into_owned(), code.to_string());
    }
    Ok(result)
}

#[tauri::command]
fn delete_path(path: String) -> Result<(), String> {
    let buf = PathBuf::from(&path);
    if !buf.exists() {
        return Err(format!("not found: {}", buf.display()));
    }
    if buf.is_dir() {
        fs::remove_dir_all(&buf)
            .map_err(|e| format!("failed to remove dir {}: {}", buf.display(), e))?;
    } else {
        fs::remove_file(&buf)
            .map_err(|e| format!("failed to remove file {}: {}", buf.display(), e))?;
    }
    Ok(())
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

// Tauri resources に同梱した Harano Aji フォント群が置かれるディレクトリ。
// dev では `app/src-tauri/resources/HaranoAjiFonts/`、リリースでは
// `<install>/resources/fonts/`(tauri.conf.json の bundle.resources で
// `fonts/` 配下に名前を付け替えてバンドルしている)。
//
// tinymist の `--font-path` はディレクトリ指定で再帰探索するため、
// dev/リリースで実体パスが違っても tinymist 側は同じ動作になる。
// resources 解決に失敗したら None を返し、呼び出し側はシステムフォント
// 任せのフォールバック動作を続ける(警告は出るが致命的でない)。
fn bundled_font_dir(app: &AppHandle) -> Option<PathBuf> {
    let resource = app.path().resource_dir().ok()?;
    // dev では submodule のディレクトリ名がそのまま、リリースでは
    // bundle.resources のマップで `fonts/` に名前変更されている。
    // 両方を順に試す。
    let release = resource.join("fonts");
    if release.is_dir() {
        return Some(release);
    }
    let dev = resource.join("resources").join("HaranoAjiFonts");
    if dev.is_dir() {
        return Some(dev);
    }
    // src-tauri/ 直下から相対(cargo run 経由)も試す
    let cargo_dev = resource.join("../resources/HaranoAjiFonts");
    if cargo_dev.is_dir() {
        return Some(cargo_dev);
    }
    None
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
    let mut cmd = Command::new("tinymist");
    cmd.current_dir(&root)
        .args([
            "preview",
            "--no-open",
            "--data-plane-host",
            PREVIEW_DATA_PLANE,
            "--control-plane-host",
            PREVIEW_CONTROL_PLANE,
            "--root",
            &root,
        ]);
    if let Some(fonts) = bundled_font_dir(&app) {
        cmd.args(["--font-path", &fonts.to_string_lossy()]);
    }
    cmd.arg(&path);
    let child = cmd
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
fn export_pdf(app: AppHandle, input: String, output: String) -> Result<(), String> {
    // preview と同じく filesystem ルートを `--root` に渡す。文書外の絶対パスを
    // 読みたい(例: ホーム配下のスクショ画像)用途に合わせる。
    let root = filesystem_root_for(&input);
    let mut cmd = Command::new("tinymist");
    cmd.current_dir(&root)
        .args(["compile", "--root", &root]);
    if let Some(fonts) = bundled_font_dir(&app) {
        cmd.args(["--font-path", &fonts.to_string_lossy()]);
    }
    cmd.args([&input, &output]);
    let result = cmd
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

    let mut cmd = TokioCommand::new("tinymist");
    cmd.arg("lsp");
    if let Some(fonts) = bundled_font_dir(&app) {
        cmd.args(["--font-path", &fonts.to_string_lossy()]);
    }
    let mut child = cmd
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

// ============================================================
// タブ状態の永続化
// ------------------------------------------------------------
// 開いていたタブ一覧(file タブのパス + 無題タブの content / dirty)を
// `<app_data_dir>/tabs.json` に保存する。settings.json は氏が直接編集する
// 設計なので、長文 content が混ざらないよう別ファイルに分離する。
// 保存・読み込みのスキーマはフロント側が JSON.stringify / JSON.parse する
// 前提で、Rust は文字列をそのまま read/write するだけ(構造を関知しない)。
// ============================================================

fn tab_state_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app_data_dir: {}", e))?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create data dir {}: {}", dir.display(), e))?;
    Ok(dir.join("tabs.json"))
}

#[tauri::command]
fn save_tab_state(app: AppHandle, payload: String) -> Result<(), String> {
    let path = tab_state_path(&app)?;
    fs::write(&path, payload).map_err(|e| format!("failed to save tab state: {}", e))
}

// 戻り値: 保存ファイルが無ければ空文字列。フロント側はこれを「未保存」と
// 解釈してデフォルト(空タブ 1 枚)で起動する。
#[tauri::command]
fn load_tab_state(app: AppHandle) -> Result<String, String> {
    let path = tab_state_path(&app)?;
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(&path).map_err(|e| format!("failed to read tab state: {}", e))
}

// settings.json の JSON 構文を検証する。Tauri Store の reload はパース失敗
// 時の詳細(行・列)を返さないので、ユーザに「どこが壊れているか」を伝える
// ためにここで serde_json で別途パースして行・列を抽出する。
//
// 戻り値:
//   Ok(None)         パース成功(または settings.json 未作成)
//   Ok(Some(msg))    パースエラー。msg は人間可読な「行 X, 列 Y: ...」
//
// Err は I/O エラー(read 失敗など)のみ。fs read を本体エラーにせず
// Some(msg) で返すかは要検討だが、ファイル read 失敗自体は別問題なので
// Err のままにする。
#[tauri::command]
fn validate_settings_json(app: AppHandle) -> Result<Option<String>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app_data_dir: {}", e))?;
    let path = dir.join("settings.json");
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read settings.json: {}", e))?;
    if content.trim().is_empty() {
        // 空ファイル(初回 seed の "{}\n" すら無い)は許容して null 扱い
        return Ok(None);
    }
    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(_) => Ok(None),
        Err(e) => Ok(Some(format!(
            "settings.json 行 {} 列 {}: {}",
            e.line(),
            e.column(),
            e
        ))),
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
        .plugin(tauri_plugin_os::init())
        .manage(PreviewState::default())
        .manage(LspState::default())
        .invoke_handler(tauri::generate_handler![
            open_file,
            save_file,
            list_directory,
            create_file,
            create_folder,
            rename_path,
            delete_path,
            git_status,
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
            validate_settings_json,
            prepare_untitled_path,
            cleanup_untitled_path,
            save_tab_state,
            load_tab_state
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
