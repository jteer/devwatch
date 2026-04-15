use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use devwatch_core::ipc::DaemonMessage;
use devwatch_core::types::VcsEvent;

pub async fn notify_loop(
    mut event_rx: broadcast::Receiver<DaemonMessage>,
    cancel: CancellationToken,
) {
    loop {
        tokio::select! {
            result = event_rx.recv() => {
                match result {
                    Ok(DaemonMessage::Event(event)) => send_notification(&event),
                    Ok(_) => {} // Polled, Pong, etc. — nothing to notify
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("notifier lagged, missed {n} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("event channel closed, notifier exiting");
                        break;
                    }
                }
            }
            _ = cancel.cancelled() => {
                info!("notifier shutting down");
                break;
            }
        }
    }
}

fn send_notification(event: &VcsEvent) {
    let (summary, body) = match event {
        VcsEvent::NewPullRequest(pr) => (
            format!("New PR: {}", pr.title),
            format!("#{} in {} by {}", pr.number, pr.repo, pr.author),
        ),
        VcsEvent::PullRequestUpdated { new, .. } => (
            format!("PR updated: {}", new.title),
            format!("#{} in {} — state: {}", new.number, new.repo, new.state),
        ),
        VcsEvent::PullRequestClosed(pr) => (
            format!("PR closed: {}", pr.title),
            format!("#{} in {}", pr.number, pr.repo),
        ),
        VcsEvent::Notification(n) => {
            let summary = match n.reason.as_str() {
                "comment"          => format!("New comment: {}", n.subject_title),
                "mention"          => format!("Mentioned: {}", n.subject_title),
                "review_requested" => format!("Review requested: {}", n.subject_title),
                "ci_activity"      => format!("CI update: {}", n.subject_title),
                "assign"           => format!("Assigned: {}", n.subject_title),
                _                  => format!("GitHub: {}", n.subject_title),
            };
            (summary, n.repo.clone())
        }
    };

    if let Err(e) = notify_rust::Notification::new()
        .summary(&summary)
        .body(&body)
        .appname("devwatch")
        .show()
    {
        error!("notification error: {e}");
    }
}
