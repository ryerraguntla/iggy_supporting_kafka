use tokio::signal;
use tokio::sync::broadcast;

use iggy_supporting_kafka::server::init_tracing;
use iggy_supporting_kafka::{KafkaServer, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let config = ServerConfig::default();
    let server = KafkaServer::new(config);

    let (tx, rx) = broadcast::channel(1);
    let mut server_task = tokio::spawn(async move { server.run(rx).await });

    // Wait for either a clean shutdown signal or an early server exit (e.g. bind failure).
    tokio::select! {
        result = &mut server_task => {
            // Server exited before receiving a shutdown signal.
            return Ok(result??);
        }
        _ = signal::ctrl_c() => {
            let _ = tx.send(());
        }
    }

    server_task.await??;
    Ok(())
}
