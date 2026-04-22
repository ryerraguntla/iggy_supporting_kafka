use bytes::Bytes;

use iggy_supporting_kafka::protocol::codec::{Decoder, Encoder};

#[test]
fn codec_round_trip_primitives_and_nullable_fields() {
    let mut enc = Encoder::with_capacity(128);
    enc.write_i8(-3);
    enc.write_i16(42);
    enc.write_i32(123_456);
    enc.write_i64(9_999_999);
    enc.write_nullable_string(Some("client-a"));
    enc.write_nullable_string(None);
    enc.write_nullable_bytes(Some(&[1, 2, 3]));
    enc.write_nullable_bytes(None);
    let bytes = enc.freeze();

    let mut dec = Decoder::new(bytes);
    assert_eq!(dec.read_i8().unwrap(), -3);
    assert_eq!(dec.read_i16().unwrap(), 42);
    assert_eq!(dec.read_i32().unwrap(), 123_456);
    assert_eq!(dec.read_i64().unwrap(), 9_999_999);
    assert_eq!(dec.read_nullable_string().unwrap().as_deref(), Some("client-a"));
    assert_eq!(dec.read_nullable_string().unwrap(), None);
    assert_eq!(dec.read_nullable_bytes().unwrap().unwrap(), Bytes::from_static(&[1, 2, 3]));
    assert_eq!(dec.read_nullable_bytes().unwrap(), None);
}

#[test]
fn decoder_returns_underflow_error() {
    let mut dec = Decoder::new(Bytes::from_static(&[0x00]));
    let err = dec.read_i32().expect_err("must fail");
    assert!(err.to_string().contains("buffer underflow"));
}
