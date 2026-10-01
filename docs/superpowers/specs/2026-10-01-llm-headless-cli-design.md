# LLM-friendly headless CLI — design

Date: 2026-10-01. Status: approved sections 1-3 in chat.

## 1. Intent

Agent with only the `contextual` binary must discover capabilities
and build working pipelines without reading the repo.
`contextual [PATH]` stays TUI; new subcommands are headless
(stdout only, no raw mode / alt-screen) for pipes (`jq`, `fzf`, `grep`).

Success: user says "assemble X with contextual", agent runs
`--help` / `llm-help` and then `dump` / `list-files` / `keys`.

## 2. CLI contract (section 1, approved)

Backward compatible: no subcommand + optional `PATH` = current TUI.

New subcommands (dispatch in `main.rs` before `TerminalGuard::enter()`):

- `dump <path> [--lines a:b] [--wrap N|--no-wrap] [--search q] [--format text|json]`
- `list-files [root] [--format text|json] [--limit N]`
- `keys [--format text|markdown|json]`
- `llm-help [--format text|markdown]`

All headless paths: stdout = payload, stderr = errors, non-zero exit
on failure. Never initialize the terminal.

## 3. Components / data flow (section 2, approved)

- `cli.rs`: `Cli { path: Option<PathBuf>, command: Option<Command> }`
  with clap derive. `Command` enum as above.
- New `headless.rs`: pure formatters, no `crossterm` dependency:
  - `dump`: `App::load_file` -> optional `search::find_matches`
    -> `viewer::build_display_lines` -> slice `a:b` (1-based inclusive,
    char-based like the rest of the project) -> render.
    - `text`: `file:line:text` per line (line = logical 1-based).
    - `json`: `{file, lang, lossy, total, returned, truncated,
      matches: [[line, col]], lines: [{n, text}]}`.
      `line`/`col` are 0-based char indices matching `find_matches`.
  - `list-files`: `file_picker::discover_files(root)` (keeps name sort).
    - `text`: one path per line. `json`: `[{name, path, size}]`
      plus `truncated` flag.
  - `keys`: single `KEYS: &[(mode, keys, action)]` table in
    `input/mod.rs`; all formats render from it. No duplicated literals.
  - `llm-help`: static template + generated `Lang` / `Theme::all()`
    names. Sections: what the tool does, dump vs list-files vs TUI,
    3-4 pipe examples, limits.
- No syntax colors in `dump`; only `lang` name. No config writes.

## 4. Errors / limits / tests (section 3, approved)

- Errors via `anyhow` to stderr, nothing partial on stdout.
  Bad `a:b`, missing file, broken pipe: message + non-zero exit,
  same `.context()` style as `load_file`.
- Limits: reuse `search::MAX_MATCHES = 10_000`,
  `file_picker::MAX_PICKER_FILES = 50_000`. `list-files --limit N`
  sets `truncated`. `dump` default = whole file, `--no-wrap` default
  (stable line numbers for agents), `--wrap N` opt-in.
- Tests: `headless.rs` unit (slicing, text/json shape, empty query),
  CLI parse (subcommand never enters TUI), full `cargo test`,
  `cargo clippy -- -D warnings` clean.

## 5. Non-goals

No MCP / JSON-RPC server, no write operations, no TUI changes,
no config mutation from headless paths.
