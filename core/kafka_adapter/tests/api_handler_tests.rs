use bytes::Bytes;

use iggy_supporting_kafka::protocol::api::{
    handle_request, is_supported_version, split_metadata_request_topics, supported_api_ranges, API_KEY_API_VERSIONS,
    API_KEY_METADATA, ERROR_UNSUPPORTED_VERSION,
};
use iggy_supporting_kafka::protocol::codec::Decoder;

#[test]
fn api_versions_response_contains_supported_ranges() {
    let body = handle_request(API_KEY_API_VERSIONS, 3, Bytes::new());
    let mut d = Decoder::new(body);

    let error_code = d.read_i16().unwrap();
    assert_eq!(error_code, 0);

    let count = d.read_i32().unwrap();
    assert!(count >= 2);

    let mut keys = Vec::new();
    for _ in 0..count {
        let key = d.read_i16().unwrap();
        let _min = d.read_i16().unwrap();
        let _max = d.read_i16().unwrap();
        keys.push(key);
    }
    let throttle = d.read_i32().unwrap();
    assert_eq!(throttle, 0);

    let expected: Vec<i16> = supported_api_ranges().iter().map(|r| r.api_key).collect();
    for k in expected {
        assert!(keys.contains(&k));
    }
}

#[test]
fn metadata_response_has_broker_array_and_topic_array() {
    let body = handle_request(API_KEY_METADATA, 0, Bytes::new());
    let mut d = Decoder::new(body);

    let broker_count = d.read_i32().unwrap();
    assert_eq!(broker_count, 1);

    let node_id = d.read_i32().unwrap();
    assert_eq!(node_id, 1);
    let host = d.read_nullable_string().unwrap().unwrap();
    assert_eq!(host, "127.0.0.1");
    let port = d.read_i32().unwrap();
    assert_eq!(port, 9093);

    let topic_count = d.read_i32().unwrap();
    assert_eq!(topic_count, 0);
}

#[test]
fn unsupported_version_returns_protocol_error() {
    let mut req = Vec::new();
    req.extend_from_slice(&1_i32.to_be_bytes());
    let body = handle_request(API_KEY_METADATA, 99, Bytes::from(req));
    let mut d = Decoder::new(body);
    let broker_count = d.read_i32().unwrap();
    assert_eq!(broker_count, 1);
    let _ = d.read_i32().unwrap();
    let _ = d.read_nullable_string().unwrap();
    let _ = d.read_i32().unwrap();
    let topic_count = d.read_i32().unwrap();
    assert_eq!(topic_count, 1);
    let topic_error = d.read_i16().unwrap();
    assert_eq!(topic_error, ERROR_UNSUPPORTED_VERSION);
    let topic_name = d.read_nullable_string().unwrap().unwrap();
    assert_eq!(topic_name, "unknown-topic");
    let partitions_count = d.read_i32().unwrap();
    assert_eq!(partitions_count, 0);
    let controller_id = d.read_i32().unwrap();
    assert_eq!(controller_id, 1);
}

#[test]
fn unknown_api_key_returns_error_only_payload() {
    let body = handle_request(999, 0, Bytes::new());
    let mut d = Decoder::new(body);
    assert_eq!(d.read_i16().unwrap(), ERROR_UNSUPPORTED_VERSION);
}

#[test]
fn metadata_topic_split_reads_array_count() {
    let mut raw = Vec::new();
    raw.extend_from_slice(&2_i32.to_be_bytes());
    assert_eq!(split_metadata_request_topics(Bytes::from(raw)), 2);
}

#[test]
fn version_support_table_is_applied() {
    assert!(is_supported_version(API_KEY_API_VERSIONS, 3));
    assert!(!is_supported_version(API_KEY_API_VERSIONS, 10));
    assert!(is_supported_version(API_KEY_METADATA, 1));
    assert!(!is_supported_version(API_KEY_METADATA, -1));
}
