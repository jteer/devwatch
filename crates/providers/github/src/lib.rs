use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use devwatch_core::{
    config::RepoConfig,
    provider::VcsProvider,
    types::{Notification, PullRequest},
};
use tracing::debug;

// ── GitHub Notifications API response shapes ──────────────────────────────────

#[derive(Deserialize)]
struct GhNotif {
    id: String,
    repository: GhRepo,
    subject: GhSubject,
    reason: String,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct GhRepo {
    full_name: String,
}

#[derive(Deserialize)]
struct GhSubject {
    title: String,
    url: String,
    #[serde(rename = "type")]
    kind: String,
}

/// Convert a GitHub API URL to a browser-accessible HTML URL.
///
/// Examples:
///   `https://api.github.com/repos/owner/repo/pulls/123`
///     → `https://github.com/owner/repo/pull/123`
///   `https://api.github.com/repos/owner/repo/issues/456`
///     → `https://github.com/owner/repo/issues/456`
fn api_url_to_html(api_url: &str) -> String {
    api_url
        .replace("api.github.com/repos/", "github.com/")
        .replace("/pulls/", "/pull/")
}

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
                    reviewers:  pr.requested_reviewers
                        .unwrap_or_default()
                        .into_iter()
                        .map(|u| u.login)
                        .collect(),
                    assignees:  pr.assignees
                        .unwrap_or_default()
                        .into_iter()
                        .map(|u| u.login)
                        .collect(),
                }
            })
            .collect();

        Ok(prs)
    }

    fn provider_name(&self) -> &str {
        "github"
    }

    async fn get_notifications(&self) -> Result<Vec<Notification>> {
        debug!("polling GitHub notifications");

        let mut raw: Vec<GhNotif> = Vec::new();
        let mut page_num = 1u32;
        loop {
            let page_str = page_num.to_string();
            let batch: Vec<GhNotif> = self
                .client
                .get("/notifications", Some(&[("per_page", "100"), ("page", page_str.as_str())]))
                .await
                .map_err(|e| anyhow!("GitHub notifications API error (page {page_num}): {e}"))?;
            let done = batch.len() < 100;
            raw.extend(batch);
            if done { break; }
            page_num += 1;
        }
        debug!("fetched {} notifications across {} page(s)", raw.len(), page_num);

        let notifs = raw
            .into_iter()
            .map(|n| Notification {
                id:            n.id,
                repo:          n.repository.full_name,
                subject_type:  n.subject.kind,
                subject_title: n.subject.title,
                reason:        n.reason,
                url:           api_url_to_html(&n.subject.url),
                updated_at:    n.updated_at.timestamp().max(0) as u64,
                seen:          false,
                hidden:        false,
            })
            .collect();

        Ok(notifs)
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
