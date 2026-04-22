# Kafka API Keys and Versions Guide for Iggy Compatibility

This document explains Kafka wire protocol API keys, version negotiation, and practical implementation strategy so Apache Iggy can support existing Kafka clients with predictable behavior.

## 1) Kafka Wire Protocol Basics

Each Kafka request is:

- `RequestHeader`
- `RequestBody` (schema depends on API key + version)

Request header fields include:

- `api_key`: identifies operation type (for example Produce, Fetch, Metadata)
- `api_version`: schema version for that API
- `correlation_id`: request-response correlation
- `client_id`: client identifier (optional in some versions/protocol forms)

Core point: **API versions are per API key**. `Fetch v12` and `Produce v12` are unrelated schema evolutions.

## 2) API Version Negotiation

Most clients start with `ApiVersions` (API key 18) to discover:

- which APIs are supported by the broker
- min/max version for each supported API key

The client typically chooses the highest mutually supported version.

### Why this matters for Iggy

- `ApiVersions` must be implemented early and accurately.
- Incorrect min/max advertisements cause silent negotiation failures and client instability.
- A centralized capabilities registry should feed both runtime routing and `ApiVersions` responses.

## 3) API Keys that Matter Most

Below is a practical grouping by customer workload and migration priority.

### 3.1 Producer-critical APIs

- `0` Produce
- `3` Metadata
- `10` FindCoordinator (used in some producer flows)
- `18` ApiVersions
- `22` InitProducerId (idempotence / transactions)
- `24` AddPartitionsToTxn
- `25` AddOffsetsToTxn
- `26` EndTxn
- `27` WriteTxnMarkers (broker-side/internal path relevance)
- `28` TxnOffsetCommit

If you begin with non-transactional support, advertise transaction APIs as unsupported via `ApiVersions` until implemented.

### 3.2 Consumer-critical APIs

- `1` Fetch
- `2` ListOffsets
- `3` Metadata
- `8` OffsetCommit
- `9` OffsetFetch
- `10` FindCoordinator
- `11` JoinGroup
- `12` Heartbeat
- `13` LeaveGroup
- `14` SyncGroup
- `15` DescribeGroups
- `16` ListGroups
- `18` ApiVersions

Without group coordination APIs, only manual partition assignment clients will be reliable.

### 3.3 Admin / tooling APIs (phase-based)

- `19` CreateTopics
- `20` DeleteTopics
- `21` DeleteRecords
- `32` DescribeConfigs
- `33` AlterConfigs
- `37` CreatePartitions
- `43` ElectLeaders (optional depending on architecture)
- `44` IncrementalAlterConfigs
- Newer admin visibility APIs (for modern ecosystems):
  - `60` DescribeCluster
  - `61` DescribeProducers
  - `65` DescribeTransactions

Prioritize these based on actual customer AdminClient/tool usage.

## 4) Versioning Mechanics That Commonly Break Compatibility

### 4.1 Flexible versions (KIP-482)

Newer API versions use compact encodings and tagged fields.

Impact:

- serialization/deserialization differs from classic versions
- tagged field handling must be robust and forward-compatible

Recommended path:

1. Stabilize classic versions for core APIs.
2. Add flexible versions once baseline interoperability is validated.

### 4.2 Record/message format evolution

Produce/Fetch payload compatibility depends on record batch format support. Modern clients typically use newer record batch semantics.

### 4.3 Error code fidelity

Kafka clients depend on precise error behavior:

- retriable vs fatal expectations
- coordinator and epoch-related edge cases
- correct `throttle_time_ms` where required by version schema

### 4.4 Field nullability and defaults

Frequent bugs arise from:

- wrong nullability per version
- missing default values for omitted fields
- incorrect array/string encoding mode for the selected version

## 5) Security-Related APIs

If you plan to support secured Kafka client deployments:

- `17` SaslHandshake
- `36` SaslAuthenticate
- TLS transport behavior (outside API key set but commonly expected)

If unsupported initially, ensure clear documentation and predictable failures for secured client configs.

## 6) Recommended Iggy Implementation Architecture

Use a layered protocol adapter:

1. **API router** keyed by `(api_key, api_version)`
2. **Versioned codecs** per API version (decode/encode)
3. **Semantic translation layer** Kafka semantics -> Iggy internals
4. **Error mapping layer** Iggy/internal errors -> Kafka error codes
5. **Capabilities registry** single source of truth for `ApiVersions`

This isolates protocol churn from core broker logic.

## 7) Practical Rollout Plan

### Phase 1: minimum viable Kafka compatibility

- ApiVersions
- Metadata
- Produce (selected stable versions)
- Fetch
- ListOffsets

Enables baseline produce/consume for simple clients.

### Phase 2: consumer group parity

- FindCoordinator
- JoinGroup / SyncGroup / Heartbeat / LeaveGroup
- OffsetCommit / OffsetFetch

Enables mainstream consumer group behavior and rebalancing.

### Phase 3: stronger producer guarantees

- InitProducerId
- Transaction APIs (24, 25, 26, 28; plus broker marker support path)

Enables idempotence and transactional workloads.

### Phase 4: admin ecosystem compatibility

Implement AdminClient-facing APIs according to customer demand.

## 8) How to Choose Versions to Implement First

Do not guess from broker latest versions alone. Drive decisions using real customer clients:

- Java `kafka-clients` versions
- librdkafka-based stacks (.NET/Go/C/C++)
- Python clients (`confluent-kafka`, `kafka-python`)
- Node clients (`kafkajs`, `node-rdkafka`)

Process:

1. Capture requested API versions from client handshake behavior.
2. Implement exact high-demand version bands first.
3. Expand incrementally with regression tests.

## 9) Test Strategy for Migration Confidence

Maintain both protocol and end-to-end compatibility tests:

- **Golden protocol tests**: request/response binary fixtures by API version
- **Client integration tests**: real clients perform produce/consume/group operations
- **Negative tests**: unsupported API/version combinations return correct Kafka errors
- **Upgrade tests**: verify behavior across client library upgrades

## 10) Common Pitfalls During Kafka Facade Development

- Incorrect `ApiVersions` advertisement
- Implementing Produce but under-specifying Metadata responses
- Partial group protocol support causing rebalance loops
- Returning generic internal errors instead of Kafka-specific codes
- Mixing flexible vs classic encoding rules
- Metadata semantics not matching Kafka expectations for partition leadership and topic state

## 11) Suggested First Tracking Matrix

Create and maintain a matrix with columns:

- API key
- API name
- Versions supported (min-max and explicit list)
- Flexible/classic encoding status
- Tests passing by client library/version
- Known deviations from Apache Kafka behavior

This matrix becomes your single operational source for migration readiness.
