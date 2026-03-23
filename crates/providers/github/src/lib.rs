use anyhow::{anyhow, Result};
use async_trait::async_trait;
use devwatch_core::{
    config::RepoConfig,
    provider::VcsProvider,
    types::PullRequest,
};
use tracing::debug;

pub struct GithubProvider {
    client: octocrab::Octocrab,
}

impl GithubProvider {
    pub fn new(token: impl Into<String>) -> Result<Self> {
        let client = octocrab::Octocrab::builder()
            .personal_token(token.into())
            .build()
            .map_err(|e| anyhow!("failed to build octocrab client: {e}"))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl VcsProvider for GithubProvider {
    async fn get_pull_requests(&self, repo: &RepoConfig) -> Result<Vec<PullRequest>> {
        let (owner, repo_name) = repo
            .name
            .split_once('/')
            .ok_or_else(|| anyhow!("invalid repo format '{}': expected 'owner/repo'", repo.name))?;

        debug!(repo = %repo.name, "polling GitHub pull requests");

        let page = self
            .client
            .pulls(owner, repo_name)
            .list()
            .state(octocrab::params::State::Open)
            .per_page(100)
            .send()
            .await
            .map_err(|e| anyhow!("GitHub API error for '{}': {e}", repo.name))?;

        let prs = page
            .items
            .into_iter()
            .map(|pr| {
                let state = match &pr.state {
                    Some(s) => format!("{s:?}").to_lowercase(),
                    None => "unknown".to_string(),
                };
                PullRequest {
                    id:         pr.id.0,
                    number:     pr.number,
                    title:      pr.title.unwrap_or_default(),
                    state,
                    url:        pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
                    author:     pr.user.map(|u| u.login).unwrap_or_default(),
                    repo:       repo.name.clone(),
                    provider:   "github".to_string(),
                    created_at: pr.created_at
                        .map(|dt| dt.timestamp().max(0) as u64)
                        .unwrap_or(0),
                    draft:      pr.draft.unwrap_or(false),
                }
            })
            .collect();

        Ok(prs)
    }

    fn provider_name(&self) -> &str {
        "github"
    }
}

/// Build a GithubProvider from a RepoConfig, falling back to the
/// GITHUB_TOKEN environment variable if no per-repo token is set.
pub fn from_repo_config(repo: &RepoConfig) -> Result<GithubProvider> {
    let token = repo
        .token
        .clone()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .ok_or_else(|| {
            anyhow!(
                "no token for GitHub repo '{}': set `token` in config or GITHUB_TOKEN env var",
                repo.name
            )
        })?;
    GithubProvider::new(token)
}
