# Kafka Client API Call Lifecycle Documentation

## Overview

This document describes the complete lifecycle of API calls made by a typical Kafka client (producer and consumer) when connecting, sending/receiving messages, and closing sessions. This serves as a reference for implementing Kafka protocol compatibility in Iggy.

---

## 1. Connection Establishment Phase

### 1.1 ApiVersions (API Key 18)
**Purpose:** Client discovers which API keys and versions the broker supports

**Request Flow:**
```
Client → Server: ApiVersionsRequest (v0-v5)
  - client_software_name (v3+)
  - client_software_version (v3+)

Server → Client: ApiVersionsResponse
  - error_code (i16)
  - api_keys[] (array of supported API keys with min/max versions)
  - throttle_time_ms (v1+)
```

**Timing:** First request on every new connection

**Current Implementation Status:** ✅ Implemented in `src/protocol/api.rs`

---

### 1.2 SaslHandshake (API Key 17) - Optional
**Purpose:** Negotiate SASL authentication mechanism

**Request Flow:**
```
Client → Server: SaslHandshakeRequest (v0-v1)
  - mechanism (string: "PLAIN", "SCRAM-SHA-256", etc.)

Server → Client: SaslHandshakeResponse
  - error_code (i16)
  - mechanisms[] (array of supported mechanisms)
```

**Timing:** Before SaslAuthenticate if authentication is enabled

**Implementation Priority:** 🟡 Phase 2

---

### 1.3 SaslAuthenticate (API Key 36) - Optional
**Purpose:** Perform actual authentication

**Request Flow:**
```
Client → Server: SaslAuthenticateRequest (v0-v2)
  - auth_bytes (bytes: mechanism-specific payload)

Server → Client: SaslAuthenticateResponse
  - error_code (i16)
  - auth_bytes (bytes: server response)
  - session_lifetime_ms (v1+)
```

**Timing:** After SaslHandshake, before any data operations

**Implementation Priority:** 🟡 Phase 2

---

### 1.4 Metadata (API Key 3)
**Purpose:** Discover cluster topology, topics, and partition assignments

**Request Flow:**
```
Client → Server: MetadataRequest (v0-v13)
  - topics[] (nullable array of topic names, null = all topics)
  - allow_auto_topic_creation (v4+)
  - include_cluster_authorized_operations (v8+)
  - include_topic_authorized_operations (v8+)

Server → Client: MetadataResponse
  - throttle_time_ms (v3+)
  - brokers[] (node_id, host, port, rack)
  - cluster_id (v2+)
  - controller_id (v1+)
  - topic_metadata[] (error_code, topic_name, partitions[])
    - partitions[] (error_code, partition_index, leader_id, replica_nodes[], isr_nodes[])
```

**Timing:** After ApiVersions, before any produce/fetch operations

**Current Implementation Status:** ✅ Partial (returns stub data)

---

## 2. Producer Lifecycle (Send Messages)

### 2.1 InitProducerId (API Key 22) - For Transactions/Idempotence
**Purpose:** Get a producer ID for idempotent or transactional producers

**Request Flow:**
```
Client → Server: InitProducerIdRequest (v0-v6)
  - transactional_id (nullable string)
  - transaction_timeout_ms (i32)
  - producer_id (v3+)
  - producer_epoch (v3+)

Server → Client: InitProducerIdResponse
  - throttle_time_ms (v0+)
  - error_code (i16)
  - producer_id (i64)
  - producer_epoch (i16)
```

**Timing:** Once per producer initialization (before first Produce)

**Implementation Priority:** 🔵 Phase 2 (transactional support)

---

### 2.2 Produce (API Key 0) ⭐ CRITICAL
**Purpose:** Send records to topic partitions

**Request Flow:**
```
Client → Server: ProduceRequest (v3-v13)
  - transactional_id (v3+, nullable)
  - acks (i16: -1=all, 0=none, 1=leader)
  - timeout_ms (i32)
  - topic_data[] (array per topic)
    - topic (string)
    - partition_data[] (array per partition)
      - partition (i32)
      - records (bytes: RecordBatch format, see below)

Server → Client: ProduceResponse
  - responses[] (per topic)
    - topic (string)
    - partition_responses[] (per partition)
      - partition (i32)
      - error_code (i16)
      - base_offset (i64) ⭐ KEY FIELD for Iggy compatibility
      - log_append_time_ms (v2+)
      - log_start_offset (v5+)
  - throttle_time_ms (v1+)
```

**RecordBatch Wire Format (v2, used since Kafka 0.11):**
```
baseOffset:          i64   (first offset in this batch)
batchLength:         i32   (bytes following this field)
partitionLeaderEpoch:i32
magic:               i8    (2 for v2)
crc:                 u32   (CRC32C from attributes to end)
attributes:          i16   (compression, timestamp type, transactional bits)
lastOffsetDelta:     i32   (offset delta of last record in batch)
firstTimestamp:      i64   (ms since epoch)
maxTimestamp:        i64
producerId:          i64   (-1 if non-transactional)
producerEpoch:       i16
baseSequence:        i32
records:             i32   (count)
  [Record]*
    length:          varint
    attributes:      i8
    timestampDelta:  varint
    offsetDelta:     varint
    keyLength:       varint (or -1 for null)
    key:             bytes
    valueLength:     varint
    value:           bytes
    headers:         varint count + [headerKey, headerValue]*
```

**Timing:** Core produce loop — repeated for every batch send

**Implementation Priority:** ✅ CRITICAL — Phase 1

**Iggy Mapping:**
```rust
Kafka Produce → Iggy send_messages
- Extract records from RecordBatch
- Map topic → stream/topic in Iggy
- Map partition → partition_id
- Map record.key, record.value → Iggy message payload
- Return base_offset + partition_id (per apache/iggy#3044)
```

---

## 3. Consumer Lifecycle (Receive Messages)

### 3.1 FindCoordinator (API Key 10)
**Purpose:** Find the group coordinator for consumer group operations

**Request Flow:**
```
Client → Server: FindCoordinatorRequest (v0-v6)
  - key (string: group_id or transactional_id)
  - key_type (i8: 0=group, 1=transaction)

Server → Client: FindCoordinatorResponse
  - throttle_time_ms (v1+)
  - error_code (i16)
  - error_message (v1+)
  - node_id (i32)
  - host (string)
  - port (i32)
```

**Timing:** Before joining consumer group

**Implementation Priority:** 🟡 Phase 2

---

### 3.2 JoinGroup (API Key 11)
**Purpose:** Join a consumer group and elect a leader

**Request Flow:**
```
Client → Server: JoinGroupRequest (v0-v9)
  - group_id (string)
  - session_timeout_ms (i32)
  - rebalance_timeout_ms (v1+)
  - member_id (string: "" for first join)
  - group_instance_id (v5+, nullable)
  - protocol_type (string: "consumer")
  - protocols[] (array of supported assignment strategies)
    - name (string: "range", "roundrobin", "sticky", etc.)
    - metadata (bytes: ConsumerProtocolSubscription)

Server → Client: JoinGroupResponse
  - throttle_time_ms (v2+)
  - error_code (i16)
  - generation_id (i32)
  - protocol_name (string: chosen strategy)
  - leader (string: member_id of group leader)
  - member_id (string: assigned member_id)
  - members[] (only populated for leader)
    - member_id, metadata
```

**Timing:** Once per consumer group join/rebalance

**Implementation Priority:** 🟡 Phase 2

---

### 3.3 SyncGroup (API Key 14)
**Purpose:** Receive partition assignment from group leader

**Request Flow:**
```
Client → Server: SyncGroupRequest (v0-v5)
  - group_id (string)
  - generation_id (i32)
  - member_id (string)
  - group_instance_id (v3+)
  - protocol_type (v5+)
  - protocol_name (v5+)
  - assignments[] (only leader sends this)
    - member_id
    - assignment (bytes: ConsumerProtocolAssignment)

Server → Client: SyncGroupResponse
  - throttle_time_ms (v1+)
  - error_code (i16)
  - protocol_type (v5+)
  - protocol_name (v5+)
  - assignment (bytes: topic-partition assignments)
```

**Timing:** After JoinGroup, before fetching

**Implementation Priority:** 🟡 Phase 2

---

### 3.4 Heartbeat (API Key 12)
**Purpose:** Keep consumer group membership alive

**Request Flow:**
```
Client → Server: HeartbeatRequest (v0-v4)
  - group_id (string)
  - generation_id (i32)
  - member_id (string)
  - group_instance_id (v3+)

Server → Client: HeartbeatResponse
  - throttle_time_ms (v1+)
  - error_code (i16)
```

**Timing:** Every few seconds (session.timeout.ms / 3)

**Implementation Priority:** 🟡 Phase 2

---

### 3.5 OffsetFetch (API Key 9)
**Purpose:** Fetch committed offsets for consumer group

**Request Flow:**
```
Client → Server: OffsetFetchRequest (v1-v10)
  - group_id (string)
  - topics[] (nullable, null = all committed)
    - topic (string)
    - partition_indexes[] (i32[])

Server → Client: OffsetFetchResponse
  - throttle_time_ms (v3+)
  - topics[]
    - topic (string)
    - partitions[]
      - partition_index (i32)
      - committed_offset (i64)
      - committed_leader_epoch (v5+)
      - metadata (nullable string)
      - error_code (i16)
  - error_code (i16, v2+)
```

**Timing:** After SyncGroup, before first Fetch

**Implementation Priority:** 🟡 Phase 2

---

### 3.6 Fetch (API Key 1) ⭐ CRITICAL
**Purpose:** Fetch records from topic partitions

**Request Flow:**
```
Client → Server: FetchRequest (v4-v18)
  - replica_id (i32: -1 for consumer, broker_id for replica)
  - max_wait_ms (i32)
  - min_bytes (i32)
  - max_bytes (i32, v3+)
  - isolation_level (i8, v4+: 0=read_uncommitted, 1=read_committed)
  - session_id (i32, v7+)
  - session_epoch (i32, v7+)
  - topics[]
    - topic (string)
    - partitions[]
      - partition (i32)
      - current_leader_epoch (i32, v9+)
      - fetch_offset (i64) ⭐ where to start reading
      - last_fetched_epoch (i32, v12+)
      - log_start_offset (i64, v5+)
      - partition_max_bytes (i32)
  - forgotten_topics_data[] (v7+)
  - rack_id (v11+)

Server → Client: FetchResponse
  - throttle_time_ms (v1+)
  - error_code (i16, v7+)
  - session_id (i32, v7+)
  - responses[]
    - topic (string)
    - partitions[]
      - partition_index (i32)
      - error_code (i16)
      - high_watermark (i64)
      - last_stable_offset (i64, v4+)
      - log_start_offset (i64, v5+)
      - aborted_transactions[] (v4+)
      - preferred_read_replica (i32, v11+)
      - records (bytes: RecordBatch format) ⭐ actual data
```

**Timing:** Poll loop — repeated continuously while consuming

**Implementation Priority:** ✅ CRITICAL — Phase 1

**Iggy Mapping:**
```rust
Kafka Fetch → Iggy poll_messages or get_messages
- Map topic → stream/topic
- Map partition → partition_id
- Map fetch_offset → consumer_offset_id
- Map partition_max_bytes → message count limit
- Convert Iggy messages back to Kafka RecordBatch format
```

---

### 3.7 ListOffsets (API Key 2)
**Purpose:** Query available offsets (earliest, latest, specific timestamp)

**Request Flow:**
```
Client → Server: ListOffsetsRequest (v1-v11)
  - replica_id (i32: -1)
  - isolation_level (i8, v2+)
  - topics[]
    - topic (string)
    - partitions[]
      - partition_index (i32)
      - timestamp (i64: -2=earliest, -1=latest, or ms epoch)
      - current_leader_epoch (i32, v4+)

Server → Client: ListOffsetsResponse
  - throttle_time_ms (v2+)
  - topics[]
    - topic (string)
    - partitions[]
      - partition_index (i32)
      - error_code (i16)
      - timestamp (i64, v1+: actual timestamp of record at offset)
      - offset (i64) ⭐ resolved offset
      - leader_epoch (i32, v1+)
```

**Timing:** Before first fetch to discover start position

**Implementation Priority:** ✅ CRITICAL — Phase 1

**Iggy Mapping:**
```rust
Kafka ListOffsets → Iggy get_consumer_offset or stats APIs
- timestamp=-2 (earliest) → offset 0
- timestamp=-1 (latest) → current_offset from partition stats
- timestamp=N → binary search or not supported (return error)
```

---

### 3.8 OffsetCommit (API Key 8)
**Purpose:** Commit consumer offsets to broker

**Request Flow:**
```
Client → Server: OffsetCommitRequest (v2-v10)
  - group_id (string)
  - generation_id (i32)
  - member_id (string)
  - group_instance_id (v7+)
  - retention_time_ms (v2-v4, deprecated)
  - topics[]
    - topic (string)
    - partitions[]
      - partition_index (i32)
      - committed_offset (i64)
      - committed_leader_epoch (i32, v6+)
      - committed_metadata (nullable string)

Server → Client: OffsetCommitResponse
  - throttle_time_ms (v3+)
  - topics[]
    - topic (string)
    - partitions[]
      - partition_index (i32)
      - error_code (i16)
```

**Timing:** Periodically during consumption (auto-commit or manual commit)

**Implementation Priority:** 🟡 Phase 2

**Iggy Mapping:**
```rust
Kafka OffsetCommit → Iggy store_consumer_offset
- Map group_id → consumer_group_id
- Map topic/partition → topic_id, partition_id
- Map committed_offset → offset
```

---

## 4. Admin/Topic Management

### 4.1 CreateTopics (API Key 19)
**Purpose:** Create new topics

**Request Flow:**
```
Client → Server: CreateTopicsRequest (v2-v7)
  - topics[]
    - name (string)
    - num_partitions (i32, -1 = use assignments)
    - replication_factor (i16, -1 = use assignments)
    - assignments[] (manual partition replica assignments)
    - configs[] (topic configuration overrides)
  - timeout_ms (i32)
  - validate_only (bool)

Server → Client: CreateTopicsResponse
  - throttle_time_ms (v2+)
  - topics[]
    - name (string)
    - error_code (i16)
    - error_message (v1+)
    - topic_id (v7+)
    - num_partitions (i32, v5+)
    - replication_factor (i16, v5+)
    - configs[] (v5+)
```

**Timing:** Admin operation, before producing/consuming

**Implementation Priority:** ✅ Phase 1

**Iggy Mapping:**
```rust
Kafka CreateTopics → Iggy create_topic
- Map name → topic_name
- Map num_partitions → partitions_count
- Iggy doesn't have replication_factor (single-node or handled differently)
```

---

### 4.2 DeleteTopics (API Key 20)
**Purpose:** Delete topics

**Request Flow:**
```
Client → Server: DeleteTopicsRequest (v1-v6)
  - topic_names[] (v0-v5) or topics[] with name+topic_id (v6+)
  - timeout_ms (i32)

Server → Client: DeleteTopicsResponse
  - throttle_time_ms (v1+)
  - responses[]
    - name (string)
    - error_code (i16)
    - error_message (v5+)
```

**Timing:** Admin operation

**Implementation Priority:** 🟡 Phase 2

---

## 5. Graceful Shutdown

### 5.1 LeaveGroup (API Key 13)
**Purpose:** Leave consumer group cleanly

**Request Flow:**
```
Client → Server: LeaveGroupRequest (v0-v5)
  - group_id (string)
  - member_id (string, v0-v2) or members[] (v3+)

Server → Client: LeaveGroupResponse
  - throttle_time_ms (v1+)
  - error_code (i16)
  - members[] (v3+)
```

**Timing:** On consumer close

**Implementation Priority:** 🟡 Phase 2

---

### 5.2 Connection Close
**Action:** TCP FIN or RST

**Timing:** After all in-flight requests complete

---

## 6. Complete Lifecycle Examples

### Producer Session
```
1. Connect TCP
2. ApiVersions
3. Metadata (discover topics/partitions)
4. [Optional] SaslHandshake → SaslAuthenticate
5. [Optional] InitProducerId (if idempotent/transactional)
6. Produce (repeated)
7. [If transactional] AddPartitionsToTxn → EndTxn
8. Close TCP
```

### Consumer Session
```
1. Connect TCP
2. ApiVersions
3. Metadata (discover brokers/topics)
4. [Optional] SaslHandshake → SaslAuthenticate
5. FindCoordinator (get group coordinator)
6. JoinGroup (join consumer group)
7. SyncGroup (receive partition assignment)
8. OffsetFetch (get last committed offsets)
9. ListOffsets (if no committed offsets, find start position)
10. Fetch (poll loop, repeated)
11. Heartbeat (background task, every few seconds)
12. OffsetCommit (periodic or manual)
13. LeaveGroup (on shutdown)
14. Close TCP
```

### Admin Session
```
1. Connect TCP
2. ApiVersions
3. [Optional] SaslHandshake → SaslAuthenticate
4. Metadata (current state)
5. CreateTopics / DeleteTopics / DescribeConfigs / etc.
6. Close TCP
```

---

## 7. Kafka to Iggy Protocol Mapping Summary

| Kafka API | Iggy SDK Equivalent | Priority | Notes |
|-----------|---------------------|----------|-------|
| ApiVersions | N/A (protocol handshake) | ✅ Done | Already implemented |
| Metadata | `get_streams()`, `get_topics()` | ✅ Done | Returns stub, needs real impl |
| Produce | `send_messages()` | ✅ P1 | Must return base_offset + partition_id |
| Fetch | `poll_messages()`, `get_messages()` | ✅ P1 | Convert Iggy messages → RecordBatch |
| ListOffsets | `get_consumer_offset()`, stats APIs | ✅ P1 | Map timestamp queries |
| CreateTopics | `create_topic()` | ✅ P1 | Ignore replication_factor |
| OffsetCommit | `store_consumer_offset()` | 🟡 P2 | Consumer group support |
| OffsetFetch | `get_consumer_offset()` | 🟡 P2 | Consumer group support |
| FindCoordinator | Return self (single-node) | 🟡 P2 | Iggy is not distributed |
| JoinGroup | Consumer group coordinator | 🟡 P2 | Requires state management |
| SyncGroup | Consumer group coordinator | 🟡 P2 | Requires state management |
| Heartbeat | Consumer group coordinator | 🟡 P2 | Requires state management |
| LeaveGroup | Consumer group coordinator | 🟡 P2 | Requires state management |
| SaslHandshake | `login()` / `authenticate()` | 🟡 P2 | If Iggy auth enabled |
| SaslAuthenticate | `login()` / `authenticate()` | 🟡 P2 | If Iggy auth enabled |
| InitProducerId | N/A | 🔵 P3 | Transactional/idempotent support |
| DeleteTopics | `delete_topic()` | 🟡 P2 | Admin API |

---

## 8. References

- **Kafka Protocol Guide:** https://kafka.apache.org/protocol.html
- **Kafka Wire Format (JSON schemas):** https://github.com/apache/kafka/tree/trunk/clients/src/main/resources/common/message
- **kafka-protocol Rust Crate:** https://crates.io/crates/kafka-protocol
- **Iggy Send Messages Enhancement:** https://github.com/apache/iggy/discussions/3044
- **kafka-tool (Message Generator):** `tools/kafka-tool/README.md`

---

**Document Version:** 1.0  
**Last Updated:** 2026-05-02  
**Maintained By:** iggy_supporting_kafka project