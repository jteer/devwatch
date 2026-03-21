use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use devwatch_core::{config::RepoConfig, ipc::DaemonMessage, provider::VcsProvider, types::VcsEvent};

use crate::state::DaemonState;
use crate::store::Store;

pub struct ProviderEntry {
    pub provider: Arc<dyn VcsProvider>,
    pub repo: RepoConfig,
}

pub async fn poll_loop(
    entries: Arc<Vec<ProviderEntry>>,
    state: Arc<Mutex<DaemonState>>,
    store: Arc<Mutex<Store>>,
    event_tx: broadcast::Sender<DaemonMessage>,
    interval_secs: u64,
    cancel: CancellationToken,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
    // Don't wait for the first tick — poll immediately on startup.
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                poll_all(&entries, &state, &store, &event_tx).await;
                // Always broadcast Polled so clients can reset their countdown,
                // even when no PRs changed this cycle.
                let _ = event_tx.send(DaemonMessage::Polled);
            }
            _ = cancel.cancelled() => {
                info!("poller shutting down");
                break;
            }
        }
    }
}

async fn poll_all(
    entries: &[ProviderEntry],
    state: &Mutex<DaemonState>,
    store: &Mutex<Store>,
    event_tx: &broadcast::Sender<DaemonMessage>,
) {
    for entry in entries {
        match entry.provider.get_pull_requests(&entry.repo).await {
            Err(e) => {
                warn!(repo = %entry.repo.name, "poll error: {e}");
            }
            Ok(prs) => {
                let events = state.lock().await.update(prs.clone());

                if events.is_empty() {
                    continue;
                }

                // Persist each PR change and record the event.
                {
                    let db = store.lock().await;
                    for event in &events {
                        match event {
                            VcsEvent::PullRequestClosed(pr) => {
                                if let Err(e) = db.delete_pr(&pr.provider, &pr.repo, pr.number) {
                                    error!("db delete_pr: {e}");
                                }
                            }
                            VcsEvent::NewPullRequest(pr)
                            | VcsEvent::PullRequestUpdated { new: pr, .. } => {
                                if let Err(e) = db.upsert_pr(pr) {
                                    error!("db upsert_pr: {e}");
                                }
                            }
                        }
                        if let Err(e) = db.record_event(event) {
                            error!("db record_event: {e}");
                        }
                    }
                }

                // Broadcast events to connected clients.
                for event in events {
                    let _ = event_tx.send(DaemonMessage::Event(event));
                }
            }
        }
    }
}
