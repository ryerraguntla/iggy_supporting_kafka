# iggy_supporting_kafka

Reference repository for planning and implementing Kafka protocol compatibility in Apache Iggy. The goal is to let existing Kafka clients connect to Iggy with no client-side changes.

---

## Repository layout

```
src/                            Main library + binary (standalone Kafka listener)
  main.rs                       Binary entry point – starts the TCP server
  server.rs                     Listener + connection lifecycle
  error.rs                      Shared error types
  protocol/
    codec.rs                    Binary encoder / decoder primitives
    header.rs                   Request / response header codec
    api.rs                      API dispatcher and response builders
tests/                          Integration and unit tests
  codec_tests.rs
  header_tests.rs
  api_handler_tests.rs
  golden_wire_fixtures_tests.rs
  server_integration_tests.rs
tools/kafka-tool/               CLI for generating and sending Kafka wire messages
docs/                           Design notes and implementation checklists
```

---

## Prerequisites

- **Rust** (nightly toolchain) – a `rust-toolchain.toml` at the repo root pins the
  correct channel automatically; `rustup` will download it on first use.

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## (1) Listener + Connection Lifecycle

### What it does

`src/server.rs` implements a multi-connection Kafka TCP listener:

| Concern | Implementation |
|---|---|
| Bind & accept | `TcpListener` on `127.0.0.1:9093` (configurable) |
| Per-connection task | Each accepted connection runs in its own `tokio::spawn` task |
| Framing | 4-byte big-endian length prefix read with a configurable timeout |
| Frame size guard | Rejects frames larger than `max_frame_size` (default 8 MiB) |
| Header version | Auto-detected per request from `(api_key, api_version)` |
| Clean disconnect | `UnexpectedEof` / `ConnectionReset` logged at INFO, not WARN |
| Graceful shutdown | `broadcast::Receiver<()>` – send one message to stop the listener |
| Structured logging | `tracing` with peer address on every log line |

### Build

```bash
# from repo root
cargo build
```

### Run the server

```bash
cargo run
```

Default output:
```
INFO kafka listener bound on 127.0.0.1:9093
```

Stop with **Ctrl-C**.

### Configure

Pass a custom `ServerConfig` in code, or use environment variables to control log level:

```bash
RUST_LOG=debug cargo run          # verbose per-frame logging
RUST_LOG=iggy_supporting_kafka=info cargo run
```

`ServerConfig` fields (all have defaults):

| Field | Default | Description |
|---|---|---|
| `bind_addr` | `127.0.0.1:9093` | TCP address to listen on |
| `max_frame_size` | 8 MiB | Maximum allowed request frame |
| `read_timeout` | 15 s | Per-read deadline |
| `write_timeout` | 10 s | Per-write deadline |

### Test the listener

```bash
cargo test --test server_integration_tests
```

Tests cover: valid frame round-trip, length-prefix big-endian format, zero-length frame rejection, oversized frame rejection.

---

## (2) Protocol Header + Codec Foundation

### What it does

#### Codec (`src/protocol/codec.rs`)

`Encoder` and `Decoder` wrap `bytes::BytesMut` / `bytes::Bytes` and provide:

| Method | Description |
|---|---|
| `read_u8` / `write_u8` | Single unsigned byte |
| `read_i8` / `write_i8` | Signed byte |
| `read_i16` / `write_i16` | Big-endian i16 |
| `read_i32` / `write_i32` | Big-endian i32 |
| `read_i64` / `write_i64` | Big-endian i64 |
| `read_bool` / `write_bool` | i8 ≠ 0 |
| `read_varint` / `write_varint` | Unsigned varint (7 bits/byte, LSB first) |
| `read_nullable_string` / `write_nullable_string` | i16-prefixed legacy string |
| `read_compact_nullable_string` / `write_compact_nullable_string` | Varint-prefixed compact string (flexible versions) |
| `read_nullable_bytes` / `write_nullable_bytes` | i32-prefixed legacy bytes |
| `read_compact_nullable_bytes` / `write_compact_nullable_bytes` | Varint-prefixed compact bytes (flexible versions) |
| `read_tagged_fields` / `write_empty_tagged_fields` | Tagged-fields section (flexible versions) |

All reads enforce bounds and return `KafkaProtocolError::BufferUnderflow` on short input.

#### Header (`src/protocol/header.rs`)

**Request header** supports two wire formats:

| Version | Wire format |
|---|---|
| v1 (non-flexible) | `api_key i16 \| api_version i16 \| correlation_id i32 \| client_id NULLABLE_STRING` |
| v2 (flexible) | `api_key i16 \| api_version i16 \| correlation_id i32 \| client_id COMPACT_NULLABLE_STRING \| tagged_fields` |

**Response header**:

| Version | Wire format |
|---|---|
| v0 | `correlation_id i32` |
| v1 | `correlation_id i32 \| tagged_fields (0x00)` |

**Version lookup helpers** — the server calls these to pick the right format per request:

```rust
// which header format the client will send for this API + version
request_header_version(api_key: i16, api_version: i16) -> i16

// which header format to use in the response
// (ApiVersions always returns v0 regardless of request version)
response_header_version(api_key: i16, api_version: i16) -> i16
```

The flexible-version threshold table covers all 64 API keys defined in Kafka 4.x.

#### Currently handled APIs (`src/protocol/api.rs`)

| API key | Name | Versions | Flexible from |
|---|---|---|---|
| 18 | ApiVersions | 0 – 3 | v3 |
| 3 | Metadata | 0 – 1 | v9 (not reached) |

ApiVersions v3 responses use compact array encoding (varint length) and include per-entry and top-level tagged-fields bytes, matching the Kafka wire spec.

### Build

```bash
cargo build
```

### Run all tests

```bash
cargo test
```

Expected output: **32 tests, 0 failures** across five test suites.

### Run a specific suite

```bash
cargo test --test codec_tests
cargo test --test header_tests
cargo test --test api_handler_tests
cargo test --test golden_wire_fixtures_tests
cargo test --test server_integration_tests
```

---

## Smoke-test end-to-end with kafka-tool

The `tools/kafka-tool` CLI generates and sends real Kafka wire messages.

### Build the tool

```bash
cd tools/kafka-tool
cargo build
```

### Generate binary fixtures for every API key and version

```bash
cargo run -- generate --output ./kafka_messages/
# → 248 .bin files written
```

### List supported API keys

```bash
cargo run -- list
```

### Send a single request to the running server

```bash
# Terminal 1 – start the server
cd ../..
cargo run

# Terminal 2 – send ApiVersions v3
cd tools/kafka-tool
cargo run -- send --host 127.0.0.1:9093 --api-key 18 --version 3
```

### Send all messages and verify no crashes

```bash
cargo run -- verify --host 127.0.0.1:9093
```

---

## Design notes

- `docs/kafka-api-keys-and-versions.md` – full API key / version reference
- `docs/v1-implementation-checklist-by-module.md` – ordered build checklist
- `docs/v1-status.md` – live status board
- `docs/manual-testing-playbook.md` – end-to-end testing workflow with Kafka CLI tools
