# alfred-agent

A small Rust CLI that periodically runs the [Claude Code](https://claude.com/claude-code)
CLI in headless mode (`claude -p`), feeds it a prompt from a file, parses the
`stream-json` output into a typed model, and logs the final result.

## Requirements

- Rust (edition 2024, i.e. a recent stable toolchain) – <https://rustup.rs>
- The `claude` CLI installed, authenticated, and available in `PATH`

## Install

```bash
git clone <repo-url> alfred_agent && cd alfred_agent
./install.sh
```

The script builds a release binary, installs it as `al` into `~/.local/bin`,
and creates `~/.alfred/` with an empty `system_prompt.txt` and a `prompt.txt`
containing `Hello` (existing files are never overwritten).

| Option | Effect |
| --- | --- |
| `PREFIX=/usr/local ./install.sh` | Install into `/usr/local/bin` instead |
| `./install.sh --uninstall` | Remove the binary (keeps `~/.alfred`) |

Make sure the bin directory is in your `PATH`.

## Usage

```bash
# run claude every 5 minutes
al --run --interval 300

# run once at startup, then every 10 minutes, with a custom model, tools, and log level
al --run -i 600 --immediately -m sonnet --allowed-tools "Read,Grep" -l debug

# inline prompt instead of ~/.alfred/prompt.txt
al --run -i 3600 --prompt "Summarize today's TODOs"
```

`--run` requires `--interval`. Stop the agent with `Ctrl+C`.

Without `--run` the program parses arguments and exits. `al --version` prints `1.0`.

### Options

| Flag | Default | Description |
| --- | --- | --- |
| `--run` | off | Start the periodic loop (requires `--interval`) |
| `-i, --interval <SECS>` | – | Seconds between runs |
| `-m, --model <MODEL>` | `opus` | Claude model (see below) |
| `--immediately` | off | Run once right at startup instead of waiting one full interval |
| `--allowed-tools <LIST>` | unset | Passed to `--allowedTools` when set |
| `--disallowed-tools <LIST>` | unset | Passed to `--disallowedTools` when set |
| `--max-turns <N>` | `50` | Passed to `--max-turns` |
| `-p, --permission-mode <MODE>` | `accept-edits` | `accept-edits`, `bypass-permissions`, or `plan` |
| `--effort <LEVEL>` | `medium` | `low`, `medium`, `high`, `xhigh`, `max`, `ultracode` (`ultracode` is rejected; `claude --effort` doesn't accept it) |
| `--system-prompt <TEXT>` | unset | Inline text appended to the system prompt; takes precedence over `--system-prompt-file` |
| `--prompt <TEXT>` | unset | Inline prompt (wrapped as stream-json); takes precedence over `--prompt-file` |
| `--system-prompt-file <PATH>` | `~/.alfred/system_prompt.txt` | Appended system prompt (`.txt`) |
| `--prompt-file <PATH>` | `~/.alfred/prompt.txt` | Prompt text (`.txt`), wrapped as stream-json and sent on stdin |
| `--mcp-config <PATH>` | `~/.alfred/mcp_config.json` | MCP config (`.json`), passed to `--mcp-config` if the file exists |
| `--settings <PATH>` | `~/.alfred/settings.json` | Claude settings (`.json`), passed to `--settings` if the file exists |
| `-l, --log-level <LEVEL>` | `info` | `debug`, `info`, `warn`, `error`, `trace` (lowercase) |
| `-v, --verbose` | off | Accepted but not used yet |
| `--duration <VALUE>` | `""` | Accepted but not used yet |

Supported models: `opus`, `sonnet`, `haiku`, `claude-opus-5-5`,
`claude-sonnet-5-5`, `claude-haiku-5-5`, `claude-opus-5`, `claude-sonnet-5`,
`claude-opus-4-8`, `claude-sonnet-4-6`, `claude-haiku-4-5`. Any other value is
rejected when a run starts; the error is logged and the loop continues.

### Config files (`~/.alfred/`)

- `system_prompt.txt` – plain text, passed to `--append-system-prompt`. Missing or empty = skipped.
  Must have a `.txt` extension.
- `prompt.txt` – plain-text prompt. Its content is placed in the `content` field of a
  `stream-json` input message before being written to Claude's stdin, e.g. a file
  containing `Summarize today's TODOs` is sent as

  ```json
  {"type":"user","message":{"role":"user","content":"Summarize today's TODOs"}}
  ```

  If missing or empty, nothing is written to Claude's stdin.
- `mcp_config.json` – MCP server configuration. Missing = skipped.
- `settings.json` – Claude Code settings file. Missing = skipped.

A leading `~` in file paths is expanded to your home directory.

## How it works

1. `main.rs` parses `Args` (clap), initializes `tracing`, and builds an `Agent`.
2. `Agent::run` spawns two tokio tasks joined by an mpsc channel:
   - **trigger** – sleeps `interval` seconds, then sends a signal.
   - **worker** – on each signal runs `handle()`.
   Ctrl+C cancels both through a `CancellationToken`.
3. `handle()` spawns:
   `claude -p --output-format stream-json --verbose [--append-system-prompt ...] [--allowedTools ...] [--disallowedTools ...] --max-turns <n> --model <m> [--mcp-config <f>] [--settings <f>] --effort <e> --permission-mode <p> --input-format stream-json`
   writes the prompt to stdin, waits, and parses stdout line by line.
4. The `result` message text is logged at `INFO`. A non-zero exit code or a config
   error (bad model, wrong file extension, `ultracode` effort) is logged as an
   error and the loop continues.

## Project layout

```
src/
├── main.rs          # entry point
├── lib.rs           # module exports
├── cli.rs           # CLI args, file loading, model validation
├── config.rs        # env-based Config (.env via dotenvy; not yet wired in)
├── trace.rs         # tracing subscriber setup
└── agent/
    ├── mod.rs
    ├── agent.rs     # scheduler loop and claude subprocess handling
    └── model.rs     # typed model + parser for claude stream-json output
install.sh           # build & install script
```

## Development

```bash
cargo build
cargo run -- --run --interval 60 --immediately -l debug
cargo test
```

## Notes

- The agent runs Claude with `--permission-mode acceptEdits` by default; restrict
  `--allowed-tools` and use `--permission-mode plan` for read-only behavior.
- The first run happens after one full interval unless `--immediately` is set.
- `--permission-mode bypass-permissions` skips all permission checks; use with care.
