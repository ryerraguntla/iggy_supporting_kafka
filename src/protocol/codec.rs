use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::error::{KafkaProtocolError, Result};

pub struct Decoder {
    bytes: Bytes,
}

impl Decoder {
    pub fn new(bytes: Bytes) -> Self {
        Self { bytes }
    }

    pub fn remaining(&self) -> usize {
        self.bytes.remaining()
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        self.ensure(1)?;
        Ok(self.bytes.get_i8())
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        self.ensure(2)?;
        Ok(self.bytes.get_i16())
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        self.ensure(4)?;
        Ok(self.bytes.get_i32())
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        self.ensure(8)?;
        Ok(self.bytes.get_i64())
    }

    pub fn read_nullable_string(&mut self) -> Result<Option<String>> {
        let len = self.read_i16()?;
        if len < 0 {
            return Ok(None);
        }
        let len = len as usize;
        self.ensure(len)?;
        let chunk = self.bytes.copy_to_bytes(len);
        String::from_utf8(chunk.to_vec())
            .map(Some)
            .map_err(|_| KafkaProtocolError::InvalidUtf8)
    }

    pub fn read_nullable_bytes(&mut self) -> Result<Option<Bytes>> {
        let len = self.read_i32()?;
        if len < 0 {
            return Ok(None);
        }
        let len = len as usize;
        self.ensure(len)?;
        Ok(Some(self.bytes.copy_to_bytes(len)))
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<Bytes> {
        self.ensure(len)?;
        Ok(self.bytes.copy_to_bytes(len))
    }

    fn ensure(&self, needed: usize) -> Result<()> {
        let remaining = self.bytes.remaining();
        if remaining < needed {
            return Err(KafkaProtocolError::BufferUnderflow { needed, remaining });
        }
        Ok(())
    }
}

pub struct Encoder {
    bytes: BytesMut,
}

impl Encoder {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: BytesMut::with_capacity(capacity),
        }
    }

    pub fn write_i8(&mut self, v: i8) {
        self.bytes.put_i8(v);
    }

    pub fn write_i16(&mut self, v: i16) {
        self.bytes.put_i16(v);
    }

    pub fn write_i32(&mut self, v: i32) {
        self.bytes.put_i32(v);
    }

    pub fn write_i64(&mut self, v: i64) {
        self.bytes.put_i64(v);
    }

    pub fn write_nullable_string(&mut self, v: Option<&str>) {
        match v {
            None => self.write_i16(-1),
            Some(s) => {
                self.write_i16(s.len() as i16);
                self.bytes.put_slice(s.as_bytes());
            }
        }
    }

    pub fn write_nullable_bytes(&mut self, v: Option<&[u8]>) {
        match v {
            None => self.write_i32(-1),
            Some(b) => {
                self.write_i32(b.len() as i32);
                self.bytes.put_slice(b);
            }
        }
    }

    pub fn write_bytes(&mut self, b: &[u8]) {
        self.bytes.put_slice(b);
    }

    pub fn freeze(self) -> Bytes {
        self.bytes.freeze()
    }
}
