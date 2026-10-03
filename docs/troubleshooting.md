# Troubleshooting

Almost every kitz problem is a connectivity problem, and `kitz doctor` tells you
which layer is failing:

```sh
kitz doctor <env>
```

It checks three things in order and exits non-zero if any of them fails:

1. **TCP** - can this machine open a connection to each broker?
2. **IAM token** - can your AWS credentials sign an MSK token? (only for `auth = "iam"`)
3. **Handshake + metadata** - does the full Kafka handshake work? (skipped if step 1 failed)

Find your symptom below.

## `connection timed out` in step 1

The brokers are not reachable from this machine. kitz never got as far as
Kafka, so nothing in your kitz config can fix this. MSK brokers live on private
VPC addresses, so you need a network path into that VPC:

- **On a VPN?** Check the VPN routes the cluster's VPC range. On macOS:
  `route -n get <broker-ip>` should show your VPN interface (`utun…`), not
  `en0`. If it shows `en0`, your VPN profile doesn't cover that VPC.
- **VPN routes it, still timing out?** The cluster's security group has to
  allow inbound traffic on the broker port from the address your VPN traffic
  arrives from (often a VPN gateway IP, not your laptop's IP). Ask whoever runs
  the VPN which source IP AWS sees.
- **Mesh VPNs (NetBird, Tailscale, …)** can route a range to your laptop while
  their access policy still drops the traffic. If one cluster with a permissive
  security group also times out, it's the VPN policy - request access to the
  AWS resources in your VPN's access tool.
- **Being able to see the cluster in the AWS console proves nothing here.** The
  console talks to the public AWS API; kitz has to reach the brokers themselves.
- **No VPN route at all?** Run kitz on a bastion or EC2 instance inside the VPC.

## `connection refused` in step 1

The host is reachable but nothing listens on that port. Usually the port is
wrong for the listener the cluster has enabled - see the next section.

## Port and `auth` don't match

MSK uses a different port per authentication type. kitz warns when they
disagree (`⚠ port 9092 is MSK's plaintext port, but auth = "iam"`).

| `auth` | Port (in VPC) | Port (public access) |
|---|---|---|
| `plaintext` | 9092 | - |
| `tls` | 9094 | 9194 |
| `iam` | 9098 | 9198 |

To see which listeners your cluster has, open the MSK console → your cluster →
**View client information**, or run:

```sh
aws kafka get-bootstrap-brokers --cluster-arn <arn>
```

Each bootstrap string listed there is one enabled auth type; copy the one you
want and set the matching `auth`.

## IAM token fails in step 2

`failed to provide credentials` means your AWS credentials can't be loaded:

- `aws_profile` in the config must name a profile that exists in
  `~/.aws/config`. Remove the line to use the default credential chain.
- SSO profiles expire - run `aws sso login --profile <name>`.
- Check what identity you're using: `aws sts get-caller-identity`.

If the token works but the handshake fails with an authorization error, the
IAM principal needs `kafka-cluster:Connect` (and `DescribeTopic`,
`ReadData`, `DescribeGroup`… for what you do in kitz) on the cluster.

## Connected, but some data is missing

- **A topic's config shows `(unavailable)`** - your principal lacks
  `DescribeConfigs` on that topic. Everything else still works.
- **A consumer group shows no lag** - kitz reads committed offsets; a group
  that has never committed has none, and a principal without
  `DescribeGroup` can't read them.

## Still stuck

[Open an issue](https://github.com/Harry-kp/kitz/issues/new/choose) and paste
the output of `kitz doctor <env>` (redact broker hostnames if they're
sensitive). kitz also writes librdkafka's own log to
`~/Library/Caches/kitz/kitz.log`; the last lines there often name the exact
failure. Set `KITZ_DEBUG=1` before starting kitz for verbose logs.
