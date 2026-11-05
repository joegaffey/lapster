mod hub;
mod protocol;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{header, HeaderName, HeaderValue};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use hub::{Hub, JoinRequest};
use protocol::{ClientMessage, ServerMessage};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(about = "Lapster relay-only multiplayer room server")]
struct Args {
    #[arg(long, default_value = "0.0.0.0:3000")]
    bind: SocketAddr,
    #[arg(long, default_value = "dist")]
    static_dir: PathBuf,
    #[arg(long, default_value_t = 4)]
    max_players: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "relay_server=info,tower_http=warn".into()),
        )
        .init();

    let args = Args::parse();
    if !args.static_dir.is_dir() {
        warn!(
            "static directory {} does not exist; serving 404s",
            args.static_dir.display()
        );
    }

    let hub = Arc::new(Hub::new(args.max_players));
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(&args.static_dir).append_index_html_on_directories(true))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-embedder-policy"),
            HeaderValue::from_static("require-corp"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-resource-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .with_state(hub);

    let listener = tokio::net::TcpListener::bind(args.bind)
        .await
        .with_context(|| format!("failed to bind {}", args.bind))?;
    info!(
        "relay server listening on http://{} (static: {}, max players: {})",
        args.bind,
        args.static_dir.display(),
        args.max_players
    );
    axum::serve(listener, app)
        .await
        .context("relay server failed")
}

async fn ws_handler(ws: WebSocketUpgrade, State(hub): State<Arc<Hub>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, hub))
}

async fn handle_socket(mut socket: WebSocket, hub: Arc<Hub>) {
    let Some(Ok(Message::Text(text))) = socket.recv().await else {
        let _ = send_json(
            &mut socket,
            &ServerMessage::Error {
                message: "expected a join message".to_string(),
            },
        )
        .await;
        return;
    };
    let join = match ClientMessage::parse(text.as_str()) {
        Ok(ClientMessage::Join {
            room,
            password,
            name,
            car,
            track,
        }) => JoinRequest {
            room,
            password,
            name,
            car,
            track,
        },
        Ok(_) => {
            let _ = send_json(
                &mut socket,
                &ServerMessage::Error {
                    message: "first message must be join".to_string(),
                },
            )
            .await;
            return;
        }
        Err(message) => {
            let _ = send_json(&mut socket, &ServerMessage::Error { message }).await;
            return;
        }
    };

    let joined = match hub.join(join) {
        Ok(joined) => joined,
        Err(error) => {
            let _ = send_json(
                &mut socket,
                &ServerMessage::Error {
                    message: error.to_string(),
                },
            )
            .await;
            return;
        }
    };
    let room = match &joined.welcome {
        ServerMessage::Welcome { room, .. } => room.clone(),
        _ => unreachable!("join always returns a welcome"),
    };
    let player_id = joined.player_id;
    let mut receiver = joined.receiver;
    info!(
        "player {player_id} joined room {room} in slot {}",
        joined.slot
    );
    if !send_json(&mut socket, &joined.welcome).await {
        hub.leave(&room, player_id);
        return;
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else {
                    break;
                };
                match message {
                    Message::Text(text) => {
                        match ClientMessage::parse(text.as_str()) {
                            Ok(ClientMessage::State { seq, tick, state }) => {
                                hub.update_state(&room, player_id, seq, tick, state);
                            }
                            Ok(ClientMessage::Ping) => {
                                if !send_json(&mut socket, &ServerMessage::Pong).await {
                                    break;
                                }
                            }
                            Ok(ClientMessage::Join { .. }) => {
                                let _ = send_json(
                                    &mut socket,
                                    &ServerMessage::Error {
                                        message: "already joined".to_string(),
                                    },
                                )
                                .await;
                            }
                            Err(message) => {
                                let _ = send_json(
                                    &mut socket,
                                    &ServerMessage::Error { message },
                                )
                                .await;
                            }
                        }
                    }
                    Message::Ping(payload) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            outgoing = receiver.recv() => {
                match outgoing {
                    Ok(message) => {
                        if socket
                            .send(Message::Text(message.as_str().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    hub.leave(&room, player_id);
    info!("player {player_id} left room {room}");
}

async fn send_json(socket: &mut WebSocket, message: &ServerMessage) -> bool {
    let Ok(text) = serde_json::to_string(message) else {
        return false;
    };
    socket.send(Message::Text(text.into())).await.is_ok()
}
