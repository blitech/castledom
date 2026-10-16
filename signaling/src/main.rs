//! Matchbox full-mesh signaling for Cémantix-style room codes.
//!
//! Clients connect to: `ws://127.0.0.1:3536/castledom_<CODE>`

use matchbox_signaling::SignalingServer;
use std::net::Ipv4Addr;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), matchbox_signaling::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let addr = (Ipv4Addr::UNSPECIFIED, 3536);
    info!("Matchbox signaling on {addr:?}");
    info!("Rooms: ws://127.0.0.1:3536/castledom_<CODE>");

    let server = SignalingServer::full_mesh_builder(addr)
        .on_connection_request(|connection| {
            info!("Connecting: {connection:?}");
            Ok(true)
        })
        .on_id_assignment(|(socket, id)| info!("{socket} -> {id}"))
        .on_peer_connected(|id| info!("Joined: {id}"))
        .on_peer_disconnected(|id| info!("Left: {id}"))
        .cors()
        .trace()
        .build();
    server.serve().await
}
