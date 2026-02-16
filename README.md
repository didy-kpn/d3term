# d3term

`d3term` は、`Tauri + @xterm/xterm + portable-pty` で構成した、単一セッション前提の軽量ターミナルエミュレータです。  
タブやパネルの管理は行わず、`tmux` / `zellij` と組み合わせて使う設計です。

## ユーザー向けの使い方

### 1. 前提環境

- macOS
- Node.js 20 以上
- Rust stable
- `zellij` または `tmux`（任意）

`d3term` は現状 macOS 専用です。

### 2. 起動

```bash
npm install
npm run tauri dev
```

起動後は Tauri ウィンドウで端末が開きます。  
既定では `zellij attach -c d3term` を実行します。

### 3. 基本操作

- 端末入力はそのまま子プロセスに送信されます。
- ウィンドウリサイズに合わせて PTY サイズが更新されます。
- プロセス終了時は `[process exited: <code>]` が表示されます。

## 設定ファイル

### 配置パス

次の順序で読み込みます。

1. `$XDG_CONFIG_HOME/d3term/config.toml`
2. `$HOME/.config/d3term/config.toml`（`XDG_CONFIG_HOME` 未設定時）

作成例:

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/d3term"
cp config/config.example.toml "${XDG_CONFIG_HOME:-$HOME/.config}/d3term/config.toml"
```

### 推奨設定（zellij 運用）

```toml
[startup]
multiplexer = "zellij"
shell = "/bin/zsh"
shell_args = ["-l"]
zellij_command = "zellij attach -c d3term"
tmux_command = "tmux new-session -A -s d3term"

[terminal]
theme = "system"
font_family = "'JetBrains Mono', Menlo, monospace"
font_size = 13
letter_spacing = 0
line_height = 1.2
scrollback = 10000
```

### 設定項目の詳細

#### `[startup]`

- `multiplexer`
  - 値: `"none" | "tmux" | "zellij"`
  - 既定値: `"zellij"`
  - 説明: 起動時に使うプロセスを指定します。
- `shell`
  - 値: 文字列（例: `"/bin/zsh"`）
  - 既定値: `SHELL` 環境変数（未設定時は `"/bin/zsh"`）
  - 説明: `multiplexer = "none"` やフォールバック時の起動シェルです。
- `shell_args`
  - 値: 文字列配列
  - 既定値: `[]`（内部で `["-l"]` を補完）
  - 説明: シェル起動時の引数です。
- `zellij_command`
  - 値: 文字列
  - 既定値: `"zellij attach -c d3term"`
  - 説明: `multiplexer = "zellij"` 時に実行します。
- `tmux_command`
  - 値: 文字列
  - 既定値: `"tmux new-session -A -s main"`
  - 説明: `multiplexer = "tmux"` 時に実行します。

#### `[terminal]`

- `theme`
  - 値: `"system" | "dark" | "light"`
  - 既定値: `"system"`
  - 説明: `system` は macOS のライト/ダーク設定に追従します。
- `font_family`
  - 値: 文字列
  - 既定値: `"'JetBrains Mono', Menlo, monospace"`
  - 説明: `@xterm/xterm` の `fontFamily` に適用します。スペースを含むフォント名は `'<name>'` のようにクォートしてください（例: `"'GoMono Nerd Font Mono'"`）。
- `font_size`
  - 値: 数値
  - 既定値: `13`
  - 説明: UI 反映時に `8-72` の範囲へ補正されます。
- `line_height`
  - 値: 数値
  - 既定値: `1.2`
  - 説明: UI 反映時に `1.0-2.5` の範囲へ補正されます。
- `letter_spacing`
  - 値: 数値
  - 既定値: `0`
  - 説明: 文字の横方向の間隔です。UI 反映時に `-10.0-10.0` の範囲へ補正されます。詰めたい場合は `-3` から `-8` を試してください。
- `scrollback`
  - 値: 数値
  - 既定値: `10000`
  - 説明: UI 反映時に `100-200000` の範囲へ補正されます。

### 反映タイミング

- `terminal.*` は保存後に自動再読込され、即時反映されます。
- `startup.*` は次回セッション起動時に反映されます。

## プリセット例

### tmux を既定にする

```toml
[startup]
multiplexer = "tmux"
tmux_command = "tmux new-session -A -s d3term"
```

### 通常シェルで起動する

```toml
[startup]
multiplexer = "none"
shell = "/bin/zsh"
shell_args = ["-l"]
```

## トラブルシューティング

- `Please specify the session to attach to ...` が出る
  - `zellij` のアタッチ先が曖昧です。`zellij_command = "zellij attach -c d3term"` を使ってください。
- `warning` が出てシェル起動になる
  - 指定した `tmux` / `zellij` の実行ファイルが見つからない状態です。PATH または設定値を確認してください。
- ローカルサーバー直アクセスでエラーが出る
  - `http://localhost:1420` はブラウザ確認用です。PTY は Tauri ウィンドウでのみ起動します。
- 設定変更が反映されない
  - TOML の構文エラーを確認してください。構文エラー時は前回有効設定を維持します。
- 日本語の文字間が広く見える
  - `letter_spacing = -3` から `-8` を試してください。あわせて `font_family = "'GoMono Nerd Font Mono'"` のようにフォント名をクォートしてください。
- 半角英数の文字間も広い
  - `letter_spacing` は横方向の文字間隔です。`line_height` では改善しません。`letter_spacing = -5` 前後まで下げて調整してください。

## テスト

```bash
npm run test
npm run build
cd src-tauri && cargo test
```

## 仕様書

- 要件定義書: `docs/requirements.md`
- 技術設計書: `docs/design.md`
- 設定例: `config/config.example.toml`
