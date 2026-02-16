# d3term 要件定義書

## 1. 文書情報

- 文書名: d3term 要件定義書
- 対象バージョン: v0.1.x
- 作成日: 2026-02-16
- 対象OS: macOS

## 2. 背景と目的

d3term は、`tmux` / `zellij` と組み合わせて使うことを前提にした、単一セッションの軽量ターミナルエミュレータである。タブやパネル管理は提供せず、端末描画と PTY 接続に責務を限定する。ユーザーは基本的に無設定で利用開始でき、必要な調整のみを `TOML` 設定ファイルで行う。

## 3. 想定ユーザー

- 日常的に `tmux` または `zellij` を利用する開発者
- 設定画面よりも設定ファイル編集を好むユーザー
- 速い起動と低オーバーヘッドを優先するユーザー

## 4. スコープ

### 4.1 対象範囲（In Scope）

- Tauri ベースのデスクトップアプリ（macOS）
- `@xterm/xterm` による端末描画
- portable-pty による PTY 接続
- 単一ウィンドウ・単一セッション
- 設定ファイル読込と自動再読込
- multiplexer 未導入時のフォールバック

### 4.2 対象外（Out of Scope）

- タブ管理、パネル分割、セッション一覧 UI
- 設定 GUI
- SSH クライアント機能
- ファイル転送機能

## 5. 機能要件

### FR-01 起動と表示

アプリ起動時に端末画面を表示できること。初期状態で入力可能であること。

### FR-02 PTY 入出力

キーボード入力を PTY に送信し、PTY 出力を端末へ表示できること。ウィンドウサイズ変更を PTY に反映できること。

### FR-03 multiplexer 起動

設定 `startup.multiplexer` に応じて起動対象を切り替えできること。

- `zellij`: `startup.zellij_command` を実行
- `tmux`: `startup.tmux_command` を実行
- `none`: 通常シェルを実行

### FR-04 multiplexer 不在時フォールバック

指定した multiplexer コマンドが存在しない場合、警告を表示し、通常シェルへ自動フォールバックすること。

### FR-05 設定ファイル探索

設定ファイルは次の順で探索すること。

1. `$XDG_CONFIG_HOME/d3term/config.toml`
2. `$HOME/.config/d3term/config.toml`（`XDG_CONFIG_HOME` 未設定時）

### FR-06 設定ファイル自動再読込

設定ファイル変更を監視し、変更後に自動再読込すること。構文不正時は直前の有効設定を維持すること。

### FR-07 テーマと表示設定

テーマ、フォント、フォントサイズ、行間、スクロールバックを設定で変更できること。`theme = "system"` で OS のライト/ダーク設定に追従すること。

### FR-08 開発時の安全挙動

Tauri WebView 外（通常ブラウザ）で画面を開いた場合は、クラッシュせず、バックエンドが未接続である旨を表示すること。

## 6. 非機能要件

### NFR-01 軽量性

端末機能に責務を限定し、タブ/パネルなどの機能を含めないこと。

### NFR-02 体感性能

入力から表示までの遅延が体感上ストレスなく利用できること。スクロールやリサイズで大きな表示破綻が発生しないこと。

### NFR-03 安定性

起動コマンド不在、設定読み込み失敗、設定監視エラー時にアプリが即時終了しないこと。

### NFR-04 運用容易性

設定項目は `config.toml` で完結し、アプリ再インストールなしで挙動を調整できること。

## 7. 設定要件

### 7.1 設定セクション

- `[startup]`
- `[terminal]`

### 7.2 設定値

- `startup.multiplexer`: `"none" | "tmux" | "zellij"`（既定: `"zellij"`）
- `startup.shell`: 起動シェル文字列（未設定時は環境変数 `SHELL`、なければ `/bin/zsh`）
- `startup.shell_args`: シェル引数配列（空時は `["-l"]`）
- `startup.zellij_command`: 既定 `"zellij attach -c d3term"`
- `startup.tmux_command`: 既定 `"tmux new-session -A -s main"`
- `terminal.theme`: `"system" | "dark" | "light"`（既定: `"system"`）
- `terminal.font_family`: フォント指定（既定: `"'JetBrains Mono', Menlo, monospace"`。スペースを含むフォント名はクォート必須）
- `terminal.font_size`: 数値（UI反映時に 8-72 で補正）
- `terminal.letter_spacing`: 数値（UI反映時に -10.0-10.0 で補正）
- `terminal.line_height`: 数値（UI反映時に 1.0-2.5 で補正）
- `terminal.scrollback`: 数値（UI反映時に 100-200000 で補正）

## 8. 受け入れ基準

1. 起動後に端末が表示され、`echo hello` が実行できる。
2. `multiplexer = "zellij"` で `zellij attach -c d3term` 相当の挙動になる。
3. `multiplexer = "tmux"` で tmux セッションに接続または作成できる。
4. multiplexer 未導入時に警告が表示され、通常シェル起動へ切り替わる。
5. `config.toml` の `terminal` 値変更がアプリ再起動なしで反映される。
6. `config.toml` 構文エラー時にアプリが継続動作する。
7. `npm run test`、`npm run build`、`cargo test` が成功する。

## 9. 既知制約

- 本プロダクトは macOS のみを対象とし、Linux / Windows は検討対象外とする。
- ターミナル管理機能は multiplexer へ委譲する。
