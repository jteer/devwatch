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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VcsEvent {
    NewPullRequest(PullRequest),
    PullRequestUpdated { old: PullRequest, new: PullRequest },
    PullRequestClosed(PullRequest),
}
