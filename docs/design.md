# d3term 技術設計書

## 1. 文書情報

- 文書名: d3term 技術設計書
- 対象バージョン: v0.1.x
- 作成日: 2026-02-16
- 対象OS: macOS

## 2. 設計方針

1. ターミナル表示と PTY 接続に責務を限定する。
2. セッション管理は `tmux` / `zellij` に委譲する。
3. 無設定で使える既定値を持たせる。
4. 設定反映は可能な範囲で自動化し、再起動コストを下げる。
5. 失敗時は停止よりフォールバックを優先する。

## 3. 全体構成

### 3.1 レイヤ構成

- Frontend（TypeScript / @xterm/xterm）
  - 端末描画
  - キー入力送信
  - バックエンドイベント反映
- Backend（Rust / Tauri / portable-pty）
  - PTY 起動・入出力
  - multiplexer 起動判定
  - 設定読込とファイル監視

### 3.2 主要ファイル

- `src/main.ts`: アプリ起動エントリ
- `src/terminal.ts`: xterm 管理、Tauri コマンド/イベント連携
- `src/config-client.ts`: フロント側設定型と補正ロジック
- `src-tauri/src/main.rs`: Tauri 起動、state 登録
- `src-tauri/src/commands.rs`: 公開 command
- `src-tauri/src/pty.rs`: PTY セッション管理
- `src-tauri/src/config.rs`: 設定型、読込、監視再読込
- `src-tauri/src/state.rs`: アプリ共有状態

## 4. 起動シーケンス

1. `src/main.ts` で `D3TermApp` を生成し `init()` を呼ぶ。
2. `src/terminal.ts` で xterm を生成し Addon をロードする。
3. 端末テーマと基本表示設定を適用する。
4. `isTauri()` を判定する。
5. Tauri でない場合は「バックエンド未接続」メッセージを表示して終了する。
6. Tauri の場合は `listen()` でイベント購読を登録する。
7. `start_session(cols, rows)` を呼び、PTY セッションを開始する。
8. 入力イベントで `write_stdin(data)` を呼び、リサイズ時に `resize(cols, rows)` を呼ぶ。

## 5. バックエンド設計

### 5.1 AppState

`AppState` は次の2要素を保持する。

- `ConfigManager`: 現在設定とファイル監視
- `SessionManager`: 現在の PTY セッション

### 5.2 command インターフェース

- `start_session(cols: u16, rows: u16) -> SessionInfo`
  - 設定読込済み値に基づき起動コマンドを決定する。
  - `config:updated` を emit してフロントへ現設定を通知する。
- `write_stdin(data: String) -> ()`
  - PTY writer へ文字列を書き込む。
- `resize(cols: u16, rows: u16) -> ()`
  - PTY の行列サイズを更新する。
- `stop_session() -> ()`
  - 既存プロセスを kill する。

### 5.3 event インターフェース

- `pty:data`
  - payload: `{ data: string }`
  - PTY 標準出力/標準エラー由来の表示文字列
- `session:exit`
  - payload: `{ code: number | null }`
  - 子プロセス終了コード
- `warning`
  - payload: `{ message: string }`
  - フォールバックや設定エラー通知
- `config:updated`
  - payload: `{ config: AppConfig, path: string }`
  - 現在有効な設定値

## 6. 起動コマンド解決ロジック

### 6.1 multiplexer 判定

- `none`: shell を使用
- `tmux`: `tmux_command` を使用
- `zellij`: `zellij_command` を使用

### 6.2 zellij 既定補正

互換目的で `zellij_command` が `zellij attach -c` のみだった場合、内部で `d3term` セッション名を補完して `zellij attach -c d3term` として実行する。

### 6.3 コマンド不在時

`PATH` 上で実行ファイルが見つからない場合は `warning` を emit し、shell 起動へフォールバックする。

## 7. 設定設計

### 7.1 設定ファイルパス

1. `$XDG_CONFIG_HOME/d3term/config.toml`
2. `$HOME/.config/d3term/config.toml`（XDG 未設定時）

### 7.2 既定値

- `startup.multiplexer = "zellij"`
- `startup.zellij_command = "zellij attach -c d3term"`
- `startup.tmux_command = "tmux new-session -A -s main"`
- `terminal.theme = "system"`
- `terminal.font_family = "'JetBrains Mono', Menlo, monospace"`
- `terminal.font_size = 13`
- `terminal.letter_spacing = 0`
- `terminal.line_height = 1.2`
- `terminal.scrollback = 10000`

### 7.3 再読込

- `notify` で設定ディレクトリを監視する。
- 連続イベントは 200ms デバウンスで抑制する。
- TOML パース成功時のみ設定を更新する。
- 失敗時は旧設定を維持して `warning` を通知する。

## 8. フロントエンド設計

### 8.1 xterm Addon

- `FitAddon`: コンテナに合わせて列・行を調整
- `WebLinksAddon`: URL 自動リンク
- `Unicode11Addon`: Unicode 幅の互換性向上
- `WebglAddon`: 描画高速化（失敗時は警告表示して継続）

### 8.2 反映タイミング

- `terminal.*`: `config:updated` 受信時に即時反映
- `startup.*`: 次回 `start_session` 時に反映

### 8.3 直接ブラウザアクセス

Tauri 外では `listen` / `invoke` を呼ばず、説明メッセージだけ表示する。これにより DevTools での `transformCallback` 例外を回避する。

## 9. エラーハンドリング

- PTY 起動失敗: command エラー返却
- multiplexer 不在: 警告 + shell フォールバック
- 設定不正: 警告 + 旧設定維持
- WebGL 初期化失敗: 警告 + 通常描画継続
- 子プロセス終了: `session:exit` を表示

## 10. テスト設計

### 10.1 自動テスト

- `src/config-client.test.ts`
  - 設定正規化
  - 数値補正
  - 既定値フォールバック
- `src-tauri/src/config.rs` の unit test
  - 設定パス解決
  - TOML パース
- `src-tauri/src/pty.rs` の unit test
  - コマンドパース
  - zellij 補正
  - フォールバック判定

### 10.2 手動確認

1. `npm run tauri dev` で起動する。
2. コマンド入力が実行される。
3. `config.toml` 編集で `terminal` 項目が反映される。
4. `multiplexer` を `none/tmux/zellij` に切替えて起動確認する。

## 11. 運用方針

- macOS 専用アプリとして設計し、OS 差分吸収レイヤは導入しない。
- telemetry を追加する場合も、端末内容の収集は行わない方針を維持する。
