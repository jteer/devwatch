mod notifier;
mod poller;
mod server;
mod state;
mod store;

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::info;

use devwatch_core::{ipc::DaemonMessage, AppConfig, VcsProvider};
use poller::ProviderEntry;
use state::DaemonState;
use store::Store;

fn load_config() -> anyhow::Result<AppConfig> {
    let cfg = config::Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("DEVWATCH").separator("__"))
        .build()?
        .try_deserialize::<AppConfig>()?;
    Ok(cfg)
}

#[tokio::main]
async fn main() -> Result<()> {
    // ── Logging ──────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "daemon=info,warn".parse().unwrap()),
        )
        .init();

    // ── Config ────────────────────────────────────────────────────────────────
    let cfg = load_config()?;
    info!(
        repos = cfg.repos.len(),
        poll_interval = cfg.poll_interval_secs,
        port = cfg.daemon_port,
        "devwatch daemon starting"
    );

    // ── SQLite store ──────────────────────────────────────────────────────────
    let store = Store::open()?;
    let seed_prs = store.load_prs()?;
    info!(loaded = seed_prs.len(), "seeded state from DB");

    let notification_ids = store.load_notification_ids()?;
    info!(loaded = notification_ids.len(), "seeded notification dedup from DB");
    let store = Arc::new(Mutex::new(store));

    // ── State ─────────────────────────────────────────────────────────────────
    let daemon_state = Arc::new(Mutex::new(
        DaemonState::from_prs(seed_prs).with_known_notification_ids(notification_ids),
    ));

    // ── Providers ─────────────────────────────────────────────────────────────
    let entries: Vec<ProviderEntry> = build_provider_entries(&cfg)?;
    let entries = Arc::new(entries);

    let notif_providers: Vec<Arc<dyn VcsProvider>> = build_notification_providers(&cfg)?;
    let notif_providers = Arc::new(notif_providers);

    // ── Broadcast channel ─────────────────────────────────────────────────────
    // Capacity: hold up to 256 events before slow receivers lag.
    let (event_tx, _) = broadcast::channel::<DaemonMessage>(256);

    // ── Cancellation ──────────────────────────────────────────────────────────
    let cancel = CancellationToken::new();

    // ── Spawn tasks ───────────────────────────────────────────────────────────
    let poller_handle = {
        let entries         = Arc::clone(&entries);
        let notif_providers = Arc::clone(&notif_providers);
        let state           = Arc::clone(&daemon_state);
        let store           = Arc::clone(&store);
        let tx              = event_tx.clone();
        let cancel          = cancel.clone();
        let interval        = cfg.poll_interval_secs;
        tokio::spawn(async move {
            poller::poll_loop(entries, notif_providers, state, store, tx, interval, cancel).await;
        })
    };

    let notifier_handle = {
        let rx     = event_tx.subscribe();
        let cancel = cancel.clone();
        tokio::spawn(async move {
            notifier::notify_loop(rx, cancel).await;
        })
    };

    let server_handle = {
        let state  = Arc::clone(&daemon_state);
        let tx     = event_tx.clone();
        let cancel = cancel.clone();
        let port   = cfg.daemon_port;
        tokio::spawn(async move {
            if let Err(e) = server::run_server(port, state, tx, cancel).await {
                tracing::error!("server error: {e}");
            }
        })
    };

    // ── Await shutdown signal ─────────────────────────────────────────────────
    tokio::signal::ctrl_c().await?;
    info!("received shutdown signal");
    cancel.cancel();

    let _ = tokio::join!(poller_handle, notifier_handle, server_handle);
    info!("devwatch daemon stopped");
    Ok(())
}

/// Build one notification-capable provider per unique GitHub token.
/// Deduplicates so the Notifications API is only called once per account per poll.
fn build_notification_providers(cfg: &AppConfig) -> Result<Vec<Arc<dyn VcsProvider>>> {
    let mut seen_tokens: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut providers: Vec<Arc<dyn VcsProvider>> = Vec::new();

    for repo in &cfg.repos {
        if repo.provider != "github" {
            continue;
        }
        let token = repo
            .token
            .clone()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .unwrap_or_default();
        if token.is_empty() {
            continue;
        }
        if seen_tokens.insert(token.clone()) {
            match provider_github::GithubProvider::new(token) {
                Ok(p) => providers.push(Arc::new(p)),
                Err(e) => tracing::warn!("failed to build notification provider: {e}"),
            }
        }
    }

    Ok(providers)
}

fn build_provider_entries(cfg: &AppConfig) -> Result<Vec<ProviderEntry>> {
    let mut entries = Vec::new();

    for repo in &cfg.repos {
        let provider: Arc<dyn VcsProvider> = match repo.provider.as_str() {
            "github" => Arc::new(provider_github::from_repo_config(repo)?),
            "gitlab" => {
                let token = repo.token.clone().unwrap_or_default();
                Arc::new(provider_gitlab::GitlabProvider::new(token))
            }
            other => {
                anyhow::bail!("unknown provider '{}' for repo '{}'", other, repo.name);
            }
        };

        entries.push(ProviderEntry {
            provider,
            repo: repo.clone(),
        });
    }

    Ok(entries)
}
