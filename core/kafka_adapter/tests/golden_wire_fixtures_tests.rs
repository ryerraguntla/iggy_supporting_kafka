use bytes::Bytes;

use iggy_supporting_kafka::protocol::api::{handle_request, API_KEY_API_VERSIONS, API_KEY_METADATA};
use iggy_supporting_kafka::protocol::codec::Encoder;

#[test]
fn golden_apiversions_v1_response_fixture() {
    let actual = handle_request(API_KEY_API_VERSIONS, 1, Bytes::new());

    // error_code=0, api_count=2, (18,0,3), (3,0,1), throttle=0
    let expected: [u8; 22] = [
        0x00, 0x00, // error_code
        0x00, 0x00, 0x00, 0x02, // api count
        0x00, 0x12, 0x00, 0x00, 0x00, 0x03, // key 18 range
        0x00, 0x03, 0x00, 0x00, 0x00, 0x01, // key 3 range
        0x00, 0x00, 0x00, 0x00, // throttle_ms
    ];
    assert_eq!(actual.as_ref(), &expected);
}

#[test]
fn golden_metadata_v0_single_topic_response_fixture() {
    let mut request = Encoder::with_capacity(32);
    request.write_i32(1); // one topic
    let req_bytes = request.freeze();

    let actual = handle_request(API_KEY_METADATA, 0, req_bytes);

    // brokers[1]: node_id=1, host=127.0.0.1, port=9093
    // topics[1]: topic_error=3, topic_name=unknown-topic, partitions[0]
    // controller_id=1 (included by this implementation baseline)
    let expected: [u8; 52] = [
        0x00, 0x00, 0x00, 0x01, // broker count
        0x00, 0x00, 0x00, 0x01, // node id
        0x00, 0x09, // host len
        0x31, 0x32, 0x37, 0x2e, 0x30, 0x2e, 0x30, 0x2e, 0x31, // "127.0.0.1"
        0x00, 0x00, 0x23, 0x85, // port 9093
        0x00, 0x00, 0x00, 0x01, // topic count
        0x00, 0x03, // topic error code
        0x00, 0x0d, // topic name len
        0x75, 0x6e, 0x6b, 0x6e, 0x6f, 0x77, 0x6e, 0x2d, 0x74, 0x6f, 0x70, 0x69, 0x63, // unknown-topic
        0x00, 0x00, 0x00, 0x00, // partition count
        0x00, 0x00, 0x00, 0x01, // controller id
    ];
    assert_eq!(actual.as_ref(), &expected);
}
