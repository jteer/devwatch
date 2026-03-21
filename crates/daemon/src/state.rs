use std::collections::HashMap;

use devwatch_core::types::{PullRequest, VcsEvent};

/// Key that uniquely identifies a PR across providers and repos.
type PrKey = (String, String, u64); // (provider, repo, number)

fn key(pr: &PullRequest) -> PrKey {
    (pr.provider.clone(), pr.repo.clone(), pr.number)
}

/// In-memory view of all currently-tracked pull requests.
/// This is the authoritative source for IPC state snapshots.
pub struct DaemonState {
    prs: HashMap<PrKey, PullRequest>,
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            prs: HashMap::new(),
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
