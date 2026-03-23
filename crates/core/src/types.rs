use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    /// "open", "closed", or "merged"
    pub state: String,
    pub url: String,
    pub author: String,
    /// "owner/repo"
    pub repo: String,
    /// "github" or "gitlab"
    pub provider: String,
    /// Unix timestamp of when the PR was created; 0 = unknown.
    pub created_at: u64,
    /// True if this is a draft / work-in-progress PR.
    pub draft: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VcsEvent {
    NewPullRequest(PullRequest),
    PullRequestUpdated { old: PullRequest, new: PullRequest },
    PullRequestClosed(PullRequest),
}
