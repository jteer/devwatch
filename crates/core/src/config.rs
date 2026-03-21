use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    /// TCP port the daemon listens on (default: 7878)
    #[serde(default = "default_port")]
    pub daemon_port: u16,

    /// How often to poll each repo, in seconds (default: 60)
    #[serde(default = "default_interval")]
    pub poll_interval_secs: u64,

    pub repos: Vec<RepoConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RepoConfig {
    /// "github" or "gitlab"
    pub provider: String,

    /// "owner/repo" format
    pub name: String,

    /// Per-repo PAT; falls back to the provider-level token if omitted
    pub token: Option<String>,
}

fn default_port() -> u16 {
    7878
}

fn default_interval() -> u64 {
    60
}
