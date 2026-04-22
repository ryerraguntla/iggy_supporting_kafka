use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{error, info, warn};

use crate::error::{KafkaProtocolError, Result};
use crate::protocol::api::handle_request;
use crate::protocol::codec::Decoder;
use crate::protocol::header::{RequestHeader, ResponseHeader};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub max_frame_size: usize,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:9093".to_string(),
            max_frame_size: 8 * 1024 * 1024,
            read_timeout: Duration::from_secs(15),
            write_timeout: Duration::from_secs(10),
        }
    }
}

pub struct KafkaServer {
    config: Arc<ServerConfig>,
}

impl KafkaServer {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub async fn run(self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("kafka listener bound on {}", self.config.bind_addr);

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    info!("kafka listener shutdown requested");
                    break;
                }
                accept_result = listener.accept() => {
                    let (stream, peer) = accept_result?;
                    let cfg = Arc::clone(&self.config);
                    tokio::spawn(async move {
                        if let Err(err) = handle_connection(stream, cfg).await {
                            warn!("connection {peer} closed with error: {err}");
                        }
                    });
                }
            }
        }
        Ok(())
    }
}

async fn handle_connection(mut stream: TcpStream, config: Arc<ServerConfig>) -> Result<()> {
    loop {
        let frame = read_frame(&mut stream, config.max_frame_size, config.read_timeout).await?;
        let mut decoder = Decoder::new(frame);
        let req = RequestHeader::decode_from(&mut decoder, 1)?;
        info!(
            api_key = req.api_key,
            api_version = req.api_version,
            correlation_id = req.correlation_id,
            client_id = req.client_id.as_deref().unwrap_or(""),
            "received kafka request header"
        );

        let body = decoder.read_bytes(decoder.remaining())?;
        let body_response = handle_request(req.api_key, req.api_version, body);
        let resp_header = ResponseHeader {
            correlation_id: req.correlation_id,
        };
        let mut payload = BytesMut::with_capacity(4 + body_response.len());
        payload.put_slice(&resp_header.encode(0));
        payload.put_slice(&body_response);

        write_frame(&mut stream, &payload, config.write_timeout).await?;
    }
}

pub async fn read_frame(
    stream: &mut TcpStream,
    max_frame_size: usize,
    read_timeout: Duration,
) -> Result<bytes::Bytes> {
    let mut len_buf = [0u8; 4];
    timeout(read_timeout, stream.read_exact(&mut len_buf))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout"))??;

    let frame_len = i32::from_be_bytes(len_buf);
    if frame_len <= 0 {
        return Err(KafkaProtocolError::InvalidFrameLength(frame_len));
    }

    let frame_len = frame_len as usize;
    if frame_len > max_frame_size {
        return Err(KafkaProtocolError::FrameTooLarge {
            max_bytes: max_frame_size,
            actual_bytes: frame_len,
        });
    }

    let mut data = vec![0u8; frame_len];
    timeout(read_timeout, stream.read_exact(&mut data))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout"))??;
    Ok(bytes::Bytes::from(data))
}

pub async fn write_frame(stream: &mut TcpStream, payload: &[u8], write_timeout: Duration) -> Result<()> {
    let len = payload.len();
    if len > i32::MAX as usize {
        return Err(KafkaProtocolError::FrameTooLarge {
            max_bytes: i32::MAX as usize,
            actual_bytes: len,
        });
    }

    let mut frame = BytesMut::with_capacity(4 + len);
    frame.put_i32(len as i32);
    frame.extend_from_slice(payload);
    timeout(write_timeout, stream.write_all(&frame))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "write timeout"))??;
    Ok(())
}

pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| error!("failed to initialize tracing: {e}"));
}
