use iggy_supporting_kafka::protocol::codec::Encoder;
use iggy_supporting_kafka::protocol::header::{RequestHeader, ResponseHeader};

#[test]
fn request_header_decodes() {
    let mut enc = Encoder::with_capacity(64);
    enc.write_i16(18);
    enc.write_i16(3);
    enc.write_i32(101);
    enc.write_nullable_string(Some("kafka-cli"));
    let bytes = enc.freeze();

    let header = RequestHeader::decode(bytes, 1).expect("decode should work");
    assert_eq!(header.api_key, 18);
    assert_eq!(header.api_version, 3);
    assert_eq!(header.correlation_id, 101);
    assert_eq!(header.client_id.as_deref(), Some("kafka-cli"));
}

#[test]
fn response_header_encodes() {
    let header = ResponseHeader { correlation_id: 77 };
    let bytes = header.encode(0);
    assert_eq!(bytes.as_ref(), &[0, 0, 0, 77]);
}
