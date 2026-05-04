# Implementation Guide: Kafka to Iggy Protocol Bridge

## Phase 1: Core Produce/Fetch (CRITICAL)

### Files Created/Updated:
1. **src/protocol/requests.rs** (NEW)
   - Decode Produce, Fetch, ListOffsets, CreateTopics requests
   - Extract fields from Kafka wire format

2. **src/protocol/responses.rs** (NEW)
   - Encode stub responses for Phase 1 APIs
   - TODO markers for Iggy SDK integration

3. **src/protocol/api.rs** (UPDATED)
   - Added API constants for all critical keys
   - Extended `handle_request()` to dispatch new decoders
   - Updated `supported_api_ranges()`

4. **src/protocol/mod.rs** (UPDATED)
   - Export new modules

### Next Steps:

#### 1. Integrate Iggy SDK
```toml
# Add to Cargo.toml
[dependencies]
iggy = { version = "0.x", features = ["client"] }
tokio = { version = "1", features = ["full"] }
```

#### 2. Create Iggy Client Manager
```rust
// src/iggy_client.rs
use iggy::client::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct IggyClientPool {
    client: Arc<RwLock<Client>>,
}

impl IggyClientPool {
    pub async fn new(server_addr: &str) -> Result<Self> {
        let client = Client::builder()
            .with_tcp_endpoint(server_addr)
            .build()?;
        client.connect().await?;
        Ok(Self {
            client: Arc::new(RwLock::new(client)),
        })
    }

    pub async fn send_messages(&self, topic: &str, partition_id: u32, messages: Vec<Message>)
        -> Result<(u64, u32)>  // (base_offset, partition_id)
    {
        let client = self.client.read().await;
        // Call Iggy send_messages per apache/iggy#3044
        // Return (base_offset, partition_id)
        todo!()
    }
}
```

#### 3. Update `encode_produce_response()` in responses.rs
```rust
pub fn encode_produce_response(
    version: i16,
    req: ProduceRequest,
    iggy: &IggyClientPool
) -> Bytes {
    // For each partition with records:
    // 1. Decode RecordBatch (see tools/kafka-tool for format)
    // 2. Call iggy.send_messages()
    // 3. Get back (base_offset, partition_id)
    // 4. Write to response
    todo!()
}
```

#### 4. Decode RecordBatch in Produce
See tools/kafka-tool/src/main.rs:218-232 for RecordBatch encoding reference.
You'll need to reverse-engineer the decode.

## Testing

Use the kafka-tool to send test messages:

```bash
# Generate Kafka protocol messages
cd tools/kafka-tool
cargo run -- generate

# Start your server
cd ../..
cargo run

# Send test messages
cd tools/kafka-tool
cargo run -- send --host 127.0.0.1:9093 --api-key 0 --version 9
cargo run -- send --host 127.0.0.1:9093 --api-key 1 --version 12
cargo run -- send --host 127.0.0.1:9093 --api-key 2 --version 6
cargo run -- send --host 127.0.0.1:9093 --api-key 19 --version 5
```

## References
- Kafka Client Lifecycle: `docs/Kafka_Client_API_Lifecycle.md`
- Iggy offset return discussion: https://github.com/apache/iggy/discussions/3044
- Kafka protocol tool: `tools/kafka-tool/README.md`