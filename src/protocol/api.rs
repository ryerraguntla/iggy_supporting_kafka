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
        ApiVersionRange { api_key: API_KEY_API_VERSIONS, min_version: 0, max_version: 3 },
        ApiVersionRange { api_key: API_KEY_METADATA,     min_version: 0, max_version: 1 },
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
