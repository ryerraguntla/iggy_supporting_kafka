use tokio::signal;
use tokio::sync::broadcast;

use iggy_supporting_kafka::{KafkaServer, ServerConfig};
use iggy_supporting_kafka::server::init_tracing;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let config = ServerConfig::default();
    let server = KafkaServer::new(config);

    let (tx, rx) = broadcast::channel(1);
    let server_task = tokio::spawn(async move { server.run(rx).await });

    signal::ctrl_c().await?;
    let _ = tx.send(());

    server_task.await??;
    Ok(())
}
