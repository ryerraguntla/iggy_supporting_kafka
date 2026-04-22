use bytes::Bytes;

use crate::protocol::codec::{Decoder, Encoder};

pub const API_KEY_METADATA: i16 = 3;
pub const API_KEY_API_VERSIONS: i16 = 18;
pub const ERROR_NONE: i16 = 0;
pub const ERROR_UNSUPPORTED_VERSION: i16 = 35;
pub const ERROR_UNKNOWN_TOPIC_OR_PARTITION: i16 = 3;

#[derive(Debug, Clone, Copy)]
pub struct ApiVersionRange {
    pub api_key: i16,
    pub min_version: i16,
    pub max_version: i16,
}

pub fn supported_api_ranges() -> Vec<ApiVersionRange> {
    vec![
        ApiVersionRange {
            api_key: API_KEY_API_VERSIONS,
            min_version: 0,
            max_version: 3,
        },
        ApiVersionRange {
            api_key: API_KEY_METADATA,
            min_version: 0,
            max_version: 1,
        },
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
    // Non-flexible baseline response schema for versions <= 3:
    // error_code => i16
    // api_versions => [api_key i16, min_version i16, max_version i16]
    // throttle_time_ms => i32
    let mut e = Encoder::with_capacity(128);
    e.write_i16(error_code);
    let ranges = supported_api_ranges();
    e.write_i32(ranges.len() as i32);
    for r in ranges {
        e.write_i16(r.api_key);
        e.write_i16(r.min_version);
        e.write_i16(r.max_version);
    }

    if api_version >= 1 {
        e.write_i32(0); // throttle_time_ms
    }
    e.freeze()
}

fn encode_metadata_response(_api_version: i16, body: Bytes, top_level_error_code: i16) -> Bytes {
    // Minimal schema aligned with v0/v1 compatible baseline:
    // brokers => [node_id i32, host string, port i32]
    // topic_metadata => [topic_error_code i16, topic string, partitions [..]]
    let mut e = Encoder::with_capacity(256);

    // brokers
    e.write_i32(1);
    e.write_i32(1);
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
        e.write_i32(0); // partitions array count
    }

    // v1 includes controller_id after topic metadata
    e.write_i32(1);
    e.freeze()
}

fn encode_error_only_response(error_code: i16) -> Bytes {
    let mut e = Encoder::with_capacity(2);
    e.write_i16(error_code);
    e.freeze()
}

pub fn split_metadata_request_topics(body: Bytes) -> usize {
    // Used in tests to validate decode boundary behavior for future handlers.
    // Schema for baseline request versions begins with array length of topics.
    let mut d = Decoder::new(body);
    d.read_i32().unwrap_or_default().max(0) as usize
}
