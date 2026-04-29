# tiny-ai-shell (`ta`)

Natural language to shell command using a local LLM (Ollama).

```
$ ta "今いるディレクトリのファイルを日付順に並べて表示"
$ ls -lt

[y] Execute  [n] Cancel  [c] Copy  [e] Explain  [r] Rewrite
>
```

## Prerequisites

- [Ollama](https://ollama.com/) running locally (`ollama serve`)
- A pulled model (default: `mistral`): `ollama pull mistral`

## Installation

### Option 1: Build and install manually

```bash
git clone https://github.com/coxless/tiny-ai-shell.git
cd tiny-ai-shell
make install          # builds and copies to /usr/local/bin/ta (requires sudo)
```

Or without sudo:

```bash
make build
cp bin/ta ~/.local/bin/ta   # ensure ~/.local/bin is in $PATH
```

### Option 2: cargo install

```bash
cargo install --path .
```

### Option 3: Static binary (musl)

Build a fully static binary with no external library dependencies:

```bash
# Install the musl target first (once)
rustup target add x86_64-unknown-linux-musl

make build-static
sudo cp bin/ta /usr/local/bin/ta
```

## Usage

```
ta [flags] "natural language instruction"

Flags:
  --dry           Show the generated command without executing it
  --explain       Show an explanation immediately after generating the command
  --no-context    Skip sending the current directory context to the LLM
  --model string  Ollama model to use (default: "mistral")
  --url string    Ollama API base URL (default: "http://localhost:11434")
  -h, --help      Show help
  -V, --version   Show version
```

### Interactive keys

| Key | Action |
|-----|--------|
| `y` | Execute |
| `n` / `Esc` / `q` | Cancel |
| `c` | Copy to clipboard |
| `e` | Explain the command |
| `r` | Rewrite with additional instruction |

## Configuration

`~/.config/ta/config.toml` (created automatically on first run):

```toml
model = "mistral"
ollama_url = "http://localhost:11434"
language = "ja"   # language for explanations (ja or en)
```

Priority order: CLI flag > environment variable > config file > default

| Setting | Env var | Default |
|---------|---------|---------|
| model | `TA_MODEL` | `mistral` |
| ollama_url | `TA_OLLAMA_URL` | `http://localhost:11434` |

## Build targets

```bash
make build          # release binary -> bin/ta
make build-static   # static musl binary -> bin/ta
make install        # build + sudo cp bin/ta /usr/local/bin/ta
make test           # run unit tests
make clean          # remove build artifacts
```

## Manual test checklist

- [ ] `ta "list files"` generates and executes a command
- [ ] `y` executes, `n` cancels, `c` copies, `e` explains, `r` rewrites
- [ ] `rm -rf` pattern triggers the danger warning
- [ ] `ta --dry "list files"` prints the command without executing
- [ ] `ta --model llama3 "..."` uses the specified model
- [ ] Ollama not running shows: `Ollama is not running. Start with: ollama serve`
- [ ] Unknown model shows: `Model 'x' not found. Pull with: ollama pull x`
- [ ] Single binary runs without external dependencies
