use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::{PullRequest, VcsEvent};

/// Messages sent **from the daemon to clients**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonMessage {
    /// A state-change event (new PR, updated PR, closed PR).
    Event(VcsEvent),
    /// Full snapshot of currently-tracked pull requests.
    StateSnapshot { pull_requests: Vec<PullRequest> },
    /// Daemon is beginning a poll cycle.
    PollingStarted,
    /// Daemon completed a poll cycle.
    PollingFinished,
    /// An error the daemon wants to surface to the client.
    Error { message: String },
    /// Response to a client Ping.
    Pong,
}

/// Messages sent **from clients to the daemon**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Subscribe: receive a StateSnapshot immediately, then live Events.
    Subscribe,
    /// Request a StateSnapshot without subscribing to the event stream.
    GetState,
    /// Liveness check.
    Ping,
}

/// Transport abstraction so the JSON/TCP implementation can be swapped
/// for gRPC/tonic, tarpc, or Unix-socket transports without touching
/// the daemon or client business logic.
#[async_trait]
pub trait MonitorTransport: Send + Sync {
    async fn send_daemon_msg(&self, msg: &DaemonMessage) -> Result<()>;
    async fn recv_client_msg(&self) -> Result<ClientMessage>;
}
