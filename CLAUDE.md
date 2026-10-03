# CLAUDE.md

## Commands
- Build: `cargo build --locked`
- Run TUI: `cargo run --locked -- [env]` (needs a config: `cargo run -- init`, `./kitz.toml`, or `-- --config <path>`)
- Connectivity check, no TUI: `cargo run --locked -- doctor [env]`
- Test (all): `cargo test --locked`
- Test (one module/test): `cargo test --locked config::` or `cargo test --locked <name>`
- Format: `cargo fmt --all` (CI: `cargo fmt --all -- --check`)
- Lint: `cargo clippy --all-targets --locked -- -D warnings`

## Structure
Single binary crate, flat `src/`:
- `config.rs` — TOML env profiles (`[[env]]`), `Auth` enum. New config fields go here.
- `kafka.rs` — all librdkafka/MSK IAM calls and the `doctor` command. Blocking.
- `worker.rs` — background thread; `Cmd` in, `Evt` out. UI never calls `kafka` directly.
- `app.rs` — state + key handling (two views: `View::Topics` / `View::Groups`); `ui.rs` — rendering only (+ smoke/behaviour tests); `theme.rs` — colors and branding consts.

## Conventions
- Before writing a helper, grep for one. `ui.rs` already has `truncate`, `fmt_count`, `pretty_json`, `centered`.
- Files under ~30 lines that belong to one module go in that module, not in a new file.
- Errors: `anyhow::Result` + `.context(...)`; no `unwrap` on I/O or network results.
- Colors only via `theme::` consts — never inline `Color::Rgb` in `ui.rs`.
- Any destructive Kafka op must respect `EnvProfile::prod` (typed confirmation).

## Gotchas
- First build is slow (~2 min): librdkafka + OpenSSL are vendored via cmake. Use long timeouts; needs `cmake` installed.
- CI tests/lints on macOS only (SASL links against system libsasl2).
- librdkafka logs go to stderr; the TUI redirects fd 2 to `~/Library/Caches/kitz/kitz.log` (see `main.rs`). Never `eprintln!` for user-facing TUI messages.
- Without a config file the binary exits immediately ("no config found"); `--help`/`--version` work without one.
- `kitz.toml` is gitignored — it holds real broker addresses. Never commit it.
- UI tests render on a `TestBackend` (see `demo_app()` in `ui.rs`); there is no live-Kafka test.
- `KeyCode::*` is glob-imported in `app.rs` input handlers, so a type named `Tab` would be shadowed - hence `View` and the `Tab_` alias.
- ratatui word-wrap drops the line after a whitespace-only line; give empty fields a placeholder.
- Releases are automated (release-plz + cargo-dist). Don't hand-edit `CHANGELOG.md` or bump the version.

## Working on an issue
1. Reproduce with a failing test in the `#[cfg(test)] mod tests` of the relevant file.
2. Fix with minimal churn; reuse existing helpers.
3. Run fmt, clippy, and the full test suite — all must pass.
4. Branch `fix/<short-desc>` or `feat/<short-desc>`; conventional commit message (drives the changelog).
5. PR description: root cause, what changed, tests added.
