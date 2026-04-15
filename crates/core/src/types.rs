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
    /// GitHub logins of requested reviewers.
    pub reviewers: Vec<String>,
    /// GitHub logins of assignees.
    pub assignees: Vec<String>,
}

/// A GitHub account-level notification (comment, mention, review request, CI, etc.)
/// sourced from the GitHub Notifications API (`GET /notifications`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notification {
    /// Unique notification ID from GitHub.
    pub id: String,
    /// Repository in "owner/repo" format.
    pub repo: String,
    /// Subject type: "PullRequest", "Issue", "CheckSuite", "Release", "Commit", etc.
    pub subject_type: String,
    /// Title of the subject (PR title, issue title, …).
    pub subject_title: String,
    /// Reason for the notification: "comment", "mention", "review_requested",
    /// "ci_activity", "author", "assign", "subscribed", etc.
    pub reason: String,
    /// Browser-ready HTML URL for the subject.
    pub url: String,
    /// Unix timestamp of when the notification was last updated.
    pub updated_at: u64,
    /// Whether the user has marked this notification as seen in the UI.
    #[serde(default)]
    pub seen: bool,
    /// Whether the user has soft-deleted (hidden) this notification.
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VcsEvent {
    NewPullRequest(PullRequest),
    PullRequestUpdated { old: PullRequest, new: PullRequest },
    PullRequestClosed(PullRequest),
    /// A GitHub account-level notification event.
    Notification(Notification),
}
