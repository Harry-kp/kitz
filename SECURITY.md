# Security

kitz connects to Kafka clusters you point it at, often production ones, using
your AWS credentials. This page says what it does with them.

## What kitz does

- **Credentials.** For `auth = "iam"`, kitz signs a short-lived MSK token with
  the standard AWS credential chain (or the `aws_profile` you set). It never
  reads, stores, or prints your keys. Your config file contains no secrets.
- **Network.** kitz talks only to the brokers in your config, plus the AWS
  credential providers when using IAM. There is no telemetry, no update check,
  and no server of its own.
- **Files.** It reads `./kitz.toml` or `~/.config/kitz/config.toml`, and writes
  librdkafka's log to `~/Library/Caches/kitz/kitz.log`. Message payloads are
  shown on screen and copied to the clipboard only when you press `y`; they are
  never written to disk.
- **Writes to your cluster.** Only when you ask: create topic, add partitions,
  delete topic, delete consumer group. On an environment marked `prod = true`,
  deleting and adding partitions (both irreversible) require typing the exact
  name to confirm. Viewing messages uses a consumer that never commits offsets.

## Reporting a vulnerability

Please don't open a public issue. Use
[GitHub's private vulnerability reporting](https://github.com/Harry-kp/kitz/security/advisories/new)
with a description and steps to reproduce. You'll get a reply within a week.

Only the latest release is supported with fixes.
