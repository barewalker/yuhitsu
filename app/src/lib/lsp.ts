import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  LSPClient,
  languageServerExtensions,
  type Transport,
} from "@codemirror/lsp-client";

/**
 * Tauri バックエンドで起動した tinymist lsp と双方向 JSON-RPC を交わす Transport。
 *
 * 役割分担:
 *   - 送信(JS → Rust): invoke('lsp_send') で JSON 文字列を Rust に渡す。
 *     Content-Length ヘッダの組み立ては Rust 側で行う。
 *   - 受信(Rust → JS): listen('lsp:message') で Tauri event を購読する。
 *     ヘッダ剥離 / バイト → 文字列の変換は Rust 側で完了している。
 */
class TauriLspTransport implements Transport {
  private handlers = new Set<(value: string) => void>();
  private unlisten: UnlistenFn | null = null;

  async attach(): Promise<void> {
    // 多重 attach を防ぐ
    if (this.unlisten) return;
    this.unlisten = await listen<string>("lsp:message", (event) => {
      const payload = event.payload;
      if (typeof payload !== "string") return;
      for (const handler of this.handlers) {
        try {
          handler(payload);
        } catch (e) {
          console.error("[lsp] handler threw:", e);
        }
      }
    });
  }

  async detach(): Promise<void> {
    this.unlisten?.();
    this.unlisten = null;
    this.handlers.clear();
  }

  send(message: string): void {
    // 失敗は Promise rejection で受け、await はしない(send は同期 API)
    invoke("lsp_send", { message }).catch((e) => {
      console.error("[lsp] send failed:", e);
    });
  }

  subscribe(handler: (value: string) => void): void {
    this.handlers.add(handler);
  }

  unsubscribe(handler: (value: string) => void): void {
    this.handlers.delete(handler);
  }
}

export type LspSession = {
  client: LSPClient;
  transport: TauriLspTransport;
  rootUri: string;
  /** LSP プロセスを止めて、クライアント・購読を解除する */
  shutdown: () => Promise<void>;
};

/**
 * 指定したファイルパスから LSP セッションを起動する。
 *
 * - tinymist lsp を spawn(Tauri command 経由)
 * - Transport を準備し、Tauri event を購読
 * - LSPClient を connect、initialize の rootUri はファイルの親ディレクトリ
 *   を渡す。`file:///` のようなファイルシステムルート相当を渡すと tinymist
 *   が "entry is not in any set root directory" でフォールバックし、補完・
 *   診断・hover 全体が壊れるため避ける。
 *   note: 副作用として、`#image("/abs")` の hover URL は rootUri 起点で
 *   解釈されるが、preview / PDF は Rust 側で `--root /` を渡しているので
 *   実コンパイルには影響しない。
 */
export async function startLspSession(filePath: string): Promise<LspSession> {
  await invoke("lsp_start");
  const transport = new TauriLspTransport();
  await transport.attach();

  const rootUri = pathToFileUri(parentDir(filePath));
  const client = new LSPClient({
    rootUri,
    // 機能本体(補完・診断・hover・format・signature help 等)は
    // languageServerExtensions() を extensions として渡すことで初めて有効になる。
    // languageServerSupport() はファイルとエディタの紐付けを担当するのみ。
    extensions: languageServerExtensions(),
    // 細かい設定(formatterMode 等)は @codemirror/lsp-client では
    // 直接渡せないため、必要になったら自前 Workspace で対応する。
  });
  client.connect(transport);

  return {
    client,
    transport,
    rootUri,
    async shutdown() {
      try {
        client.disconnect();
      } catch (e) {
        console.warn("[lsp] disconnect failed:", e);
      }
      await transport.detach();
      try {
        await invoke("lsp_stop");
      } catch (e) {
        console.warn("[lsp] stop failed:", e);
      }
    },
  };
}

export function pathToFileUri(path: string): string {
  // Linux / macOS は / 始まりの絶対パス、Windows は C:\ 始まり。
  // ここでは Linux / macOS を主対象に簡易実装する(Windows は将来要対応)。
  // パス内の特殊文字はパーセントエンコードする。
  const normalized = path.startsWith("/") ? path : "/" + path.replace(/\\/g, "/");
  return "file://" + encodeURI(normalized);
}

export function parentDir(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}
