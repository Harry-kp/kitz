# kitz

> your Kafka desk clerk

A terminal UI for **AWS MSK** with first-class **IAM auth**, multi-environment
switching, and live topic / consumer-group inspection. The wedge no other Kafka
TUI has: kitz authenticates to MSK with your `~/.aws` creds (SASL OAUTHBEARER /
SigV4) — no broker-string, cert, or JAAS juggling.

> ⚠️ **Work in progress · macOS only for now.** Linux/Windows builds are
> planned. The prebuilt macOS binary is fully self-contained (librdkafka and
> OpenSSL are baked in) — nothing to install alongside it.

[![test](https://github.com/Harry-kp/kitz/actions/workflows/test.yml/badge.svg)](https://github.com/Harry-kp/kitz/actions/workflows/test.yml)
[![crates.io](https://img.shields.io/crates/v/kitz.svg)](https://crates.io/crates/kitz)
[![license](https://img.shields.io/crates/l/kitz.svg)](./LICENSE)

## Features

- **IAM-native auth** - connects to MSK with your AWS credentials; plaintext and
  plain-TLS clusters supported too (`auth = "iam" | "tls" | "plaintext"`).
- **Environment hot-switch** - `1`–`9` to jump between stag / preprod / prod /
  regression without restarting. Prod is tagged red with a delete guardrail.
- **Bird's-eye dashboard** - Topics, live **Config**/**Detail** (flip with `f`),
  an incoming-**events graph**, and an activity **Log** - all at once.
- **Detail** - partitions, ISR/replicas, watermarks, ~event count, and the
  consumer groups actually subscribed to the topic.
- **Peek** - browse recent events with pretty-printed JSON; copy payload/key.
- **Consumer groups** - full-screen view (`G`); delete with confirmation.
- **Admin** - create topic, add partitions, delete topic/group.
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
```

These ship a self-contained macOS binary - librdkafka and OpenSSL are baked in,
so there's nothing else to install.

**From source (`cargo install`)** compiles librdkafka, so it needs `cmake` +
Xcode Command Line Tools:

```sh
brew install cmake
cargo install kitz
```

## Quick start

```sh
cp kitz.toml.example kitz.toml   # then edit: your brokers, regions, auth
export AWS_PROFILE=your-profile    # or set aws_profile per-env in the toml
kitz
```

`kitz.toml` (also read from `~/.config/kitz/config.toml`):

```toml
[[env]]
name = "stag"
bootstrap = "b-1.mycluster.xxxxx.kafka.eu-central-1.amazonaws.com:9092"
region = "eu-central-1"
auth = "plaintext"   # 9092=plaintext · 9094=tls · 9098=iam
prod = false
```

> **Note:** MSK brokers are usually private VPC IPs - run kitz somewhere that
> can route to them (on the VPC's VPN, or a bastion inside the VPC). Stuck?
> `kitz doctor <env>` tells you whether it's network, creds, or protocol.

## Keys

| Key | Action |
|---|---|
| `1`–`9` / `e` | switch environment |
| `⇥` / `h` `l` | move focus between panes |
| `↑↓` / `j` `k` | navigate · `g` jump top |
| `f` | flip Detail ⟷ Config |
| `w` | event counts + live graph |
| `p` | peek events (`y`/`Y` copy) |
| `y` | copy selected topic name |
| `/` | filter topics |
| `c` / `a` / `d` | create / add-partitions / delete topic |
| `G` | consumer groups |
| `x` | actions menu · `?` help · `q` quit |

## Building from source

kitz links **librdkafka** (built from source via cmake):

```sh
brew install cmake        # macOS
cargo build --release
```

## License

MIT © Harry KP
