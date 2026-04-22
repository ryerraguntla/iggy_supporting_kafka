# Manual Testing Playbook (Standalone Kafka Facade Foundation)

This playbook validates the current standalone Rust Kafka facade implementation:

- listener + connection lifecycle
- frame read/write safety
- request header decode
- response header encode
- minimal API routing:
  - `ApiVersions` (18)
  - `Metadata` (3)
- unsupported API/version fallback behavior

## 1) Prerequisites

- Rust toolchain (`cargo`, `rustc`)
- Optional toolsets for manual protocol probing:
  - Apache Kafka scripts (`kafka-broker-api-versions.sh`)
  - Confluent CLI equivalent (`kafka-broker-api-versions`)
  - `kcat`

## 2) Build and unit/integration test

```bash
cargo test
```

Expected:

- all tests pass
- includes golden fixture tests for `ApiVersions` and `Metadata` response bytes

## 3) Start the standalone server

```bash
RUST_LOG=info cargo run
```

Expected startup log:

- listener bound on `127.0.0.1:9093`

## 4) Kafka script verification

### 4.1 ApiVersions handshake probe

```bash
kafka-broker-api-versions.sh --bootstrap-server 127.0.0.1:9093
```

Expected:

- tool connects successfully
- server logs show request header with `api_key=18`
- tool may still report partial incompatibility (normal at this stage), but handshake traffic is visible and server responds

### 4.2 Negative port/control check

Run same command against an unused port (for sanity):

```bash
kafka-broker-api-versions.sh --bootstrap-server 127.0.0.1:9199
```

Expected:

- immediate connection failure (confirms test sensitivity)

## 5) Confluent CLI verification

```bash
kafka-broker-api-versions --bootstrap-server 127.0.0.1:9093
```

Expected:

- same as Kafka script flow (connect + handshake request/response visibility)

## 6) kcat smoke checks

### 6.1 Metadata query

```bash
kcat -b 127.0.0.1:9093 -L
```

Expected:

- socket connectivity succeeds
- metadata output may be partial/non-final due to intentionally minimal Metadata response
- server logs include `api_key=3` for metadata path

## 7) Frame robustness checks

Use netcat or custom script to send malformed prefixes if needed. Current behavior:

- zero/negative frame length -> rejected
- oversized frame length -> rejected
- read/write timeouts -> operation aborted

These are covered in integration tests; manual checks are optional unless hardening regressions are suspected.

## 8) API-level behavior expectations (current stage)

### Supported keys

- `ApiVersions` key `18`, versions `0..=3`
- `Metadata` key `3`, versions `0..=1`

### Unsupported behavior

- unsupported version for supported API key:
  - returns API-specific payload carrying `UNSUPPORTED_VERSION` semantics
- unknown API key:
  - returns short error-only payload for compatibility baseline

## 9) Troubleshooting

- If server does not start:
  - verify no process is already using `127.0.0.1:9093`
- If scripts cannot connect:
  - verify server is running and local firewall rules
- If tests fail unexpectedly:
  - rerun with verbose output:

```bash
cargo test -- --nocapture
```

## 10) Exit criteria for this stage

You can treat this stage as complete when:

- `cargo test` is green locally
- Kafka/Confluent ApiVersions probes reach the server
- metadata probe reaches server and returns structured response
- no panics/crashes on malformed or oversized frames
