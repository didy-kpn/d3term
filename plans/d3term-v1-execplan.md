# d3term v1 を Tauri + xterm.js + portable-pty で実装する


This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This repository includes `/.codex/.agent/PLANS.md` outside the repo root at `$HOME/.codex/.agent/PLANS.md`. This plan is maintained in accordance with that document.

## Purpose / Big Picture


この変更により、ユーザーは起動直後から使える単一セッションの高速ターミナルを利用できます。タブやパネルは持たず、必要な多重化は tmux/zellij に委譲します。設定ファイルを置かなくても動作し、設定を置いた場合は保存後に自動で再読込されるため、再起動なしで見た目を調整できます。

## Progress


- [x] (2026-02-16 10:40Z) Vite + TypeScript + xterm.js のフロントエンド骨格を追加した。
- [x] (2026-02-16 10:44Z) Tauri プロジェクト骨格とコマンド配線を追加した。
- [x] (2026-02-16 10:48Z) portable-pty で PTY セッション起動・入力・リサイズ・停止を実装した。
- [x] (2026-02-16 10:52Z) XDG 設定パス解決・TOML 読込・notify 監視再読込を実装した。
- [x] (2026-02-16 13:30Z) `npm run test`、`npm run build`、`cargo test` を実行し、全件成功を確認した。
- [x] (2026-02-16 13:55Z) `docs/requirements.md` と `docs/design.md` を追加し、`README.md` をユーザー向けに詳細化した。
- [x] (2026-02-16 14:47Z) `@xterm/*` へ依存移行し、`config/config.example.toml` へ設定例を移動し、`letter_spacing` 設定を追加した。
- [x] (2026-02-16 15:10Z) `font_family` の自動クォートと `letter_spacing` の補正範囲拡張（-10〜10）を実装し、表示間隔問題の調整幅を増やした。
- [x] (2026-02-16 15:30Z) Ghostty 風の表示へ寄せるため、暗色パレット調整・Canvas描画固定・フォントウェイト調整・背景/余白/スムージングCSS調整を実施した。

## Surprises & Discoveries


- Observation: 初期リポジトリは `LICENSE` のみで、実装はゼロから作成する必要があった。
  Evidence: `ls -la` で `LICENSE` 以外のアプリ実装ファイルが存在しなかった。

- Observation: `xterm@^5.5.0` は npm 上に存在せず、実在バージョンへ調整が必要だった。
  Evidence: `npm install` で `No matching version found for xterm@^5.5.0` が発生した。

- Observation: Tauri の `generate_context!` は `icons/icon.png` の RGBA PNG を要求した。
  Evidence: `cargo test` で `icon ... is not RGBA` エラーが発生し、RGBA PNG 追加で解消した。

## Decision Log


- Decision: v1 の対応 OS は macOS に限定する。
  Rationale: 初期品質を高く保ちながら PTY 差分対応コストを抑えるため。
  Date/Author: 2026-02-16 / Codex

- Decision: 多重化のデフォルトを zellij (`zellij attach -c d3term`) にする。
  Rationale: セッション既存時は接続し、未存在時は自動作成されるため無設定運用に向くため。
  Date/Author: 2026-02-16 / Codex

- Decision: 設定は `TOML` ファイルのみとし、UI での設定編集は実装しない。
  Rationale: 軽量性と実装の単純性を維持するため。
  Date/Author: 2026-02-16 / Codex

- Decision: 設定はファイル監視で自動再読込する。
  Rationale: 再起動不要での調整を可能にし、運用体験を改善するため。
  Date/Author: 2026-02-16 / Codex

- Decision: tmux/zellij が見つからない場合は警告表示して通常シェルへフォールバックする。
  Rationale: 起動不能を避け、最低限の利用継続性を優先するため。
  Date/Author: 2026-02-16 / Codex

## Outcomes & Retrospective


フロントエンド・バックエンド・設定再読込・フォールバック挙動まで実装し、`npm run test`、`npm run build`、`cargo test` の全件成功を確認しました。さらに、仕様の明文化として要件定義書と技術設計書を追加し、README をユーザー運用中心に拡充しました。運用方針は macOS 専用のまま軽量性を優先します。

## Context and Orientation


このリポジトリは実装開始時点で `LICENSE` のみでした。新規構成として、フロントエンドは `src/main.ts` と `src/terminal.ts` を起点に xterm.js で描画します。バックエンドは `src-tauri/src/main.rs` から起動し、`src-tauri/src/commands.rs` で Tauri コマンドを公開し、`src-tauri/src/pty.rs` で PTY を管理し、`src-tauri/src/config.rs` で設定ファイル読込と監視を行います。設定型は `AppConfig`, `StartupConfig`, `TerminalConfig`, `ThemeMode`, `MultiplexerMode` です。

## Plan of Work


フロントエンドでは xterm インスタンスの生成、各種 Addon の読み込み、バックエンドイベント購読、入力送信、リサイズ通知を実装します。バックエンドでは start/write/resize/stop コマンドを定義し、`portable-pty` でシェルまたは tmux/zellij を起動します。設定は XDG 規約に基づくパスから TOML を読み込み、`notify` で変化検知したときに再読込し、`config:updated` イベントでフロントへ反映します。

## Concrete Steps


作業ディレクトリはリポジトリルートです。

1. 依存定義とビルド設定を追加する (`package.json`, `tsconfig*.json`, `vite.config.ts`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`)。
2. フロント実装を追加する (`src/main.ts`, `src/terminal.ts`, `src/config-client.ts`, `src/styles.css`)。
3. バックエンド実装を追加する (`src-tauri/src/main.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/pty.rs`, `src-tauri/src/config.rs`, `src-tauri/src/state.rs`)。
4. ドキュメントを追加する (`README.md`)。
5. テストを追加し実行する (`src/config-client.test.ts`, Rust の `#[cfg(test)]`)。

## Validation and Acceptance


以下を満たせば受け入れとします。

1. アプリ起動後、ターミナルが表示され、入力したコマンドが実行される。
2. `multiplexer = "zellij"` で `zellij attach -c d3term` が使われる。
3. zellij 未導入時は警告を出し、通常シェルで利用を継続できる。
4. `config.toml` 編集後にフォントサイズやテーマなどが再起動なしで反映される。
5. `npm run test` と `cargo test` が通る。

## Idempotence and Recovery


この実装は加算的であり、同じ手順を再実行しても既存設定を破壊しません。設定ファイルが存在しない場合は既定値で動作し、無効な TOML の場合は直前の設定を維持しつつ警告表示します。多重化コマンドが失敗してもシェルフォールバックで起動不能を回避します。

## Artifacts and Notes


主要ファイル:

- `src/terminal.ts`: xterm 初期化・I/O・テーマ適用・警告表示。
- `src-tauri/src/pty.rs`: PTY 起動、stdin 書込、resize、終了通知。
- `src-tauri/src/config.rs`: XDG 設定パス、TOML 読込、notify 再読込。
- `src-tauri/src/commands.rs`: Tauri コマンドの公開。

## Interfaces and Dependencies


公開コマンド:

- `start_session(cols: u16, rows: u16) -> SessionInfo`
- `write_stdin(data: String) -> ()`
- `resize(cols: u16, rows: u16) -> ()`
- `stop_session() -> ()`

イベント:

- `pty:data`
- `session:exit`
- `config:updated`
- `warning`

主要依存:

- Frontend: `@xterm/xterm`, `@xterm/addon-fit`, `@xterm/addon-web-links`, `@xterm/addon-webgl`, `@xterm/addon-unicode11`
- Backend: `tauri`, `portable-pty`, `notify`, `toml`, `serde`, `shell-words`

この改訂では、ユーザー要件の「無設定で使える軽量端末」を満たすために、単一セッション端末の最小構成を新規実装し、設定ファイル運用と多重化フォールバックを明示化しました。
