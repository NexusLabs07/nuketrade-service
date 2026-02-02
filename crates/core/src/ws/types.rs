//! WebSocket configuration and types.

use chrono::{Timelike, Utc};
use std::time::Duration;
use tokio::time::Instant;

/// Configuration for WebSocket connections and feed behavior.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Maximum number of connection retries before backing off.
    pub max_retries: u32,
    /// Delay between reconnection attempts in seconds.
    pub reconnect_delay_secs: u64,
    /// Interval between database writes in seconds.
    pub db_write_interval_secs: u64,
    /// Interval between state updates in seconds.
    pub state_update_interval_secs: u64,
    /// Optional ping interval in seconds for keepalive.
    pub ping_interval_secs: Option<u64>,
    /// Maximum time without updates before data is considered stale.
    pub stale_threshold_secs: u64,
    /// Whether to use custom WebSocket config (for certain exchanges).
    pub use_custom_ws_config: bool,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            reconnect_delay_secs: 5,
            db_write_interval_secs: 30 * 60, // 30 minutes
            state_update_interval_secs: 5,
            ping_interval_secs: None,
            stale_threshold_secs: 60,
            use_custom_ws_config: false,
        }
    }
}

impl WsConfig {
    /// Create a new WsConfig with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set reconnect delay.
    pub fn with_reconnect_delay(mut self, secs: u64) -> Self {
        self.reconnect_delay_secs = secs;
        self
    }

    /// Set database write interval.
    pub fn with_db_write_interval(mut self, secs: u64) -> Self {
        self.db_write_interval_secs = secs;
        self
    }

    /// Set state update interval.
    pub fn with_state_update_interval(mut self, secs: u64) -> Self {
        self.state_update_interval_secs = secs;
        self
    }

    /// Set ping interval for keepalive.
    pub fn with_ping_interval(mut self, secs: u64) -> Self {
        self.ping_interval_secs = Some(secs);
        self
    }

    /// Set stale threshold.
    pub fn with_stale_threshold(mut self, secs: u64) -> Self {
        self.stale_threshold_secs = secs;
        self
    }

    /// Enable custom WebSocket config.
    pub fn with_custom_ws_config(mut self) -> Self {
        self.use_custom_ws_config = true;
        self
    }

    /// Get reconnect delay as Duration.
    pub fn reconnect_delay(&self) -> Duration {
        Duration::from_secs(self.reconnect_delay_secs)
    }

    /// Get DB write interval as Duration.
    pub fn db_write_interval(&self) -> Duration {
        Duration::from_secs(self.db_write_interval_secs)
    }

    /// Get state update interval as Duration.
    pub fn state_update_interval(&self) -> Duration {
        Duration::from_secs(self.state_update_interval_secs)
    }

    /// Get ping interval as Duration, if set.
    pub fn ping_interval(&self) -> Option<Duration> {
        self.ping_interval_secs.map(Duration::from_secs)
    }

    /// Get stale threshold as Duration.
    pub fn stale_threshold(&self) -> Duration {
        Duration::from_secs(self.stale_threshold_secs)
    }

    /// Calculate the Instant for the next aligned DB write time.
    ///
    /// This ensures all exchanges write to the database at the same wall-clock times
    /// (e.g., at :00 and :30 for 30-minute intervals), making it easier to correlate
    /// funding rates across exchanges on the frontend.
    pub fn next_aligned_db_write(&self) -> Instant {
        let now = Utc::now();
        let interval_secs = self.db_write_interval_secs as i64;

        // Calculate seconds since midnight
        let seconds_today = now.num_seconds_from_midnight() as i64;

        // Find the next aligned time slot
        let current_slot = seconds_today / interval_secs;
        let next_slot_seconds = (current_slot + 1) * interval_secs;

        // Calculate seconds until next slot
        let seconds_until_next = next_slot_seconds - seconds_today;

        // Convert to Instant (from now)
        Instant::now() + Duration::from_secs(seconds_until_next as u64)
    }
}
