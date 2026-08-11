use anyhow::Context;

/// Configuration for the feed/worker process, loaded from environment
/// variables. User-facing API config lives in the TypeScript server
/// (perp-aggragetor-ts); this process only needs the database, Turnkey
/// signing credentials, and automation worker settings.
#[derive(Debug, Clone)]
pub struct Config {
    // Database
    pub db_url: String,

    // Turnkey (automation worker signs orders through it)
    pub turnkey_api_base_url: String,
    pub turnkey_parent_org_id: String,
    pub turnkey_api_public_key: String,
    pub turnkey_api_private_key: String,

    // Automation: shared bearer token protecting the /internal API
    // (feed snapshot + intent polling), used by the TypeScript server.
    pub automation_internal_token: Option<String>,
    /// Lease TTL (in seconds) issued to a worker when it picks up an
    /// intent. Defaults to 60s.
    pub automation_lease_ttl_sec: i64,

    // In-process automation worker (executor for Hyperliquid/Pacifica).
    //
    // Note: there is no shared "agent wallet" env var. HL's agent model is
    // 1 agent ↔ 1 master, so each user has their own Turnkey-managed agent
    // wallet stored in `hl_agent_wallets`. The worker looks it up per
    // intent.
    pub automation_worker_enabled: bool,
    pub automation_worker_poll_interval_sec: u64,
    pub automation_worker_id: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing.
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let db_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable is required")?;

        let turnkey_api_base_url = std::env::var("TURNKEY_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.turnkey.com".to_string());

        let turnkey_parent_org_id = std::env::var("TURNKEY_PARENT_ORG_ID")
            .or_else(|_| std::env::var("TURNKEY_ORGANIZATION_ID"))
            .context("TURNKEY_PARENT_ORG_ID (or TURNKEY_ORGANIZATION_ID) is required")?;

        let turnkey_api_public_key = std::env::var("TURNKEY_API_PUBLIC_KEY")
            .context("TURNKEY_API_PUBLIC_KEY is required")?;

        let turnkey_api_private_key = std::env::var("TURNKEY_API_PRIVATE_KEY")
            .context("TURNKEY_API_PRIVATE_KEY is required")?;

        let automation_internal_token = std::env::var("AUTOMATION_INTERNAL_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        let automation_lease_ttl_sec = std::env::var("AUTOMATION_LEASE_TTL_SEC")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        let automation_worker_enabled = std::env::var("AUTOMATION_WORKER_ENABLED")
            .ok()
            .map(|s| matches!(s.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);
        let automation_worker_poll_interval_sec =
            std::env::var("AUTOMATION_WORKER_POLL_INTERVAL_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);
        let automation_worker_id =
            std::env::var("AUTOMATION_WORKER_ID").unwrap_or_else(|_| "rust-executor".to_string());

        Ok(Self {
            db_url,
            turnkey_api_base_url,
            turnkey_parent_org_id,
            turnkey_api_public_key,
            turnkey_api_private_key,
            automation_internal_token,
            automation_lease_ttl_sec,
            automation_worker_enabled,
            automation_worker_poll_interval_sec,
            automation_worker_id,
        })
    }
}
