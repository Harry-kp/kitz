# Contributing

Thanks for helping. kitz is maintained by one person, so a little coordination
up front saves us both time.

## Build and test

```sh
brew install cmake          # librdkafka is built from source
cargo build                 # first build takes a few minutes
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

`cargo run -- init` writes a starter config; for a throwaway cluster, any local
Kafka works (`auth = "plaintext"`). [CLAUDE.md](CLAUDE.md) has the project's
conventions and gotchas, for people and coding agents alike.

## Before you open a pull request

**For anything bigger than a small fix, open an issue first** and wait for a
reply, so neither of us spends time on a change that won't land.

Pull requests I'll close:

- New dependencies that weren't agreed in an issue first.
- Changes that make a destructive action (delete, add partitions) easier on a
  `prod = true` environment.
- Unrelated reformatting or refactors bundled with a fix.

Commit titles follow [Conventional Commits](https://www.conventionalcommits.org)
(`fix: …`, `feat: …`) - the changelog and version bumps are generated from them.
