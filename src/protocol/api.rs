use bytes::{BufMut, Bytes, BytesMut};

use crate::iggy_bridge::IggyBridge;
use crate::protocol::codec::{Decoder, Encoder};
use crate::protocol::requests::{
    decode_create_topics_request, decode_fetch_request, decode_list_offsets_request,
    decode_produce_request,
};
use crate::protocol::responses::{
    encode_create_topics_response, encode_fetch_response, encode_list_offsets_response,
    encode_produce_response,
};

pub const API_KEY_PRODUCE: i16 = 0;
pub const API_KEY_FETCH: i16 = 1;
pub const API_KEY_LIST_OFFSETS: i16 = 2;
pub const API_KEY_METADATA: i16 = 3;
pub const API_KEY_OFFSET_COMMIT: i16 = 8;
pub const API_KEY_OFFSET_FETCH: i16 = 9;
pub const API_KEY_FIND_COORDINATOR: i16 = 10;
pub const API_KEY_JOIN_GROUP: i16 = 11;
pub const API_KEY_HEARTBEAT: i16 = 12;
pub const API_KEY_LEAVE_GROUP: i16 = 13;
pub const API_KEY_SYNC_GROUP: i16 = 14;
pub const API_KEY_DESCRIBE_GROUPS: i16 = 15;
pub const API_KEY_LIST_GROUPS: i16 = 16;
pub const API_KEY_SASL_HANDSHAKE: i16 = 17;
pub const API_KEY_API_VERSIONS: i16 = 18;
pub const API_KEY_CREATE_TOPICS: i16 = 19;
pub const API_KEY_DELETE_TOPICS: i16 = 20;

pub const ERROR_NONE: i16 = 0;
pub const ERROR_OFFSET_OUT_OF_RANGE: i16 = 1;
pub const ERROR_CORRUPT_MESSAGE: i16 = 2;
pub const ERROR_UNKNOWN_TOPIC_OR_PARTITION: i16 = 3;
pub const ERROR_INVALID_FETCH_SIZE: i16 = 4;
pub const ERROR_LEADER_NOT_AVAILABLE: i16 = 5;
pub const ERROR_NOT_LEADER_OR_FOLLOWER: i16 = 6;
pub const ERROR_REQUEST_TIMED_OUT: i16 = 7;
pub const ERROR_UNKNOWN_SERVER_ERROR: i16 = -1;
pub const ERROR_UNSUPPORTED_VERSION: i16 = 35;
pub const ERROR_TOPIC_ALREADY_EXISTS: i16 = 36;
pub const ERROR_INVALID_PARTITIONS: i16 = 37;
pub const ERROR_INVALID_REPLICATION_FACTOR: i16 = 38;
pub const ERROR_INVALID_REQUEST: i16 = 42;
pub const ERROR_UNSUPPORTED_FOR_MESSAGE_FORMAT: i16 = 43;

#[derive(Debug, Clone, Copy)]
pub struct ApiVersionRange {
    pub api_key: i16,
    pub min_version: i16,
    pub max_version: i16,
}

pub fn supported_api_ranges() -> Vec<ApiVersionRange> {
    vec![
        ApiVersionRange { api_key: API_KEY_PRODUCE,         min_version: 3,  max_version: 9 },
        ApiVersionRange { api_key: API_KEY_FETCH,           min_version: 4,  max_version: 12 },
        ApiVersionRange { api_key: API_KEY_LIST_OFFSETS,    min_version: 1,  max_version: 6 },
        ApiVersionRange { api_key: API_KEY_METADATA,        min_version: 0,  max_version: 9 },
        ApiVersionRange { api_key: API_KEY_API_VERSIONS,    min_version: 0,  max_version: 3 },
        ApiVersionRange { api_key: API_KEY_CREATE_TOPICS,   min_version: 2,  max_version: 5 },
    ]
}

pub fn handle_request(api_key: i16, api_version: i16, body: Bytes) -> Bytes {
    match api_key {
        API_KEY_API_VERSIONS => {
            if is_supported_version(api_key, api_version) {
                encode_api_versions_response(api_version, ERROR_NONE)
            } else {
                encode_api_versions_response(1, ERROR_UNSUPPORTED_VERSION)
            }
        }
        API_KEY_METADATA => {
            if is_supported_version(api_key, api_version) {
                encode_metadata_response(api_version, body, ERROR_NONE)
            } else {
                encode_metadata_response(0, body, ERROR_UNSUPPORTED_VERSION)
            }
        }
        API_KEY_PRODUCE => {
            if is_supported_version(api_key, api_version) {
                match decode_produce_request(api_version, body) {
                    Ok(req) => encode_produce_response(api_version, req),
                    Err(e) => {
                        tracing::error!("Failed to decode Produce request: {:?}", e);
                        encode_error_only_response(ERROR_CORRUPT_MESSAGE)
                    }
                }
            } else {
                encode_error_only_response(ERROR_UNSUPPORTED_VERSION)
            }
        }
        API_KEY_FETCH => {
            if is_supported_version(api_key, api_version) {
                match decode_fetch_request(api_version, body) {
                    Ok(req) => encode_fetch_response(api_version, req),
                    Err(e) => {
                        tracing::error!("Failed to decode Fetch request: {:?}", e);
                        encode_error_only_response(ERROR_CORRUPT_MESSAGE)
                    }
                }
            } else {
                encode_error_only_response(ERROR_UNSUPPORTED_VERSION)
            }
        }
        API_KEY_LIST_OFFSETS => {
            if is_supported_version(api_key, api_version) {
                match decode_list_offsets_request(api_version, body) {
                    Ok(req) => encode_list_offsets_response(api_version, req),
                    Err(e) => {
                        tracing::error!("Failed to decode ListOffsets request: {:?}", e);
                        encode_error_only_response(ERROR_CORRUPT_MESSAGE)
                    }
                }
            } else {
                encode_error_only_response(ERROR_UNSUPPORTED_VERSION)
            }
        }
        API_KEY_CREATE_TOPICS => {
            if is_supported_version(api_key, api_version) {
                match decode_create_topics_request(api_version, body) {
                    Ok(req) => encode_create_topics_response(api_version, req),
                    Err(e) => {
                        tracing::error!("Failed to decode CreateTopics request: {:?}", e);
                        encode_error_only_response(ERROR_CORRUPT_MESSAGE)
                    }
                }
            } else {
                encode_error_only_response(ERROR_UNSUPPORTED_VERSION)
            }
        }
        _ => encode_error_only_response(ERROR_UNSUPPORTED_VERSION),
    }
}

pub fn is_supported_version(api_key: i16, api_version: i16) -> bool {
    supported_api_ranges()
        .into_iter()
        .find(|r| r.api_key == api_key)
        .map(|r| api_version >= r.min_version && api_version <= r.max_version)
        .unwrap_or(false)
}

fn encode_api_versions_response(api_version: i16, error_code: i16) -> Bytes {
    // ApiVersions v3+ uses flexible encoding (compact arrays, tagged fields).
    // v0-2 uses the legacy fixed-width encoding.
    let flexible = api_version >= 3;
    let ranges = supported_api_ranges();
    let mut e = Encoder::with_capacity(128);

    e.write_i16(error_code);

    if flexible {
        // COMPACT_ARRAY: varint(len + 1); each entry ends with empty tagged fields.
        e.write_varint((ranges.len() + 1) as u64);
        for r in &ranges {
            e.write_i16(r.api_key);
            e.write_i16(r.min_version);
            e.write_i16(r.max_version);
            e.write_empty_tagged_fields();
        }
    } else {
        e.write_i32(ranges.len() as i32);
        for r in &ranges {
            e.write_i16(r.api_key);
            e.write_i16(r.min_version);
            e.write_i16(r.max_version);
        }
    }

    if api_version >= 1 {
        e.write_i32(0); // throttle_time_ms
    }

    if flexible {
        e.write_empty_tagged_fields(); // top-level tagged fields section
    }

    e.freeze()
}

fn encode_metadata_response(_api_version: i16, body: Bytes, top_level_error_code: i16) -> Bytes {
    // Minimal v0/v1-compatible response (non-flexible — Metadata is non-flexible below v9).
    // brokers => [node_id i32, host NULLABLE_STRING, port i32]
    // topic_metadata => [topic_error_code i16, topic NULLABLE_STRING, partitions [..]]
    // controller_id => i32  (written unconditionally for baseline compat)
    let mut e = Encoder::with_capacity(256);

    e.write_i32(1); // broker count
    e.write_i32(1); // node_id
    e.write_nullable_string(Some("127.0.0.1"));
    e.write_i32(9093);

    let topics_count = split_metadata_request_topics(body);
    e.write_i32(topics_count as i32);
    for _ in 0..topics_count {
        e.write_i16(if top_level_error_code == ERROR_NONE {
            ERROR_UNKNOWN_TOPIC_OR_PARTITION
        } else {
            top_level_error_code
        });
        e.write_nullable_string(Some("unknown-topic"));
        e.write_i32(0); // empty partitions array
    }

    e.write_i32(1); // controller_id
    e.freeze()
}

fn encode_error_only_response(error_code: i16) -> Bytes {
    let mut e = Encoder::with_capacity(2);
    e.write_i16(error_code);
    e.freeze()
}

pub fn split_metadata_request_topics(body: Bytes) -> usize {
    let mut d = Decoder::new(body);
    d.read_i32().unwrap_or_default().max(0) as usize
}

/// Async handler that routes Kafka requests through `IggyBridge` for Produce, Fetch,
/// ListOffsets, and CreateTopics.  All other API keys fall back to the stub encoder.
pub async fn handle_request_with_iggy(
    api_key: i16,
    api_version: i16,
    body: Bytes,
    bridge: &IggyBridge,
) -> Bytes {
    match api_key {
        API_KEY_PRODUCE => {
            if !is_supported_version(api_key, api_version) {
                return encode_error_only_response(ERROR_UNSUPPORTED_VERSION);
            }
            let req = match decode_produce_request(api_version, body) {
                Ok(r) => r,
                Err(_) => return encode_error_only_response(ERROR_CORRUPT_MESSAGE),
            };
            // Produce each partition's records to Iggy.
            for topic in &req.topics {
                for p in &topic.partitions {
                    if let Some(records) = p.records.clone() {
                        if let Err(e) = bridge
                            .ensure_stream_and_topic(&topic.topic, 1)
                            .await
                        {
                            tracing::error!(
                                topic = %topic.topic,
                                "iggy ensure_stream_and_topic failed: {e}"
                            );
                        }
                        if let Err(e) = bridge
                            .produce(&topic.topic, p.partition as u32, records)
                            .await
                        {
                            tracing::error!(
                                topic = %topic.topic,
                                partition = p.partition,
                                "iggy produce failed: {e}"
                            );
                        }
                    }
                }
            }
            encode_produce_response(api_version, req)
        }

        API_KEY_FETCH => {
            if !is_supported_version(api_key, api_version) {
                return encode_error_only_response(ERROR_UNSUPPORTED_VERSION);
            }
            let req = match decode_fetch_request(api_version, body) {
                Ok(r) => r,
                Err(_) => return encode_error_only_response(ERROR_CORRUPT_MESSAGE),
            };

            let flexible = api_version >= 12;
            let mut e = Encoder::with_capacity(512);

            if api_version >= 1 { e.write_i32(0); } // throttle_time_ms
            if api_version >= 7 {
                e.write_i16(ERROR_NONE); // error_code
                e.write_i32(0);          // session_id
            }

            if flexible { e.write_varint((req.topics.len() + 1) as u64); }
            else        { e.write_i32(req.topics.len() as i32); }

            for topic in &req.topics {
                if flexible { e.write_compact_nullable_string(Some(&topic.topic)); }
                else        { e.write_nullable_string(Some(&topic.topic)); }

                if flexible { e.write_varint((topic.partitions.len() + 1) as u64); }
                else        { e.write_i32(topic.partitions.len() as i32); }

                for partition in &topic.partitions {
                    let payloads = bridge
                        .fetch(
                            &topic.topic,
                            partition.partition as u32,
                            partition.fetch_offset as u64,
                            100,
                        )
                        .await
                        .unwrap_or_default();

                    let hwm = bridge
                        .high_watermark(&topic.topic, partition.partition as u32)
                        .await
                        .unwrap_or(0);

                    e.write_i32(partition.partition);
                    e.write_i16(ERROR_NONE);
                    e.write_i64(hwm as i64);                   // high_watermark
                    if api_version >= 4 { e.write_i64(hwm as i64); } // last_stable_offset
                    if api_version >= 5 { e.write_i64(0); }    // log_start_offset
                    if api_version >= 4 {
                        // aborted_transactions[] empty
                        if flexible { e.write_varint(1); } else { e.write_i32(0); }
                    }
                    if api_version >= 11 { e.write_i32(-1); }  // preferred_read_replica

                    // Concatenate all fetched payloads as the records field.
                    let records: Option<Bytes> = if payloads.is_empty() {
                        None
                    } else {
                        let total: usize = payloads.iter().map(|b| b.len()).sum();
                        let mut buf = BytesMut::with_capacity(total);
                        for b in payloads { buf.put_slice(&b); }
                        Some(buf.freeze())
                    };
                    if flexible { e.write_compact_nullable_bytes(records.as_ref().map(|b| b.as_ref())); }
                    else        { e.write_nullable_bytes(records.as_ref().map(|b| b.as_ref())); }

                    if flexible { e.write_empty_tagged_fields(); }
                }
                if flexible { e.write_empty_tagged_fields(); }
            }
            if flexible { e.write_empty_tagged_fields(); }
            e.freeze()
        }

        API_KEY_LIST_OFFSETS => {
            if !is_supported_version(api_key, api_version) {
                return encode_error_only_response(ERROR_UNSUPPORTED_VERSION);
            }
            let req = match decode_list_offsets_request(api_version, body.clone()) {
                Ok(r) => r,
                Err(_) => return encode_error_only_response(ERROR_CORRUPT_MESSAGE),
            };
            // Resolve real watermarks from Iggy, then encode.
            let flexible = api_version >= 6;
            let mut e = Encoder::with_capacity(256);
            if api_version >= 2 { e.write_i32(0); } // throttle_time_ms
            if flexible { e.write_varint((req.topics.len() + 1) as u64); }
            else        { e.write_i32(req.topics.len() as i32); }

            for topic in &req.topics {
                if flexible { e.write_compact_nullable_string(Some(&topic.topic)); }
                else        { e.write_nullable_string(Some(&topic.topic)); }
                if flexible { e.write_varint((topic.partitions.len() + 1) as u64); }
                else        { e.write_i32(topic.partitions.len() as i32); }

                for p in &topic.partitions {
                    let offset = bridge
                        .high_watermark(&topic.topic, p.partition as u32)
                        .await
                        .unwrap_or(0);

                    e.write_i32(p.partition);
                    e.write_i16(ERROR_NONE);
                    if api_version >= 1 { e.write_i64(1_700_000_000_000); } // timestamp placeholder
                    e.write_i64(offset as i64);
                    if api_version >= 4 { e.write_i32(-1); } // leader_epoch
                    if flexible { e.write_empty_tagged_fields(); }
                }
                if flexible { e.write_empty_tagged_fields(); }
            }
            if flexible { e.write_empty_tagged_fields(); }
            e.freeze()
        }

        API_KEY_CREATE_TOPICS => {
            if !is_supported_version(api_key, api_version) {
                return encode_error_only_response(ERROR_UNSUPPORTED_VERSION);
            }
            let req = match decode_create_topics_request(api_version, body) {
                Ok(r) => r,
                Err(_) => return encode_error_only_response(ERROR_CORRUPT_MESSAGE),
            };
            // Create each topic in Iggy; if validate_only skip the actual creation.
            if !req.validate_only {
                for topic in &req.topics {
                    let _ = bridge
                        .ensure_stream_and_topic(
                            &topic.name,
                            topic.num_partitions.max(1) as u32,
                        )
                        .await;
                }
            }
            encode_create_topics_response(api_version, req)
        }

        // Everything else: fall back to the stub synchronous handler.
        _ => handle_request(api_key, api_version, body),
    }
}
