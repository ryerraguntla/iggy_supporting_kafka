# Concrete Client Implementation Matrix and Test Plan

This document translates Kafka compatibility strategy into concrete API/version targets for specific client families, plus an actionable test harness plan.

Assumption: initial goal is reliable migration for common producer/consumer workloads before full Admin/transactions parity.

## 1) Client Families and Prioritized Support Bands

Use this as your default target unless customer telemetry indicates different versions.

### 1.1 Java (`org.apache.kafka:kafka-clients`)

Focus: broad enterprise usage, strict protocol behavior, consumer groups, idempotent producers in many deployments.

**Phase A (baseline runtime compatibility)**

- `ApiVersions` (18): implement stable versions including flexible response support where needed by modern clients
- `Metadata` (3): include topic/partition metadata semantics clients expect
- `Produce` (0): support a mid-to-modern version band first (for example v7-v9 equivalent behavior set)
- `Fetch` (1): support a modern stable band (for example v10-v12 equivalent behavior set)
- `ListOffsets` (2): support modern list offsets semantics used in consumer startup paths

**Phase B (consumer groups)**

- `FindCoordinator` (10)
- `JoinGroup` (11)
- `SyncGroup` (14)
- `Heartbeat` (12)
- `LeaveGroup` (13)
- `OffsetCommit` (8)
- `OffsetFetch` (9)

**Phase C (producer guarantees)**

- `InitProducerId` (22) for idempotence
- Transactions:
  - `AddPartitionsToTxn` (24)
  - `AddOffsetsToTxn` (25)
  - `EndTxn` (26)
  - `TxnOffsetCommit` (28)

### 1.2 librdkafka ecosystem (Confluent .NET/Go/C/C++)

Focus: heavily used in production, often requests modern API bands and depends on exact error/retry semantics.

**Phase A**

- Same baseline APIs as Java: 18, 3, 0, 1, 2
- Ensure robust `ApiVersions` negotiation and conservative feature advertising

**Phase B**

- Group APIs: 10, 11, 12, 13, 14
- Offset APIs: 8, 9

**Phase C**

- Idempotence/transactions only when Iggy semantics can match expected behavior

### 1.3 Python clients

Split by implementation:

- `confluent-kafka` (librdkafka-backed): follows librdkafka behavior profile
- `kafka-python`: may use older/simple API paths in some deployments

**Initial strategy**

- Baseline + consumer group APIs first
- Validate both manual assignment and group-managed consumer patterns

### 1.4 Node clients

- `kafkajs`: popular, often sensitive to metadata and coordinator/group behavior
- `node-rdkafka`: librdkafka-backed profile

**Initial strategy**

- Baseline APIs + group APIs
- Prioritize stable rebalance behavior and offset commit correctness

## 2) Exact API-Key-First Implementation Order

Implement in this order for maximum customer impact:

1. `18` ApiVersions
2. `3` Metadata
3. `0` Produce
4. `1` Fetch
5. `2` ListOffsets
6. `10` FindCoordinator
7. `11` JoinGroup
8. `14` SyncGroup
9. `12` Heartbeat
10. `13` LeaveGroup
11. `8` OffsetCommit
12. `9` OffsetFetch
13. `22` InitProducerId
14. `24` `25` `26` `28` transaction flow
15. Admin APIs based on customer tooling demand (`19`, `20`, `32`, `33`, `37`, `44`, ...)

## 3) Version Policy Recommendation

For each implemented API key:

- publish explicit min/max version
- support a contiguous range whenever possible
- avoid claiming versions that are only partially implemented
- include feature flags behind `ApiVersions` if semantics are incomplete

Rule: under-advertise and expand safely rather than over-advertise and break clients.

## 4) Compatibility Test Harness Plan

Use two layers:

- Golden protocol tests (wire correctness)
- Real-client compatibility tests (behavior correctness)

### 4.1 Golden Protocol Tests (binary-level)

Create fixture suites per API key/version:

- request decode fixtures: captured real request bytes -> expected parsed structure
- response encode fixtures: response object -> expected bytes
- round-trip tests where valid
- negative decode tests:
  - malformed tagged fields
  - invalid compact lengths
  - unsupported version requests

Minimum fixture set for first milestone:

- ApiVersions: 2-3 common versions
- Metadata: 2-3 common versions
- Produce: at least 2 target versions
- Fetch: at least 2 target versions
- ListOffsets: at least 2 target versions

### 4.2 Real Client Compatibility Matrix Tests

Run dockerized integration tests where each test uses an actual client library version.

**Java test scenarios**

- producer send + flush + ack verification
- consumer poll from earliest/latest
- group rebalance with 2 consumers
- offset commit and restart resume
- metadata refresh under topic update

**librdkafka test scenarios**

- high-throughput producer with retry semantics
- consumer group join/leave/heartbeat stability
- rebalance and committed offset continuity

**Python / Node test scenarios**

- basic produce/consume
- coordinator discovery
- offset commit/fetch and restart behavior

### 4.3 Conformance Assertions

For each scenario, assert:

- expected records delivered exactly once/at least once depending mode
- expected Kafka error codes returned (not generic internal failures)
- expected retry behavior for retriable errors
- no protocol decode/encode mismatches in logs
- stable group state transitions (no endless rejoin loops)

### 4.4 Negative and Chaos Tests

Include:

- unsupported API version requests -> proper error response
- coordinator unavailable transitions
- forced session timeout / heartbeat misses
- leader/metadata change simulation (as close as architecture allows)
- large batch produce/fetch boundaries

### 4.5 CI Pipeline Design

Recommended stages:

1. Unit tests for codecs and error mapping
2. Golden protocol fixtures
3. Single-client smoke matrix (fast)
4. Full multi-client compatibility matrix (nightly)
5. Transaction/idempotence suite (gated; run when feature enabled)

## 5) Suggested Initial Compatibility Targets (practical default)

If you need a concrete starting commitment for customer migration messaging:

- Java clients: baseline + consumer groups + offset APIs
- librdkafka clients: baseline + consumer groups + offset APIs
- Python/Node: same baseline and group coverage, with one stable version per major library first
- Transactions/idempotence: marked beta until semantics verified end-to-end

This provides broad migration coverage while controlling implementation risk.

## 6) Implementation Readiness Exit Criteria

Declare a client family "supported" only after:

- all target scenarios pass for at least one pinned library version
- no critical protocol incompatibility defects open
- `ApiVersions` accurately reflects actual support
- customer-like soak run passes (multi-hour produce/consume + rebalance cycles)
