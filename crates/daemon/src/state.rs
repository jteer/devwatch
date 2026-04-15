use std::collections::{HashMap, HashSet};

use devwatch_core::types::{Notification, PullRequest, VcsEvent};

/// Key that uniquely identifies a PR across providers and repos.
type PrKey = (String, String, u64); // (provider, repo, number)

fn key(pr: &PullRequest) -> PrKey {
    (pr.provider.clone(), pr.repo.clone(), pr.number)
}

/// In-memory view of all currently-tracked pull requests.
/// This is the authoritative source for IPC state snapshots.
pub struct DaemonState {
    prs: HashMap<PrKey, PullRequest>,
    /// Notification IDs seen in this session — prevents re-broadcasting on each poll.
    seen_notification_ids: HashSet<String>,
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            prs: HashMap::new(),
            seen_notification_ids: HashSet::new(),
        }
    }

    /// Seed state from persisted records (called once at startup).
    /// No events are generated — these PRs are already "seen".
    pub fn from_prs(prs: Vec<PullRequest>) -> Self {
        let mut state = Self::new();
        for pr in prs {
            state.prs.insert(key(&pr), pr);
        }
        state
    }

    /// Seed the notification dedup set from persisted IDs so restarts don't re-broadcast.
    pub fn with_known_notification_ids(mut self, ids: Vec<String>) -> Self {
        self.seen_notification_ids.extend(ids);
        self
    }

    /// Filter `notifs` to only those not yet seen this session.
    /// Inserts the IDs of returned notifications into the seen set.
    pub fn filter_new_notifications(&mut self, notifs: Vec<Notification>) -> Vec<Notification> {
        let new: Vec<Notification> = notifs
            .into_iter()
            .filter(|n| !self.seen_notification_ids.contains(&n.id))
            .collect();
        for n in &new {
            self.seen_notification_ids.insert(n.id.clone());
        }
        new
    }

    /// Diff `incoming` against the current state, update the map, and
    /// return the resulting events (new / updated / closed).
    pub fn update(&mut self, incoming: Vec<PullRequest>) -> Vec<VcsEvent> {
        let mut events = Vec::new();

        // Track which keys are present in the new snapshot.
        let mut seen_keys: std::collections::HashSet<PrKey> = std::collections::HashSet::new();

        for pr in incoming {
            let k = key(&pr);
            seen_keys.insert(k.clone());

            match self.prs.get(&k) {
                None => {
                    events.push(VcsEvent::NewPullRequest(pr.clone()));
                    self.prs.insert(k, pr);
                }
                Some(old) if old != &pr => {
                    events.push(VcsEvent::PullRequestUpdated {
                        old: old.clone(),
                        new: pr.clone(),
                    });
                    self.prs.insert(k, pr);
                }
                Some(_) => {} // unchanged
            }
        }

        // Any key not present in the new snapshot has been closed/merged.
        let closed_keys: Vec<PrKey> = self
            .prs
            .keys()
            .filter(|k| !seen_keys.contains(*k))
            .cloned()
            .collect();

        for k in closed_keys {
            if let Some(pr) = self.prs.remove(&k) {
                events.push(VcsEvent::PullRequestClosed(pr));
            }
        }

        events
    }

    pub fn all_prs(&self) -> Vec<PullRequest> {
        self.prs.values().cloned().collect()
    }
}
