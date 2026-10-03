# Changelog

All notable changes to kitz are documented here. Managed by
[release-plz](https://release-plz.dev) from conventional commits.

## [0.2.1] - 2026-10-03

### Documentation

- Add the 0.2.0 section
- Never hand-bump the version - release-plz publishes it immediately

## [0.2.0] - 2026-10-03

### Highlights

- **New layout:** two views, **Topics** and **Consumer groups** (`⇥` switches). Select something and its detail loads by itself - message count, live msg/s, retention in plain units, partitions.
- **Consumer lag** per group and per topic, including idle groups; the group view breaks it down per partition and warns when nothing is consuming.
- **Messages:** `↵` on a topic shows the latest messages, newest first (`r` reload, `y` copy).
- **Linux:** a static x86_64 binary that runs on any distro, including Amazon Linux bastions. Copy works over SSH (OSC 52).
- **Getting started:** `kitz init` writes a starter config; `kitz <env>` opens an environment directly.
- **Safer on prod:** adding partitions now needs the typed topic name, and a mistyped confirmation no longer leaks keystrokes to the dashboard.
- **Changed keys:** `w`, `f` and `z` are gone (counts load automatically; config is always shown); the log moved to `L`; `⇥` now switches views.

### Features

- Redesign around two views with self-loading details and consumer lag
- Linux-friendly copy (OSC 52) and steadier filtering

### Bug Fixes

- Parse auth into an enum so doctor and client agree
- Keep librdkafka stderr logs off the screen
- Readable connect errors
- Actionable diagnosis
- Ctrl-c quits anywhere; digits pick an env on the picker
- Tidy environment picker
- Read ~/.config/kitz on macOS; add --config
- Forms keep focus on errors; prod guard for add partitions
- Status, picker, speed, errors
- Data fidelity + polish
- `kitz -c FILE doctor ENV` parses again; friendly error without a TTY
- Bump h2 0.4.19, rustls 0.23.45 (RUSTSEC-2026-0258, RUSTSEC-2026-0285)

### Packaging

- Drop Cyrus SASL (rdkafka `sasl`); MSK IAM OAUTHBEARER comes with ssl
- Ship a static Linux x86_64 binary (musl)

### Documentation

- Add CLAUDE.md with verified commands and gotchas
- README and CLAUDE.md for the two-view UI, kitz init, kitz <env>
- Troubleshooting guide, security policy, contributing, issue forms
- Demo GIF, reproducible with tapes/ (VHS + seed script)

### Refactor

- Merge brand.rs into theme.rs
- Drop unused partition leader field and stale dead_code allow
- Ponytail audit - drop 3 deps and dead code

## [0.1.0] - 2026-07-19

- Initial release: IAM-native MSK terminal UI - environment hot-switch,
  bird's-eye dashboard (Topics / Detail ⟷ Config / events graph / logs),
  event peek with copy, consumer-group view + delete, topic admin, and
  `kitz doctor` connectivity diagnosis.
