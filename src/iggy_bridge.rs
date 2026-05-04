//! Thin async bridge between Kafka protocol handlers and the Iggy message broker.
//!
//! Mapping:
//!   Kafka topic  →  Iggy stream (same name) + Iggy topic (same name)
//!   Kafka partition (0-based)  →  Iggy partition (1-based: kafka_id + 1)
//!   Kafka records bytes  →  Iggy message payload (stored opaquely)

use bytes::Bytes;
use iggy::prelude::{
    Client, CompressionAlgorithm, Consumer, Identifier, IggyClientBuilder, IggyError, IggyExpiry,
    IggyMessage, MaxTopicSize, MessageClient, Partitioning, PolledMessages, PollingStrategy,
    StreamClient, TopicClient, UserClient, DEFAULT_ROOT_PASSWORD, DEFAULT_ROOT_USERNAME,
};
use tracing::{debug, warn};

use crate::error::{KafkaProtocolError, Result};

pub struct IggyBridge {
    client: iggy::prelude::IggyClient,
}

impl IggyBridge {
    /// Connect to an Iggy TCP server and authenticate as root.
    pub async fn connect(host: &str, port: u16) -> Result<Self> {
        let client = IggyClientBuilder::new()
            .with_tcp()
            .with_server_address(format!("{host}:{port}"))
            .build()?;
        client.connect().await?;
        client
            .login_user(DEFAULT_ROOT_USERNAME, DEFAULT_ROOT_PASSWORD)
            .await?;
        Ok(Self { client })
    }

    /// Idempotently create the Iggy stream and topic that back a Kafka topic.
    pub async fn ensure_stream_and_topic(
        &self,
        kafka_topic: &str,
        partitions: u32,
    ) -> Result<()> {
        match self.client.create_stream(kafka_topic).await {
            Ok(_) => debug!(kafka_topic, "iggy stream created"),
            Err(IggyError::StreamNameAlreadyExists(_)) => {}
            Err(e) => return Err(KafkaProtocolError::Iggy(e)),
        }

        let stream_id = Identifier::named(kafka_topic)?;
        match self
            .client
            .create_topic(
                &stream_id,
                kafka_topic,
                partitions,
                CompressionAlgorithm::None,
                None,
                IggyExpiry::NeverExpire,
                MaxTopicSize::ServerDefault,
            )
            .await
        {
            Ok(_) => debug!(kafka_topic, "iggy topic created"),
            Err(IggyError::TopicNameAlreadyExists(_, _)) => {}
            Err(e) => return Err(KafkaProtocolError::Iggy(e)),
        }

        Ok(())
    }

    /// Send a raw payload to an Iggy topic, balanced across partitions.
    /// `kafka_partition` is noted but not forwarded — iggy selects the partition via
    /// `Partitioning::balanced()` so we avoid any 0-vs-1-based partition ID confusion.
    pub async fn produce(&self, kafka_topic: &str, _kafka_partition: u32, payload: Bytes) -> Result<()> {
        let stream_id = Identifier::named(kafka_topic)?;
        let topic_id = Identifier::named(kafka_topic)?;
        let msg = IggyMessage::builder()
            .payload(payload)
            .build()
            .map_err(KafkaProtocolError::Iggy)?;
        self.client
            .send_messages(
                &stream_id,
                &topic_id,
                &Partitioning::balanced(),
                &mut [msg],
            )
            .await?;
        Ok(())
    }

    /// Poll up to `max_count` messages across all partitions of the topic starting at `offset`.
    /// Returns the raw payloads in offset order.
    pub async fn fetch(
        &self,
        kafka_topic: &str,
        _kafka_partition: u32,
        offset: u64,
        max_count: u32,
    ) -> Result<Vec<Bytes>> {
        let stream_id = Identifier::named(kafka_topic)?;
        let topic_id = Identifier::named(kafka_topic)?;
        let strategy = PollingStrategy::offset(offset);
        let polled: PolledMessages = self
            .client
            .poll_messages(
                &stream_id,
                &topic_id,
                None,  // poll across all partitions
                &Consumer::default(),
                &strategy,
                max_count,
                false,
            )
            .await?;
        Ok(polled.messages.into_iter().map(|m| m.payload).collect())
    }

    /// Return the current end-offset (number of messages) for the topic.
    /// Falls back to 0 if the topic doesn't exist or has no messages.
    pub async fn high_watermark(&self, kafka_topic: &str, _kafka_partition: u32) -> Result<u64> {
        let stream_id = Identifier::named(kafka_topic)?;
        let topic_id = Identifier::named(kafka_topic)?;

        match self
            .client
            .poll_messages(
                &stream_id,
                &topic_id,
                None,
                &Consumer::default(),
                &PollingStrategy::last(),
                1,
                false,
            )
            .await
        {
            Ok(polled) => {
                let hwm = polled
                    .messages
                    .last()
                    .map(|m| m.header.offset + 1)
                    .unwrap_or(0);
                Ok(hwm)
            }
            Err(e) => {
                warn!(kafka_topic, "high_watermark query failed: {e}");
                Ok(0)
            }
        }
    }
}
