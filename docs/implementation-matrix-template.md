# Iggy Kafka Compatibility Implementation Matrix Template

Use this file as a living rollout checklist. Fill values as implementation progresses.

## 1) Project Metadata

- Owner:
- Last updated:
- Target release:
- Scope:
  - [ ] Runtime producer/consumer compatibility
  - [ ] Consumer groups
  - [ ] Idempotence
  - [ ] Transactions
  - [ ] Admin APIs

## 2) Client Support Targets

| Client Family | Library | Version(s) Targeted | Priority (P0/P1/P2) | Status (Not Started/In Progress/Supported) | Notes |
|---|---|---|---|---|---|
| Java | kafka-clients |  |  |  |  |
| C/C++/Go/.NET | librdkafka-based |  |  |  |  |
| Python | confluent-kafka |  |  |  |  |
| Python | kafka-python |  |  |  |  |
| Node | kafkajs |  |  |  |  |
| Node | node-rdkafka |  |  |  |  |

## 3) API Key and Version Support Matrix

Legend:

- `NS` = Not started
- `IP` = In progress
- `S` = Supported
- `P` = Partial (do not advertise unless safe)

| API Key | API Name | Min Version | Max Version | Version List | Encoding (Classic/Flexible/Both) | Status | Advertised in ApiVersions | Tests Green | Notes |
|---|---|---|---|---|---|---|---|---|---|
| 18 | ApiVersions |  |  |  |  | NS | No | No |  |
| 3 | Metadata |  |  |  |  | NS | No | No |  |
| 0 | Produce |  |  |  |  | NS | No | No |  |
| 1 | Fetch |  |  |  |  | NS | No | No |  |
| 2 | ListOffsets |  |  |  |  | NS | No | No |  |
| 10 | FindCoordinator |  |  |  |  | NS | No | No |  |
| 11 | JoinGroup |  |  |  |  | NS | No | No |  |
| 14 | SyncGroup |  |  |  |  | NS | No | No |  |
| 12 | Heartbeat |  |  |  |  | NS | No | No |  |
| 13 | LeaveGroup |  |  |  |  | NS | No | No |  |
| 8 | OffsetCommit |  |  |  |  | NS | No | No |  |
| 9 | OffsetFetch |  |  |  |  | NS | No | No |  |
| 22 | InitProducerId |  |  |  |  | NS | No | No |  |
| 24 | AddPartitionsToTxn |  |  |  |  | NS | No | No |  |
| 25 | AddOffsetsToTxn |  |  |  |  | NS | No | No |  |
| 26 | EndTxn |  |  |  |  | NS | No | No |  |
| 28 | TxnOffsetCommit |  |  |  |  | NS | No | No |  |
| 19 | CreateTopics |  |  |  |  | NS | No | No |  |
| 20 | DeleteTopics |  |  |  |  | NS | No | No |  |
| 32 | DescribeConfigs |  |  |  |  | NS | No | No |  |
| 33 | AlterConfigs |  |  |  |  | NS | No | No |  |
| 44 | IncrementalAlterConfigs |  |  |  |  | NS | No | No |  |

## 4) Semantics and Error Mapping Checklist

| Area | Checklist Item | Status | Notes |
|---|---|---|---|
| Error handling | Kafka error code mapping table complete | NS |  |
| Error handling | Retriable vs fatal semantics validated | NS |  |
| Throttling | `throttle_time_ms` included where required | NS |  |
| Metadata | Topic/partition metadata parity validated | NS |  |
| Groups | Stable rebalance behavior under churn | NS |  |
| Offsets | Commit/fetch semantics parity validated | NS |  |
| Produce/Fetch | Record batch format compatibility validated | NS |  |
| Versions | Flexible/tagged fields handling validated | NS |  |

## 5) Test Harness Tracking

### 5.1 Golden Protocol Fixtures

| API | Versions Covered | Fixture Count | Decoder Tests | Encoder Tests | Negative Tests | Status |
|---|---|---|---|---|---|---|
| ApiVersions |  |  |  |  |  | NS |
| Metadata |  |  |  |  |  | NS |
| Produce |  |  |  |  |  | NS |
| Fetch |  |  |  |  |  | NS |
| ListOffsets |  |  |  |  |  | NS |

### 5.2 Real Client Integration Matrix

| Client | Version | Scenario | Result | Last Run | Notes |
|---|---|---|---|---|---|
| Java kafka-clients |  | Produce/Consume |  |  |  |
| Java kafka-clients |  | Group rebalance |  |  |  |
| librdkafka |  | Produce retry semantics |  |  |  |
| librdkafka |  | Group stability |  |  |  |
| confluent-kafka |  | Produce/Consume |  |  |  |
| kafka-python |  | Produce/Consume |  |  |  |
| kafkajs |  | Produce/Consume |  |  |  |
| kafkajs |  | Group + offsets |  |  |  |

## 6) Rollout Gates

| Gate | Criteria | Status | Owner | Notes |
|---|---|---|---|---|
| Gate 1 | Baseline APIs (18,3,0,1,2) supported and tested | NS |  |  |
| Gate 2 | Consumer group APIs stable in soak tests | NS |  |  |
| Gate 3 | Idempotence/transactions validated (if in scope) | NS |  |  |
| Gate 4 | Admin API minimum set validated (if in scope) | NS |  |  |
| Gate 5 | `ApiVersions` advertisement audit complete | NS |  |  |
| Gate 6 | Migration pilot customer signoff | NS |  |  |

## 7) Open Risks and Decisions

| ID | Risk / Decision | Impact | Mitigation / Decision Needed | Owner | Due Date | Status |
|---|---|---|---|---|---|---|
| R-001 |  |  |  |  |  | Open |
| R-002 |  |  |  |  |  | Open |

## 8) Change Log

| Date | Author | Summary |
|---|---|---|
|  |  | Initial template |
