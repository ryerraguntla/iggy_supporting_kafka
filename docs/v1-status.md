# Kafka Facade V1 Status

Tracking status for Iggy Kafka compatibility V1 rollout.

Status legend:

- `NS` = Not Started
- `IP` = In Progress
- `S` = Supported
- `B` = Blocked

## 1) Current Snapshot

- Date initialized: 2026-04-22
- Overall status: `NS`
- Scope: V1 non-transactional produce/consume + stable consumer groups

## 2) Top 10 "Start Here Now" Tasks

1. [ ] Implement Kafka listener config + TCP accept loop (`NS`)
2. [ ] Add request framing and header parsing (`NS`)
3. [ ] Build primitive codec package (classic types first) (`NS`)
4. [ ] Implement API router by `(api_key, api_version)` (`NS`)
5. [ ] Add capabilities registry and wire `ApiVersions (18)` (`NS`)
6. [ ] Implement `Metadata (3)` minimal path (`NS`)
7. [ ] Implement `Produce (0)` minimal append flow (`NS`)
8. [ ] Implement `Fetch (1)` minimal read flow (`NS`)
9. [ ] Implement `ListOffsets (2)` earliest/latest (`NS`)
10. [ ] Add first golden wire fixtures for `18/3/0/1/2` (`NS`)

## 3) API Coverage Status

| API Key | API Name | Target for V1 | Status | Notes |
|---|---|---|---|---|
| 18 | ApiVersions | Yes | NS | Must be source-of-truth driven |
| 3 | Metadata | Yes | NS | Topic/partition semantics are critical |
| 0 | Produce | Yes | NS | Non-transactional path first |
| 1 | Fetch | Yes | NS | Must align with record batch expectations |
| 2 | ListOffsets | Yes | NS | Required by consumer startup flows |
| 10 | FindCoordinator | Yes | NS | Needed for group workflows |
| 11 | JoinGroup | Yes | NS | Group membership |
| 14 | SyncGroup | Yes | NS | Assignment distribution |
| 12 | Heartbeat | Yes | NS | Session liveness |
| 13 | LeaveGroup | Yes | NS | Clean departures |
| 8 | OffsetCommit | Yes | NS | Consumer progress persistence |
| 9 | OffsetFetch | Yes | NS | Restart resume behavior |
| 22 | InitProducerId | No (V2) | NS | Deferred unless needed by pilot |
| 24 | AddPartitionsToTxn | No (V2) | NS | Deferred |
| 25 | AddOffsetsToTxn | No (V2) | NS | Deferred |
| 26 | EndTxn | No (V2) | NS | Deferred |
| 28 | TxnOffsetCommit | No (V2) | NS | Deferred |

## 4) Module Status

| Module | Status | Owner | Notes |
|---|---|---|---|
| Listener + connection lifecycle | NS | TBD | Includes framing limits/timeouts |
| Protocol codecs | NS | TBD | Classic first, flexible later |
| API router + capability registry | NS | TBD | Must back ApiVersions |
| Baseline API handlers | NS | TBD | 18,3,0,1,2 |
| Consumer group handlers | NS | TBD | 10,11,14,12,13 |
| Offset APIs | NS | TBD | 8,9 |
| Kafka->Iggy semantic mapping | NS | TBD | Keep as a clean boundary layer |
| Error mapping | NS | TBD | Internal -> Kafka error codes |
| Observability | NS | TBD | Per api/version metrics |
| Test harness | NS | TBD | Golden + real client matrix |

## 5) Verification Status

| Test Layer | Status | Notes |
|---|---|---|
| Unit tests (codec primitives) | NS |  |
| Golden wire fixtures | NS | Start with 18/3/0/1/2 |
| Java integration | NS | Produce/consume/group |
| librdkafka integration | NS | Produce/consume/group |
| Secondary client smoke (kafkajs or confluent-kafka) | NS |  |
| Soak/chaos | NS | Post-integration |

## 6) Weekly Update Template

Copy this block each week:

- Week of:
- Overall status:
- APIs moved to `IP`:
- APIs moved to `S`:
- Major blockers:
- Next 5 tasks:
