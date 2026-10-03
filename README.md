# kitz

> your Kafka desk clerk

A terminal UI for **Kafka** - topics, messages and consumer lag at a glance,
across all your environments. Built for **AWS MSK**: kitz signs in with your
`~/.aws` credentials (IAM, SASL OAUTHBEARER) - no cert or JAAS juggling - and
works just as well with any Kafka over plaintext or TLS.

[![test](https://github.com/Harry-kp/kitz/actions/workflows/test.yml/badge.svg)](https://github.com/Harry-kp/kitz/actions/workflows/test.yml)
[![crates.io](https://img.shields.io/crates/v/kitz.svg)](https://crates.io/crates/kitz)
[![license](https://img.shields.io/crates/l/kitz.svg)](./LICENSE)

![kitz demo](https://raw.githubusercontent.com/Harry-kp/kitz/main/assets/demo.gif)

## Features

- **IAM-native auth** - connects to MSK with your AWS credentials; plaintext and
  plain-TLS clusters supported too (`auth = "iam" | "tls" | "plaintext"`).
- **Environment hot-switch** - `1`–`9` to jump between stag / preprod / prod /
  regression without restarting. Prod is tagged red with a delete guardrail.
- **Topics** - pick a topic and everything loads by itself: message count,
  partitions, replication, live msg/s, retention and limits in plain units,
  which consumer groups read it and how far behind they are, per-partition
  offsets. Under-replicated partitions are flagged.
- **Consumer lag** - every group (idle ones too) with its total lag; the group
  view breaks it down per partition and warns when nothing is consuming.
- **Messages** - `↵` on a topic shows the latest messages, newest first, with
  pretty-printed JSON (key order kept); copy payload or key.
- **Admin** - create topic, add partitions, delete topic/group, with typed
  confirmation on prod for anything irreversible.
- **`kitz doctor <env>`** - layer-by-layer connectivity diagnosis.

## Install

**Prebuilt binary (recommended) - no dependencies, nothing to compile:**

```sh
# npm
npm install -g @harry-kp/kitz

# Homebrew
brew install Harry-kp/tap/kitz

# curl
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Harry-kp/kitz/releases/latest/download/kitz-installer.sh | sh

# cargo, prebuilt
cargo binstall kitz
```

Prebuilt for **macOS** (Apple Silicon and Intel) and **Linux x86_64**. The
Linux binary is fully static, so it runs on any distro - including the Amazon
Linux bastion next to your MSK cluster. librdkafka and OpenSSL are baked in;
there's nothing else to install. (Windows and Linux ARM aren't built yet.)

**From source (`cargo install`)** compiles librdkafka, so it needs `cmake` +
Xcode Command Line Tools:

```sh
brew install cmake
cargo install kitz
```

## Quick start

```sh
kitz init          # writes a commented starter config to ~/.config/kitz/config.toml
$EDITOR ~/.config/kitz/config.toml   # your brokers, regions, auth
kitz               # pick an environment - or `kitz stag` to open one directly
```

A single-environment config opens straight away. Config is read from
`./kitz.toml`, then `~/.config/kitz/config.toml`, or `--config <path>`:

```toml
[[env]]
name = "stag"
bootstrap = "b-1.mycluster.xxxxx.kafka.eu-central-1.amazonaws.com:9092"
region = "eu-central-1"
auth = "plaintext"   # 9092=plaintext · 9094=tls · 9098=iam
prod = false
```

> **Can't connect?** MSK brokers live on private VPC addresses, so kitz has to
> run somewhere that can reach them (on a VPN that routes the VPC, or a bastion
> inside it). `kitz doctor <env>` tells you whether it's the network, your
> credentials or the port, and the [troubleshooting guide](docs/troubleshooting.md)
> says what to do about each.

## Keys

| Key | Action |
|---|---|
| `⇥` | switch Topics ⟷ Consumer groups |
| `↑↓` / `j` `k` | move · `g` / `End` top / bottom · `PgUp` `PgDn` scroll detail |
| `/` | filter the list · `esc` clears |
| `↵` | Topics: latest messages (`r` reload, `y`/`Y` copy) · Groups: open its topic |
| `c` / `a` / `d` | create / add partitions / delete |
| `y` | copy the selected name |
| `r` | refresh from the cluster |
| `1`–`9` / `e` | switch environment / picker |
| `x` · `L` · `?` | all actions · activity log · help |
| `q` / `ctrl-c` | quit |

## Building from source

kitz links **librdkafka** (built from source via cmake):

```sh
brew install cmake        # macOS
cargo build --release
```

## More

- [Troubleshooting](docs/troubleshooting.md) - connection problems and what `kitz doctor` output means
- [Security](SECURITY.md) - what kitz does with your credentials and cluster
- [Contributing](CONTRIBUTING.md) - build, test, and what to discuss before a PR

## License

MIT © Harry KP

---

Apache Kafka® is a registered trademark of the Apache Software Foundation.
Amazon MSK and AWS are trademarks of Amazon.com, Inc. or its affiliates. kitz is
not affiliated with or endorsed by either.
