# tiny-ai-shell 実装計画

## 概要

`ta` コマンドとして動作するCLIツール。ローカルLLM（Ollama）を使って自然言語からシェルコマンドを生成し、安全な確認フローを経て実行する。

各ステップは **プラン作成 → 実装** の順で進める。

---

## ステップ一覧

| # | タイトル | 内容 |
|---|----------|------|
| 1 | プロジェクト骨格 | 言語選定・ディレクトリ構成・ビルド設定 |
| 2 | LLMクライアント | Ollama API通信・プロンプト生成 |
| 3 | コンテキスト取得 | pwd / ls / .git 検出 |
| 4 | セーフティ機構 | 危険コマンド検知・警告表示 |
| 5 | 確認フロー（UI） | y/n/c/e/r インタラクティブ入力 |
| 6 | コマンド実行 | シェルコマンド実行・結果表示 |
| 7 | クリップボード対応 | pbcopy / xclip / wl-copy |
| 8 | コマンド説明（explain） | LLMによるコマンド解説 |
| 9 | 修正（rewrite） | 追加指示による再生成 |
| 10 | ログ機能 | JSON形式の実行履歴保存 |
| 11 | CLIオプション | --dry / --explain / --no-context / --model |
| 12 | パッケージング | 単一バイナリビルド・インストール手順 |

---

## ステップ詳細

---

### Step 1: プロジェクト骨格

#### プラン

**言語選定**: Go

- 単一バイナリビルドが容易
- 起動100ms以内を満たす
- クロスプラットフォーム対応
- CGO不要で依存最小化

**ディレクトリ構成**:

```
tiny-ai-shell/
├── cmd/
│   └── ta/
│       └── main.go          # エントリポイント
├── internal/
│   ├── llm/                 # LLMクライアント
│   │   └── ollama.go
│   ├── context/             # コンテキスト取得
│   │   └── context.go
│   ├── safety/              # セーフティ機構
│   │   └── safety.go
│   ├── ui/                  # 確認フロー・表示
│   │   └── prompt.go
│   ├── executor/            # コマンド実行
│   │   └── executor.go
│   ├── clipboard/           # クリップボード
│   │   └── clipboard.go
│   └── logger/              # ログ
│       └── logger.go
├── go.mod
├── go.sum
├── Makefile
└── README.md
```

**依存パッケージ**（最小限）:
- `github.com/spf13/cobra` - CLI引数パース
- 標準ライブラリのみで他は実装

**実装内容**:
- `go mod init`
- エントリポイント（`cmd/ta/main.go`）のスタブ
- Makefile（`make build` で `./bin/ta` 生成）
- 各 internal パッケージのスタブファイル

**完了条件**: `make build && ./bin/ta --help` が動作する

---

### Step 2: LLMクライアント

#### プラン

**対象**: Ollama REST API（`http://localhost:11434`）

**エンドポイント**: `POST /api/generate`

**プロンプト設計**:

```
System:
You are a CLI assistant.
Rules:
- Output exactly ONE shell command
- No explanation, no markdown, no code blocks
- Avoid dangerous commands
- Prefer safe flags
- Consider the OS: {os}
- Current directory: {pwd}

User:
{input}
```

**レスポンス処理**:
- ストリーミングレスポンスを受け取り、`response` フィールドを結合
- 前後の空白・改行を除去
- コードブロック（`` ` ``）が含まれる場合は除去

**設定**:
- デフォルトモデル: `mistral` （`~/.config/ta/config.toml` で上書き可）
- タイムアウト: 30秒
- ベースURL: 環境変数 `TA_OLLAMA_URL` で上書き可

**エラーハンドリング**:
- Ollamaが起動していない場合: `"Ollama is not running. Start with: ollama serve"`
- モデルが存在しない場合: `"Model 'xxx' not found. Pull with: ollama pull xxx"`

**実装内容**:
- `internal/llm/ollama.go`: API通信・レスポンスパース
- `internal/llm/prompt.go`: プロンプト構築
- ユニットテスト（モックサーバー使用）

**完了条件**: `llm.Generate(ctx, input, contextInfo)` がOllamaからコマンド文字列を返す

---

### Step 3: コンテキスト取得

#### プラン

**取得情報**（軽量・安全）:

| 情報 | 取得方法 | 制限 |
|------|----------|------|
| カレントディレクトリ | `os.Getwd()` | なし |
| ファイル一覧 | `os.ReadDir()` | 最大20件、ファイル名のみ |
| Gitリポジトリ | `.git` ディレクトリの存在確認 | ブランチ名のみ |
| OS | `runtime.GOOS` | なし |

**出力形式**:

```go
type ContextInfo struct {
    OS      string
    PWD     string
    Files   []string  // 最大20件
    IsGit   bool
    Branch  string    // Gitの場合のみ
}
```

**`--no-context` フラグ時**: 空の `ContextInfo` を返す

**実装内容**:
- `internal/context/context.go`: コンテキスト取得ロジック

**完了条件**: `context.Gather()` が構造体を返す

---

### Step 4: セーフティ機構

#### プラン

**危険パターン定義**:

```go
var dangerousPatterns = []DangerPattern{
    {Pattern: `rm\s+-rf`, Message: "Recursive force delete"},
    {Pattern: `sudo\s+`, Message: "Elevated privileges required"},
    {Pattern: `chmod\s+777`, Message: "Insecure file permissions"},
    {Pattern: `curl[^|]+\|\s*sh`, Message: "Piping curl to shell"},
    {Pattern: `curl[^|]+\|\s*bash`, Message: "Piping curl to bash"},
    {Pattern: `>\s*/dev/sd`, Message: "Writing to block device"},
    {Pattern: `mkfs`, Message: "Filesystem formatting"},
    {Pattern: `dd\s+if=`, Message: "Low-level disk operation"},
    {Pattern: `:\(\)\{.*\}`, Message: "Fork bomb detected"},
}
```

**検知時の表示**:

```
⚠  Dangerous command detected: Recursive force delete
   rm -rf /tmp/test

Continue anyway? [y/N]:
```

- デフォルトは `N`（キャンセル）
- `y` で続行した場合も確認フローに進む（実行は確認フロー側）

**実装内容**:
- `internal/safety/safety.go`: パターンマッチング
- ユニットテスト（各パターンの検知確認）

**完了条件**: `safety.Check(command)` が危険判定と理由を返す

---

### Step 5: 確認フロー（UI）

#### プラン

**表示フォーマット**:

```
$ ls -la

[y] Execute  [n] Cancel  [c] Copy  [e] Explain  [r] Rewrite
> 
```

**入力処理**:
- 生のキー入力（Enterなし）: `term.MakeRaw()` を使用
- 大文字小文字どちらも受け付ける
- 不明なキーは無視してプロンプト再表示

**キーマッピング**:

| キー | アクション | 戻り値 |
|------|-----------|--------|
| `y` / `Y` | 実行 | `ActionExecute` |
| `n` / `N` / `ESC` / `q` | キャンセル | `ActionCancel` |
| `c` / `C` | コピー | `ActionCopy` |
| `e` / `E` | 説明 | `ActionExplain` |
| `r` / `R` | 修正 | `ActionRewrite` |

**カラー出力**:
- コマンド: シアン（`\033[36m`）
- 危険警告: 赤（`\033[31m`）
- プロンプト: 標準
- `NO_COLOR` 環境変数でオフ

**依存**: `golang.org/x/term` のみ

**実装内容**:
- `internal/ui/prompt.go`: 表示・入力処理
- `internal/ui/color.go`: カラー出力ヘルパー

**完了条件**: インタラクティブな確認フローが動作する

---

### Step 6: コマンド実行

#### プラン

**実行方式**:

```go
cmd := exec.Command("sh", "-c", command)
cmd.Stdout = os.Stdout
cmd.Stderr = os.Stderr
cmd.Stdin = os.Stdin
```

- ユーザーのシェルを継承（`$SHELL` 環境変数使用、フォールバックは `sh`）
- コマンドの終了コードを `ta` の終了コードとして伝播

**実行前後の表示**:

```
→ Executing: ls -la
[コマンドの出力]
✓ Done (exit 0)
```

失敗時:
```
✗ Failed (exit 1)
```

**`--dry` モード時**: 実行せずコマンドを表示して終了

**実装内容**:
- `internal/executor/executor.go`: コマンド実行ロジック

**完了条件**: コマンドが実行され、終了コードが伝播される

---

### Step 7: クリップボード対応

#### プラン

**OS別実装**:

| OS | コマンド | フォールバック |
|----|---------|--------------|
| macOS | `pbcopy` | なし |
| Linux (X11) | `xclip -selection clipboard` | `xsel --clipboard` |
| Linux (Wayland) | `wl-copy` | xclip にフォールバック |
| その他 | エラーメッセージ表示 | なし |

**Wayland検知**: `$WAYLAND_DISPLAY` 環境変数の有無

**実装**:

```go
func Copy(text string) error {
    // OS判定 → 適切なコマンドにパイプ
}
```

**コピー後の表示**:
```
✓ Copied to clipboard
```

**実装内容**:
- `internal/clipboard/clipboard.go`

**完了条件**: 各OS環境でコマンドがクリップボードにコピーされる

---

### Step 8: コマンド説明（explain）

#### プラン

**動作**:
1. 確認フローで `e` を選択
2. LLMに「このコマンドを日本語で簡潔に説明して」とリクエスト
3. 説明を表示後、確認フローに戻る

**プロンプト**:

```
Explain this shell command concisely in {language}:
{command}

Rules:
- One or two sentences maximum
- Focus on what it does, not how flags work in detail
- Use plain language
```

**言語検知**: `$LANG` / `$LC_ALL` 環境変数から判定（ja_JP なら日本語）

**表示例**:

```
$ ls -la
  → すべてのファイル（隠しファイルを含む）を詳細情報付きで一覧表示します。

[y] Execute  [n] Cancel  [c] Copy  [e] Explain  [r] Rewrite
> 
```

**完了条件**: `e` 押下でコマンド説明が表示され、フローに戻る

---

### Step 9: 修正（rewrite）

#### プラン

**動作**:
1. 確認フローで `r` を選択
2. 追加指示を入力するプロンプトを表示
3. 元のリクエスト + 生成コマンド + 追加指示でLLMに再リクエスト
4. 新しいコマンドを表示し、確認フローに戻る

**表示例**:

```
[r] Rewrite
Instruction: 隠しファイルはいらない

$ ls -a
→ Rewriting...
$ ls

[y] Execute  [n] Cancel  [c] Copy  [e] Explain  [r] Rewrite
> 
```

**修正用プロンプト**:

```
Original request: {original_input}
Generated command: {command}
User feedback: {rewrite_instruction}

Generate an improved command based on the feedback.
Output exactly ONE shell command, no explanation.
```

**完了条件**: `r` 押下で再生成フローが動作する

---

### Step 10: ログ機能

#### プラン

**保存先**: `~/.local/share/ta/history.json`（Linux） / `~/Library/Application Support/ta/history.json`（macOS）

**ログエントリ形式**:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "input": "フォルダ内のファイル一覧",
  "command": "ls -la",
  "action": "executed",
  "exit_code": 0,
  "context": {
    "pwd": "/home/user/project",
    "os": "linux"
  }
}
```

**`action` フィールド**: `executed` / `cancelled` / `copied` / `dry-run`

**ログサイズ制限**: 最大1000件（古いエントリを自動削除）

**実装内容**:
- `internal/logger/logger.go`: JSON読み書き・ローテーション

**完了条件**: 各アクション後にログが記録される

---

### Step 11: CLIオプション

#### プラン

**コマンド仕様**:

```
ta [flags] "自然言語の指示"

Flags:
  --dry           コマンドを表示するだけで実行しない
  --explain       コマンド生成後、即座に説明を表示
  --no-context    カレントディレクトリのコンテキストを送らない
  --model string  使用するOllamaモデル (default: "mistral")
  --url string    Ollama APIのURL (default: "http://localhost:11434")
  -h, --help      ヘルプを表示
  -v, --version   バージョンを表示
```

**設定ファイル** (`~/.config/ta/config.toml`):

```toml
model = "mistral"
ollama_url = "http://localhost:11434"
language = "ja"  # explain時の言語
```

**優先順位**: CLIフラグ > 環境変数 > 設定ファイル > デフォルト値

**実装内容**:
- `cmd/ta/main.go`: cobra によるCLI定義
- `internal/config/config.go`: 設定読み込み

**完了条件**: すべてのオプションが正しく動作する

---

### Step 12: パッケージング

#### プラン

**ビルド**:

```makefile
build:
    go build -ldflags="-s -w -X main.Version=$(VERSION)" \
        -o bin/ta ./cmd/ta

# クロスコンパイル
build-linux:
    GOOS=linux GOARCH=amd64 go build ...

build-darwin:
    GOOS=darwin GOARCH=arm64 go build ...
```

**インストール手順** (README):

```bash
# Homebrewは後回し、まず手動インストール
make build
sudo cp bin/ta /usr/local/bin/ta

# または go install
go install github.com/user/tiny-ai-shell/cmd/ta@latest
```

**前提チェック**:
- `ta` 起動時にOllamaの疎通確認（失敗時はわかりやすいメッセージ）
- 初回起動時に設定ディレクトリを自動作成

**テスト**:
- `go test ./...` でユニットテスト全通過
- 主要シナリオの手動テストチェックリスト

**完了条件**: 単一バイナリで配布可能、README にインストール手順記載

---

## 実装順序の根拠

```
Step 1 (骨格)
    ↓
Step 2 (LLM) + Step 3 (コンテキスト)  ← 並行可
    ↓
Step 4 (セーフティ) + Step 5 (UI)     ← 並行可
    ↓
Step 6 (実行)                          ← Step 4,5 依存
    ↓
Step 7,8,9,10 (各機能)                ← 並行可
    ↓
Step 11 (CLIオプション統合)
    ↓
Step 12 (パッケージング)
```

---

## MVP完了条件

- [ ] `ta "ls"` でコマンドが生成される
- [ ] y/n/c で対応するアクションが実行される
- [ ] `rm -rf` が危険と検知される
- [ ] `--dry` モードが動作する
- [ ] 単一バイナリでビルドできる
