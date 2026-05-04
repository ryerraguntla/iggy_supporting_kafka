//! Integration tests: Iggy server in Docker ← our Kafka proxy ← raw TCP Kafka client.
//!
//! Run with:
//!   cargo test --test iggy_integration_tests -- --ignored --nocapture
//!
//! Requires Docker to be running. The tests pull `apache/iggy:latest` automatically.

use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::sleep;

use iggy_supporting_kafka::{IggyBridge, KafkaServer, ServerConfig};

// ---------------------------------------------------------------------------
// Container helpers
// ---------------------------------------------------------------------------

/// Starts an `apache/iggy` container and waits until it accepts TCP connections.
/// Returns the container (caller must keep it alive) and the mapped host port.
async fn start_iggy_container() -> (testcontainers::ContainerAsync<GenericImage>, u16) {
    let container = GenericImage::new("apache/iggy", "latest")
        .with_exposed_port(8090.tcp())
        // The ready message is printed to stdout (inside ANSI-colored log lines).
        .with_wait_for(WaitFor::message_on_stdout("TCP server has started on:"))
        // Both vars must be set together when overriding root credentials.
        .with_env_var("IGGY_ROOT_USERNAME", "iggy")
        .with_env_var("IGGY_ROOT_PASSWORD", "iggy")
        // Bind TCP to all interfaces so Docker can forward the port to the host.
        .with_env_var("IGGY_TCP_ADDRESS", "0.0.0.0:8090")
        // io_uring requires seccomp relaxation; privileged is the portable way via testcontainers.
        .with_privileged(true)
        .with_startup_timeout(Duration::from_secs(120))
        .start()
        .await
        .expect("failed to start apache/iggy container — is Docker running?");

    let port = container
        .get_host_port_ipv4(8090)
        .await
        .expect("failed to get mapped port for iggy");

    // Allow a brief moment for the TCP listener to fully accept connections.
    sleep(Duration::from_millis(500)).await;
    (container, port)
}

/// Spawns our `KafkaServer` backed by `IggyBridge` on a random available port.
/// Returns the bound port and a shutdown sender.
async fn start_proxy(iggy_port: u16) -> (u16, broadcast::Sender<()>) {
    // Bind to port 0 to get an OS-assigned free port.
    let tmp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_port = tmp.local_addr().unwrap().port();
    drop(tmp);

    let bridge = IggyBridge::connect("127.0.0.1", iggy_port)
        .await
        .expect("IggyBridge::connect failed");

    let config = ServerConfig {
        bind_addr: format!("127.0.0.1:{proxy_port}"),
        ..Default::default()
    };

    let (tx, rx) = broadcast::channel::<()>(1);
    tokio::spawn(async move {
        KafkaServer::new(config)
            .with_iggy(bridge)
            .run(rx)
            .await
            .ok();
    });

    // Give the server a moment to bind.
    sleep(Duration::from_millis(200)).await;
    (proxy_port, tx)
}

// ---------------------------------------------------------------------------
// Low-level Kafka frame helpers
// ---------------------------------------------------------------------------

/// Writes a length-prefixed Kafka frame and reads back the length-prefixed response.
async fn round_trip(stream: &mut TcpStream, request: &[u8]) -> Bytes {
    // Write 4-byte length prefix + frame.
    let mut frame = BytesMut::with_capacity(4 + request.len());
    frame.put_i32(request.len() as i32);
    frame.put_slice(request);
    stream.write_all(&frame).await.unwrap();

    // Read 4-byte response length.
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await.unwrap();
    let resp_len = i32::from_be_bytes(len_buf) as usize;

    let mut resp = vec![0u8; resp_len];
    stream.read_exact(&mut resp).await.unwrap();
    Bytes::from(resp)
}

/// Builds a minimal Kafka request frame (legacy v1 request header):
///   api_key(i16) | api_version(i16) | correlation_id(i32) | client_id_len(i16) | client_id bytes
fn request_header(api_key: i16, api_version: i16, correlation_id: i32) -> BytesMut {
    let client_id = b"iggy-integration-test";
    let mut buf = BytesMut::new();
    buf.put_i16(api_key);
    buf.put_i16(api_version);
    buf.put_i32(correlation_id);
    buf.put_i16(client_id.len() as i16);
    buf.put_slice(client_id);
    buf
}

// ---------------------------------------------------------------------------
// Kafka request builders
// ---------------------------------------------------------------------------

/// CreateTopics v2 (non-flexible): creates a single topic.
fn build_create_topics_request(topic_name: &str, num_partitions: i32, replication: i16) -> Bytes {
    let mut hdr = request_header(19, 2, 1);

    // Body: topics[] | timeout_ms | validate_only
    let name_bytes = topic_name.as_bytes();
    hdr.put_i32(1);                         // topics count
    hdr.put_i16(name_bytes.len() as i16);
    hdr.put_slice(name_bytes);              // topic name
    hdr.put_i32(num_partitions);
    hdr.put_i16(replication);
    hdr.put_i32(0);                         // assignments[] empty
    hdr.put_i32(0);                         // configs[] empty
    hdr.put_i32(5_000);                     // timeout_ms
    hdr.put_u8(0);                          // validate_only = false
    hdr.freeze()
}

/// Produce v3 (non-flexible): sends raw bytes as the records field for one partition.
fn build_produce_request(topic_name: &str, partition: i32, records: &[u8]) -> Bytes {
    let mut hdr = request_header(0, 3, 2);

    let name_bytes = topic_name.as_bytes();
    hdr.put_i16(-1);                        // transactional_id = null
    hdr.put_i16(1);                         // acks = 1 (leader ack)
    hdr.put_i32(5_000);                     // timeout_ms

    hdr.put_i32(1);                         // topics count
    hdr.put_i16(name_bytes.len() as i16);
    hdr.put_slice(name_bytes);
    hdr.put_i32(1);                         // partitions count
    hdr.put_i32(partition);
    hdr.put_i32(records.len() as i32);      // records length
    hdr.put_slice(records);                 // raw bytes (treated opaquely by proxy)
    hdr.freeze()
}

/// Fetch v4 (non-flexible): fetches from one partition starting at `fetch_offset`.
/// Fetch v4 per-partition wire format: partition(i32) | fetch_offset(i64) | partition_max_bytes(i32)
/// log_start_offset is v5+, current_leader_epoch is v9+, last_fetched_epoch is v12+.
fn build_fetch_request(topic_name: &str, partition: i32, fetch_offset: i64) -> Bytes {
    let mut hdr = request_header(1, 4, 3);

    let name_bytes = topic_name.as_bytes();
    hdr.put_i32(-1);                        // replica_id = -1 (consumer)
    hdr.put_i32(500);                       // max_wait_ms
    hdr.put_i32(1);                         // min_bytes
    hdr.put_i32(1_048_576);                 // max_bytes (1 MiB), v3+
    hdr.put_u8(0);                          // isolation_level = read_uncommitted, v4+

    hdr.put_i32(1);                         // topics count
    hdr.put_i16(name_bytes.len() as i16);
    hdr.put_slice(name_bytes);
    hdr.put_i32(1);                         // partitions count
    hdr.put_i32(partition);
    hdr.put_i64(fetch_offset);              // fetch_offset
    hdr.put_i32(1_048_576);                 // partition_max_bytes (no log_start_offset in v4)
    hdr.freeze()
}

/// ListOffsets v1 (non-flexible): asks for the latest offset of one partition.
fn build_list_offsets_request(topic_name: &str, partition: i32) -> Bytes {
    let mut hdr = request_header(2, 1, 4);

    let name_bytes = topic_name.as_bytes();
    hdr.put_i32(-1);                        // replica_id = -1
    hdr.put_i32(1);                         // topics count
    hdr.put_i16(name_bytes.len() as i16);
    hdr.put_slice(name_bytes);
    hdr.put_i32(1);                         // partitions count
    hdr.put_i32(partition);
    hdr.put_i64(-1);                        // timestamp = latest
    hdr.freeze()
}

// ---------------------------------------------------------------------------
// Response parsers
// ---------------------------------------------------------------------------

/// Parses CreateTopics v2 response — returns the error code for the first topic.
fn parse_create_topics_response(resp: &Bytes) -> i16 {
    // correlation_id(i32) | throttle_time_ms(i32) | topics_count(i32)
    // | topic_name_len(i16) + bytes | error_code(i16) | ...
    let mut pos = 4; // skip correlation_id
    // skip throttle_time_ms (v2+)
    pos += 4;
    // skip topics count
    pos += 4;
    // skip topic name (i16 len + bytes)
    let name_len = i16::from_be_bytes([resp[pos], resp[pos + 1]]) as usize;
    pos += 2 + name_len;
    // error_code
    i16::from_be_bytes([resp[pos], resp[pos + 1]])
}

/// Parses Produce v3 response — returns the error_code for the first partition.
fn parse_produce_response(resp: &Bytes) -> i16 {
    // correlation_id(i32) | topics_count(i32) | topic_name | partitions_count(i32)
    // | partition(i32) | error_code(i16) | base_offset(i64) | ...
    let mut pos = 4; // skip correlation_id
    pos += 4;        // topics_count
    let name_len = i16::from_be_bytes([resp[pos], resp[pos + 1]]) as usize;
    pos += 2 + name_len;
    pos += 4;        // partitions_count
    pos += 4;        // partition index
    i16::from_be_bytes([resp[pos], resp[pos + 1]])
}

/// Parses Fetch v4 response — returns the raw records bytes for the first partition (may be None).
fn parse_fetch_records(resp: &Bytes) -> Option<Bytes> {
    // correlation_id(i32) | throttle_time_ms(i32) [v1+] | topics_count(i32)
    // | topic_name | partitions_count(i32)
    // | partition(i32) | error_code(i16) | high_watermark(i64)
    // | last_stable_offset(i64)[v4+] | aborted_txns[v4+] | records_len(i32)
    let mut pos = 4; // skip correlation_id
    pos += 4;        // throttle_time_ms
    pos += 4;        // topics_count
    let name_len = i16::from_be_bytes([resp[pos], resp[pos + 1]]) as usize;
    pos += 2 + name_len;
    pos += 4;        // partitions_count
    pos += 4;        // partition index
    pos += 2;        // error_code
    pos += 8;        // high_watermark
    pos += 8;        // last_stable_offset (v4+)
    // aborted_transactions[] (v4+) — count = 0 → 4 bytes
    pos += 4;
    // records: i32 len + bytes (-1 = null)
    let records_len = i32::from_be_bytes([resp[pos], resp[pos + 1], resp[pos + 2], resp[pos + 3]]);
    pos += 4;
    if records_len < 0 {
        None
    } else {
        Some(resp.slice(pos..pos + records_len as usize))
    }
}

/// Parses ListOffsets v1 response — returns the offset for the first partition.
fn parse_list_offsets_response(resp: &Bytes) -> i64 {
    // correlation_id(i32) | throttle_time_ms(i32)[v2+... skip for v1] | topics_count(i32)
    // | topic_name | partitions_count(i32)
    // | partition(i32) | error_code(i16) | timestamp(i64)[v1+] | offset(i64)
    let mut pos = 4; // skip correlation_id
    // v1 has no throttle_time_ms
    pos += 4; // topics_count
    let name_len = i16::from_be_bytes([resp[pos], resp[pos + 1]]) as usize;
    pos += 2 + name_len;
    pos += 4;  // partitions_count
    pos += 4;  // partition index
    pos += 2;  // error_code
    pos += 8;  // timestamp (v1+)
    i64::from_be_bytes([
        resp[pos], resp[pos+1], resp[pos+2], resp[pos+3],
        resp[pos+4], resp[pos+5], resp[pos+6], resp[pos+7],
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Directly exercise IggyBridge without any Kafka protocol layer to confirm the bridge works.
#[tokio::test]
#[ignore = "requires Docker"]
async fn iggy_bridge_direct_produce_fetch() {
    let (_container, iggy_port) = start_iggy_container().await;
    let bridge = IggyBridge::connect("127.0.0.1", iggy_port)
        .await
        .expect("IggyBridge::connect failed");

    let topic = "bridge-direct";
    bridge.ensure_stream_and_topic(topic, 1).await.expect("ensure failed");

    let payload = Bytes::copy_from_slice(b"direct-bridge-payload");
    bridge.produce(topic, 0, payload.clone()).await.expect("produce failed");

    sleep(Duration::from_millis(500)).await;

    let results = bridge.fetch(topic, 0, 0, 10).await.expect("fetch failed");
    assert!(!results.is_empty(), "fetch must return at least one message");
    assert_eq!(results[0], payload, "fetched payload must match produced payload");

    let hwm = bridge.high_watermark(topic, 0).await.expect("hwm failed");
    assert_eq!(hwm, 1, "high watermark must be 1 after one produce");
}

/// Full round-trip: CreateTopics → Produce → Fetch validates payload survives the trip.
#[tokio::test]
#[ignore = "requires Docker"]
async fn iggy_kafka_produce_fetch_round_trip() {
    let (_container, iggy_port) = start_iggy_container().await;
    let (proxy_port, _tx) = start_proxy(iggy_port).await;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{proxy_port}"))
        .await
        .unwrap();

    let topic = "round-trip-test";
    let payload = b"hello from kafka through iggy";

    // 1. Create topic.
    let create_req = build_create_topics_request(topic, 1, 1);
    let create_resp = round_trip(&mut stream, &create_req).await;
    let create_err = parse_create_topics_response(&create_resp);
    assert_eq!(create_err, 0, "CreateTopics must succeed (got error {create_err})");

    // 2. Produce.
    let produce_req = build_produce_request(topic, 0, payload);
    let produce_resp = round_trip(&mut stream, &produce_req).await;
    let produce_err = parse_produce_response(&produce_resp);
    assert_eq!(produce_err, 0, "Produce must succeed (got error {produce_err})");

    // Give Iggy a moment to commit the message.
    sleep(Duration::from_millis(500)).await;

    // 3. Fetch.
    let fetch_req = build_fetch_request(topic, 0, 0);
    let fetch_resp = round_trip(&mut stream, &fetch_req).await;
    let records = parse_fetch_records(&fetch_resp);
    assert!(records.is_some(), "Fetch must return non-null records");
    assert_eq!(
        records.as_deref().unwrap(),
        payload.as_slice(),
        "Fetched payload must match what was produced"
    );
}

/// ListOffsets returns 0 on a new topic; rises to 1 after a Produce.
#[tokio::test]
#[ignore = "requires Docker"]
async fn iggy_list_offsets_reflects_produce() {
    let (_container, iggy_port) = start_iggy_container().await;
    let (proxy_port, _tx) = start_proxy(iggy_port).await;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{proxy_port}"))
        .await
        .unwrap();

    let topic = "offsets-test";

    // Create topic.
    let req = build_create_topics_request(topic, 1, 1);
    round_trip(&mut stream, &req).await;

    // List offsets before any produce → 0.
    let req = build_list_offsets_request(topic, 0);
    let resp = round_trip(&mut stream, &req).await;
    let before = parse_list_offsets_response(&resp);
    assert_eq!(before, 0, "high-watermark must be 0 before any produce");

    // Produce one message.
    let req = build_produce_request(topic, 0, b"measure me");
    round_trip(&mut stream, &req).await;
    sleep(Duration::from_millis(200)).await;

    // List offsets after produce → 1.
    let req = build_list_offsets_request(topic, 0);
    let resp = round_trip(&mut stream, &req).await;
    let after = parse_list_offsets_response(&resp);
    assert_eq!(after, 1, "high-watermark must be 1 after one produce");
}

/// Produce multiple messages and verify Fetch returns them all in order.
#[tokio::test]
#[ignore = "requires Docker"]
async fn iggy_fetch_multiple_messages_ordered() {
    let (_container, iggy_port) = start_iggy_container().await;
    let (proxy_port, _tx) = start_proxy(iggy_port).await;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{proxy_port}"))
        .await
        .unwrap();

    let topic = "multi-msg-test";
    let messages: &[&[u8]] = &[b"first", b"second", b"third"];

    round_trip(&mut stream, &build_create_topics_request(topic, 1, 1)).await;

    for msg in messages {
        let req = build_produce_request(topic, 0, msg);
        let resp = round_trip(&mut stream, &req).await;
        assert_eq!(parse_produce_response(&resp), 0, "every produce must succeed");
    }

    sleep(Duration::from_millis(300)).await;

    // Fetch from offset 0, max 100.
    let fetch_req = build_fetch_request(topic, 0, 0);
    let fetch_resp = round_trip(&mut stream, &fetch_req).await;
    let records = parse_fetch_records(&fetch_resp);
    assert!(records.is_some(), "Fetch must return records");

    // The proxy concatenates payloads; verify the concatenated result contains all payloads.
    let data = records.unwrap();
    let combined: Vec<u8> = messages.iter().flat_map(|m| m.iter().copied()).collect();
    assert_eq!(data.as_ref(), combined.as_slice(), "all messages must be returned in order");
}

/// Verify that CreateTopics is idempotent (re-creating same topic returns success or already-exists).
#[tokio::test]
#[ignore = "requires Docker"]
async fn iggy_create_topic_idempotent() {
    let (_container, iggy_port) = start_iggy_container().await;
    let (proxy_port, _tx) = start_proxy(iggy_port).await;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{proxy_port}"))
        .await
        .unwrap();

    let topic = "idempotent-topic";
    let req = build_create_topics_request(topic, 1, 1);

    let r1 = parse_create_topics_response(&round_trip(&mut stream, &req).await);
    let r2 = parse_create_topics_response(&round_trip(&mut stream, &build_create_topics_request(topic, 1, 1)).await);

    // First call must succeed; second may return 0 (we ignore already-exists in the bridge).
    assert_eq!(r1, 0, "first CreateTopics must succeed");
    // Our bridge swallows the already-exists error, so the response re-uses the stub encoder
    // which always encodes success for valid partition counts.
    assert_eq!(r2, 0, "second CreateTopics (idempotent) must also succeed");
}
