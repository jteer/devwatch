use anyhow::Result;
use async_trait::async_trait;

use crate::config::RepoConfig;
use crate::types::PullRequest;

/// Implemented by each VCS provider (GitHub, GitLab, …).
/// The daemon only ever calls this trait — never concrete types directly.
#[async_trait]
pub trait VcsProvider: Send + Sync {
    /// Return the current open pull requests for the given repo.
    /// State diffing (and event generation) is handled by the daemon.
    async fn get_pull_requests(&self, repo: &RepoConfig) -> Result<Vec<PullRequest>>;

    fn provider_name(&self) -> &str;
}
