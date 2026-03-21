//! GitLab VCS provider — stub implementation.
//!
//! TODO (Phase N): implement using the GitLab REST API.
//! All methods currently return a "not implemented" error so the workspace
//! compiles and the trait boundary is satisfied.

use anyhow::anyhow;
use async_trait::async_trait;
use devwatch_core::{config::RepoConfig, provider::VcsProvider, types::PullRequest};

pub struct GitlabProvider;

impl GitlabProvider {
    pub fn new(_token: impl Into<String>) -> Self {
        Self
    }
}

#[async_trait]
impl VcsProvider for GitlabProvider {
    async fn get_pull_requests(&self, repo: &RepoConfig) -> anyhow::Result<Vec<PullRequest>> {
        Err(anyhow!(
            "GitLab provider not yet implemented (repo: '{}')",
            repo.name
        ))
    }

    fn provider_name(&self) -> &str {
        "gitlab"
    }
}
