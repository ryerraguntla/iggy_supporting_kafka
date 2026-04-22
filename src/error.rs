use thiserror::Error;

#[derive(Debug, Error)]
pub enum KafkaProtocolError {
    #[error("buffer underflow: needed {needed} bytes, remaining {remaining}")]
    BufferUnderflow { needed: usize, remaining: usize },
    #[error("invalid frame length: {0}")]
    InvalidFrameLength(i32),
    #[error("request exceeds max frame size ({max_bytes} bytes): {actual_bytes} bytes")]
    FrameTooLarge { max_bytes: usize, actual_bytes: usize },
    #[error("invalid utf8 string")]
    InvalidUtf8,
    #[error("unsupported request header version: {0}")]
    UnsupportedHeaderVersion(i16),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, KafkaProtocolError>;
