# Standalone Testing with Kafka / Confluent Tools

This project currently implements:

- TCP listener with Kafka length-prefixed framing
- request header decode (`api_key`, `api_version`, `correlation_id`, `client_id`)
- response header encode (`correlation_id`)
- minimal API routing for:
  - `ApiVersions` (key 18)
  - `Metadata` (key 3)
- bounded frame size and read/write timeout controls

It does not yet implement full runtime API handlers (Produce/Fetch/etc), so producer/consumer commands will connect but not complete end-to-end semantics.

## 1) Build and test locally

```bash
cargo test
cargo run
```

Server defaults to `127.0.0.1:9093`.

## 2) Quick protocol smoke with Kafka project tools

Install Apache Kafka binaries and run this against the standalone server:

```bash
kafka-broker-api-versions.sh --bootstrap-server 127.0.0.1:9093
```

Expected behavior for this foundation stage:

- Tool reaches the socket and sends request bytes.
- Server logs decoded request header metadata.
- API-level response now includes minimal `ApiVersions` and `Metadata` payloads.

## 3) Quick protocol smoke with Confluent public CLI

If you have Confluent Platform CLI tools installed:

```bash
kafka-broker-api-versions --bootstrap-server 127.0.0.1:9093
```

Expected behavior is the same as above.

## 4) Optional raw request smoke via kcat

`kcat` can validate TCP reachability and metadata call attempts:

```bash
kcat -b 127.0.0.1:9093 -L
```

At this stage, use this for connection/protocol smoke only.

## 5) Interpreting results

Good for phase (1) and (2):

- Connection accepted by listener.
- No frame parsing panic/crash.
- Request header fields are decoded and logged.
- Response length prefix and correlation id are written.

Not expected yet:

- successful topic metadata content
- produce/fetch correctness
- consumer group workflow

## 6) Suggested next implementation checkpoint

After this foundation, implement:

1. `ApiVersions` API handler
2. `Metadata` API handler
3. corresponding integration tests with real Kafka CLI tools
