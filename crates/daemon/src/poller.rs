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
    // One provider per unique GitHub account, used for account-level notifications.
    notif_providers: Arc<Vec<Arc<dyn VcsProvider>>>,
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
                let _ = event_tx.send(DaemonMessage::PollingStarted);
                poll_all(&entries, &notif_providers, &state, &store, &event_tx).await;
                let _ = event_tx.send(DaemonMessage::PollingFinished);
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
    notif_providers: &[Arc<dyn VcsProvider>],
    state: &Mutex<DaemonState>,
    store: &Mutex<Store>,
    event_tx: &broadcast::Sender<DaemonMessage>,
) {
    // ── PR state polling ──────────────────────────────────────────────────────
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
                            VcsEvent::Notification(_) => {} // handled below
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

    // ── Account-level notification polling ────────────────────────────────────
    for provider in notif_providers {
        match provider.get_notifications().await {
            Err(e) => {
                warn!("notification poll error: {e}");
            }
            Ok(notifs) => {
                let new_notifs = state.lock().await.filter_new_notifications(notifs);
                if new_notifs.is_empty() {
                    continue;
                }
                info!(count = new_notifs.len(), "broadcasting new notifications");
                let db = store.lock().await;
                for notif in new_notifs {
                    if let Err(e) = db.upsert_notification(&notif) {
                        error!("db upsert_notification: {e}");
                    }
                    let _ = event_tx.send(DaemonMessage::Event(VcsEvent::Notification(notif)));
                }
            }
        }
    }
}
