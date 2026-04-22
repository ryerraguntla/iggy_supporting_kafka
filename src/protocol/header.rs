use bytes::Bytes;

use crate::error::{KafkaProtocolError, Result};
use crate::protocol::codec::{Decoder, Encoder};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestHeader {
    pub api_key: i16,
    pub api_version: i16,
    pub correlation_id: i32,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseHeader {
    pub correlation_id: i32,
}

impl RequestHeader {
    pub fn decode(bytes: Bytes, header_version: i16) -> Result<Self> {
        if header_version != 1 && header_version != 2 {
            return Err(KafkaProtocolError::UnsupportedHeaderVersion(header_version));
        }

        let mut d = Decoder::new(bytes);
        let api_key = d.read_i16()?;
        let api_version = d.read_i16()?;
        let correlation_id = d.read_i32()?;
        let client_id = d.read_nullable_string()?;

        Ok(Self {
            api_key,
            api_version,
            correlation_id,
            client_id,
        })
    }
}

impl ResponseHeader {
    pub fn encode(&self, _header_version: i16) -> Bytes {
        let mut e = Encoder::with_capacity(4);
        e.write_i32(self.correlation_id);
        e.freeze()
    }
}
