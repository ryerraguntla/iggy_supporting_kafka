# Kafka Facade V1 Implementation Checklist by Module

This checklist is execution-ordered to help you build a migration-safe Kafka facade in Iggy with minimal rework.

V1 target: non-transactional producer/consumer + stable consumer groups.

## 0) Scope Lock (do before coding)

- [ ] Freeze V1 API key scope:
  - [ ] `18` ApiVersions
  - [ ] `3` Metadata
  - [ ] `0` Produce
  - [ ] `1` Fetch
  - [ ] `2` ListOffsets
  - [ ] `10` FindCoordinator
  - [ ] `11` JoinGroup
  - [ ] `14` SyncGroup
  - [ ] `12` Heartbeat
  - [ ] `13` LeaveGroup
  - [ ] `8` OffsetCommit
  - [ ] `9` OffsetFetch
- [ ] Freeze explicit version ranges for each API key.
- [ ] Mark out-of-scope for V1:
  - [ ] `22` InitProducerId
  - [ ] Txn APIs `24/25/26/28`
  - [ ] Admin APIs unless a hard customer dependency exists

Definition of done for this phase:

- A written support contract exists and matches what `ApiVersions` will advertise.

## 1) Listener + Connection Lifecycle

Goal: robust Kafka-port networking and request/response framing.

- [ ] Add dedicated Kafka listener endpoint (host/port config).
- [ ] Implement connection accept loop with backpressure limits.
- [ ] Add per-connection read buffer and request framing logic.
- [ ] Parse Kafka request envelope safely:
  - [ ] frame length
  - [ ] request header
  - [ ] body bytes
- [ ] Add correlation-safe response write path.
- [ ] Enforce max frame size and request timeout policy.
- [ ] Add connection-level metrics:
  - [ ] active connections
  - [ ] bytes in/out
  - [ ] decode failures

Definition of done:

- Can accept bytes and return a syntactically valid Kafka error response for unsupported APIs.

## 2) Protocol Header + Codec Foundation

Goal: version-aware decode/encode primitives reusable across APIs.

- [ ] Implement request header parser by version.
- [ ] Implement response header encoder by version.
- [ ] Build primitive codec library:
  - [ ] int8/int16/int32/int64
  - [ ] strings/nullable strings
  - [ ] arrays/nullable arrays
  - [ ] bytes/nullable bytes
- [ ] Add support for compact types/tagged fields only for selected flexible versions.
- [ ] Implement strict bounds checks and malformed input handling.
- [ ] Add codec unit tests for each primitive + edge cases.

Definition of done:

- Header and primitive codecs pass deterministic binary fixtures.

## 3) API Router + Capability Registry

Goal: one dispatch point keyed by `(api_key, api_version)` with honest support reporting.

- [ ] Implement router table mapping `(api_key, api_version)` -> handler.
- [ ] Add unsupported API/version fallback with correct Kafka error.
- [ ] Implement capability registry as single source of truth:
  - [ ] supported API keys
  - [ ] min/max versions per key
  - [ ] optional feature flags
- [ ] Wire `ApiVersions (18)` to capability registry output.
- [ ] Add startup validation to detect duplicate or missing mappings.

Definition of done:

- `ApiVersions` response is generated from runtime registry, not hardcoded duplicates.

## 4) Baseline Runtime APIs (P0)

Implement in order to maximize early interoperability.

### 4.1 ApiVersions (18)

- [ ] Decode request by supported versions.
- [ ] Return exact min/max ranges from capability registry.
- [ ] Include required response fields per version.
- [ ] Verify unsupported versions fail predictably.

### 4.2 Metadata (3)

- [ ] Map Iggy stream/topic model to Kafka topic metadata.
- [ ] Map shard/partition metadata and leader semantics.
- [ ] Return broker/controller fields required by target versions.
- [ ] Validate topic-not-found and authorization-style error mapping.

### 4.3 Produce (0)

- [ ] Decode produce requests for selected versions.
- [ ] Parse record batches used by target clients.
- [ ] Map Kafka topic-partition writes to Iggy internal append.
- [ ] Return partition-level status and base offset.
- [ ] Implement ack semantics supported in V1 contract.

### 4.4 Fetch (1)

- [ ] Decode fetch request variants for selected versions.
- [ ] Map fetch offsets to Iggy read APIs.
- [ ] Encode record batches in compatible format.
- [ ] Handle empty/partial fetch responses correctly.
- [ ] Support expected timeout/wait behavior for clients.

### 4.5 ListOffsets (2)

- [ ] Implement earliest/latest lookup semantics.
- [ ] Map timestamp-style lookups as supported by selected versions.
- [ ] Return partition offset results and expected errors.

Definition of done:

- Basic produce/consume works from at least one Java client and one librdkafka-based client.

## 5) Consumer Group APIs (P1)

Goal: stable group coordination and offset persistence.

- [ ] `FindCoordinator (10)`:
  - [ ] map group id to a coordinator identity
  - [ ] return routable coordinator endpoint
- [ ] `JoinGroup (11)`:
  - [ ] member join flow
  - [ ] protocol negotiation
- [ ] `SyncGroup (14)`:
  - [ ] assignment distribution
  - [ ] generation consistency checks
- [ ] `Heartbeat (12)`:
  - [ ] session liveness updates
  - [ ] generation/member validation
- [ ] `LeaveGroup (13)`:
  - [ ] member departure and rebalance trigger
- [ ] `OffsetCommit (8)` / `OffsetFetch (9)`:
  - [ ] persist committed offsets by group/topic/partition
  - [ ] fetch committed offsets on restart
- [ ] Add anti-flap protections for rebalance storms.

Definition of done:

- Two-consumer group scenario remains stable under joins/leaves and restart cycles.

## 6) Kafka-to-Iggy Semantic Mapping Layer

Goal: isolate protocol semantics from Iggy internal APIs.

- [ ] Create mapping adapters:
  - [ ] topic <-> stream
  - [ ] partition <-> shard
  - [ ] offset translation rules
- [ ] Define ordering and durability semantics mapping for produce.
- [ ] Define fetch isolation rules for V1 (non-transactional).
- [ ] Normalize internal errors before Kafka mapping layer.
- [ ] Document known semantic deviations from Apache Kafka.

Definition of done:

- Protocol handlers never call deep Iggy internals directly; all flow through mapping interfaces.

## 7) Error Mapping and Retries

Goal: return Kafka-meaningful behavior, not generic internal failures.

- [ ] Build explicit internal-error -> Kafka-error mapping table.
- [ ] Classify retriable vs fatal errors.
- [ ] Ensure coordinator/group errors are version-correct.
- [ ] Add `throttle_time_ms` where required by version.
- [ ] Add regression tests for top 20 expected error paths.

Definition of done:

- Error behavior in integration tests matches client retry expectations.

## 8) Observability + Operability

Goal: diagnose protocol and compatibility issues quickly.

- [ ] Add structured logs per request:
  - [ ] api_key
  - [ ] api_version
  - [ ] latency
  - [ ] status/error code
- [ ] Add counters/histograms:
  - [ ] requests by api/version
  - [ ] error rates by Kafka error code
  - [ ] decode/encode failures
  - [ ] group rebalance count
- [ ] Add debug switch to log raw protocol metadata (without payload leakage).
- [ ] Build dashboard for V1 health indicators.

Definition of done:

- One dashboard can answer "which APIs/versions fail most and why" within minutes.

## 9) Test Harness Execution Plan

### 9.1 Unit + Golden Protocol (always-on CI)

- [ ] Primitive codec tests
- [ ] Request header/response header tests
- [ ] Golden binary fixtures for each V1 API key and selected versions
- [ ] Negative protocol tests:
  - [ ] malformed length
  - [ ] truncated payload
  - [ ] unknown api version

### 9.2 Integration Matrix (containerized)

- [ ] Java `kafka-clients` pinned version:
  - [ ] produce/consume
  - [ ] group rebalance
  - [ ] commit/fetch offsets
- [ ] librdkafka pinned version:
  - [ ] produce/consume
  - [ ] group rebalance
  - [ ] offset continuity after restart
- [ ] One secondary client (`kafkajs` or `confluent-kafka`) smoke coverage

### 9.3 Soak + Chaos

- [ ] 4-8 hour soak with periodic rebalances
- [ ] coordinator transient failure simulation
- [ ] metadata refresh stress during topic changes
- [ ] large batch boundary tests

Definition of done:

- CI green for unit/golden/integration; nightly soak/chaos stable with no critical regressions.

## 10) Release Readiness Gates

- [ ] Gate A: Baseline APIs green in CI and integration matrix
- [ ] Gate B: Group APIs stable in rebalance stress tests
- [ ] Gate C: `ApiVersions` audit confirms zero over-advertised versions
- [ ] Gate D: Documentation lists exact supported APIs/versions and known deviations
- [ ] Gate E: Pilot customer validation completed

## 11) Suggested Team Task Breakdown

- [ ] Engineer 1: listener, framing, router, capability registry
- [ ] Engineer 2: baseline API handlers (18,3,0,1,2)
- [ ] Engineer 3: group coordinator and offset APIs
- [ ] Engineer 4: test harness, fixtures, CI matrix, soak tests
- [ ] Shared: error mapping table + observability

## 12) First Week Execution Sprint (example)

Day 1-2:

- [ ] listener + framing + header decode
- [ ] router + capability registry
- [ ] ApiVersions end-to-end

Day 3-4:

- [ ] Metadata + Produce skeleton
- [ ] basic Fetch + ListOffsets
- [ ] first golden fixtures

Day 5:

- [ ] Java and librdkafka smoke tests
- [ ] error mapping hardening
- [ ] publish updated support contract
