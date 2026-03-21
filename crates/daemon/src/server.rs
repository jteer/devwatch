use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use devwatch_core::ipc::{ClientMessage, DaemonMessage};
use devwatch_core::types::VcsEvent;

use crate::state::DaemonState;

pub async fn run_server(
    port: u16,
    state: Arc<Mutex<DaemonState>>,
    event_tx: broadcast::Sender<VcsEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!("IPC server listening on {addr}");

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer)) => {
                        info!(%peer, "client connected");
                        let state   = Arc::clone(&state);
                        let rx      = event_tx.subscribe();
                        let cancel  = cancel.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, state, rx, cancel).await {
                                warn!(%peer, "client error: {e}");
                            }
                            info!(%peer, "client disconnected");
                        });
                    }
                    Err(e) => error!("accept error: {e}"),
                }
            }
            _ = cancel.cancelled() => {
                info!("IPC server shutting down");
                break;
            }
        }
    }
    Ok(())
}

async fn handle_client(
    stream: TcpStream,
    state: Arc<Mutex<DaemonState>>,
    mut event_rx: broadcast::Receiver<VcsEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut subscribed = false;

    loop {
        tokio::select! {
            // Read a command from the client.
            line = lines.next_line() => {
                match line? {
                    None => break, // client disconnected
                    Some(raw) => {
                        match serde_json::from_str::<ClientMessage>(&raw) {
                            Err(e) => {
                                let msg = DaemonMessage::Error {
                                    message: format!("parse error: {e}"),
                                };
                                write_msg(&mut writer, &msg).await?;
                            }
                            Ok(ClientMessage::Ping) => {
                                write_msg(&mut writer, &DaemonMessage::Pong).await?;
                            }
                            Ok(ClientMessage::GetState) => {
                                let prs = state.lock().await.all_prs();
                                write_msg(&mut writer, &DaemonMessage::StateSnapshot {
                                    pull_requests: prs,
                                }).await?;
                            }
                            Ok(ClientMessage::Subscribe) => {
                                let prs = state.lock().await.all_prs();
                                write_msg(&mut writer, &DaemonMessage::StateSnapshot {
                                    pull_requests: prs,
                                }).await?;
                                subscribed = true;
                            }
                        }
                    }
                }
            }

            // Forward live events to subscribed clients.
            result = event_rx.recv(), if subscribed => {
                match result {
                    Ok(event) => {
                        write_msg(&mut writer, &DaemonMessage::Event(event)).await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("server rx lagged, missed {n} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            _ = cancel.cancelled() => break,
        }
    }
    Ok(())
}

async fn write_msg(
    writer: &mut (impl AsyncWriteExt + Unpin),
    msg: &DaemonMessage,
) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    Ok(())
}
