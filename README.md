> ⚠️ **Work in progress · macOS only for now.**
# franz

A terminal UI for **AWS MSK** with first-class **IAM auth**, multi-environment
switching, and live topic / consumer-group inspection. The wedge no other Kafka
TUI has: franz authenticates to MSK with your `~/.aws` creds (SASL OAUTHBEARER /
SigV4) - no broker-string, cert, or JAAS juggling.

[![test](https://github.com/Harry-kp/franz/actions/workflows/test.yml/badge.svg)](https://github.com/Harry-kp/franz/actions/workflows/test.yml)
[![crates.io](https://img.shields.io/crates/v/franz.svg)](https://crates.io/crates/franz)
[![license](https://img.shields.io/crates/l/franz.svg)](./LICENSE)

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
- **`franz doctor <env>`** - layer-by-layer connectivity diagnosis.

## Install

**Prebuilt binary (recommended) - no dependencies, nothing to compile:**

```sh
# npm
npm install -g @harry-kp/franz

# Homebrew
brew install Harry-kp/tap/franz

# curl
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Harry-kp/franz/releases/latest/download/franz-installer.sh | sh
```

These ship a self-contained macOS binary - librdkafka and OpenSSL are baked in,
so there's nothing else to install.

**From source (`cargo install`)** compiles librdkafka, so it needs `cmake` +
Xcode Command Line Tools:

```sh
brew install cmake
cargo install franz
```

## Quick start

```sh
cp franz.toml.example franz.toml   # then edit: your brokers, regions, auth
export AWS_PROFILE=your-profile    # or set aws_profile per-env in the toml
franz
```

`franz.toml` (also read from `~/.config/franz/config.toml`):

```toml
[[env]]
name = "stag"
bootstrap = "b-1.mycluster.xxxxx.kafka.eu-central-1.amazonaws.com:9092"
region = "eu-central-1"
auth = "plaintext"   # 9092=plaintext · 9094=tls · 9098=iam
prod = false
```

> **Note:** MSK brokers are usually private VPC IPs - run franz somewhere that
> can route to them (on the VPC's VPN, or a bastion inside the VPC). Stuck?
> `franz doctor <env>` tells you whether it's network, creds, or protocol.

## Keys

| Key | Action |
|---|---|
| `1`–`9` / `e` | switch environment |
| `⇥` / `h` `l` | move focus between panes |
| `↑↓` / `j` `k` | navigate · `g` jump top |
| `f` | flip Detail ⟷ Config |
| `w` | event counts + live graph |
| `p` | peek events (`y`/`Y` copy) |
| `/` | filter topics |
| `c` / `a` / `d` | create / add-partitions / delete topic |
| `G` | consumer groups |
| `x` | actions menu · `?` help · `q` quit |

## Building from source

franz links **librdkafka** (built from source via cmake):

```sh
brew install cmake        # macOS
cargo build --release
```

## License

MIT © Harry KP
