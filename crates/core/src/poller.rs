//! Networking: global rate limiter, per-provider health with exponential
//! backoff, and failover polling.

use crate::model::{ParsedResponse, ProviderId};
use crate::provider::{build_url, parse_for};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

#[derive(Clone, Debug)]
pub struct PollerConfig {
    pub backoff_base: Duration,
    pub backoff_max: Duration,
    /// While running on a fallback, retry a higher-priority provider this often.
    pub preferred_retry: Duration,
    /// Global minimum gap between any two outbound requests.
    pub min_gap: Duration,
    pub timeout: Duration,
    pub user_agent: String,
}

impl Default for PollerConfig {
    fn default() -> Self {
        Self {
            backoff_base: Duration::from_secs(2),
            backoff_max: Duration::from_secs(60),
            preferred_retry: Duration::from_secs(60),
            min_gap: Duration::from_millis(1100),
            timeout: Duration::from_secs(8),
            user_agent: "Overhead/0.1".into(),
        }
    }
}

/// 2 s doubling to 60 s: fails=1 -> base, 2 -> 2*base, ...
pub fn backoff_for(fails: u32, base: Duration, max: Duration) -> Duration {
    if fails == 0 {
        return Duration::ZERO;
    }
    let shift = (fails - 1).min(16);
    base.saturating_mul(1u32 << shift).min(max)
}

pub struct RateLimiter {
    gap: Duration,
    last: Mutex<Option<Instant>>,
}

impl RateLimiter {
    pub fn new(gap: Duration) -> Self {
        Self { gap, last: Mutex::new(None) }
    }
    /// Waits until at least `gap` has passed since the previous request.
    pub async fn acquire(&self) {
        let mut last = self.last.lock().await;
        if let Some(t) = *last {
            let next = t + self.gap;
            if next > Instant::now() {
                tokio::time::sleep_until(next).await;
            }
        }
        *last = Some(Instant::now());
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum HealthState {
    /// Never tried yet.
    Idle,
    Ok,
    Backoff,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub id: ProviderId,
    pub name: String,
    pub state: HealthState,
    pub active: bool,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    /// Seconds until this provider may be tried again (0 when available).
    pub retry_in_s: f64,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum FetchError {
    #[error("HTTP {0}")]
    Http(u16),
    #[error("timeout")]
    Timeout,
    #[error("network: {0}")]
    Network(String),
    #[error("bad response: {0}")]
    Parse(String),
}

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub id: ProviderId,
    /// URL template with {lat} {lon} {nm} (or {lamin}.. for OpenSky).
    pub template: String,
}

impl Endpoint {
    pub fn default_for(id: ProviderId) -> Self {
        Self { id, template: id.default_template().to_string() }
    }
}

#[derive(Default, Clone)]
struct Health {
    fails: u32,
    backoff_until: Option<Instant>,
    last_attempt: Option<Instant>,
    last_error: Option<String>,
    ever_ok: bool,
}

pub struct PollSuccess {
    pub provider: ProviderId,
    pub response: ParsedResponse,
    pub fetched_at: Instant,
}

pub struct Poller {
    client: reqwest::Client,
    cfg: PollerConfig,
    endpoints: Vec<Endpoint>,
    health: HashMap<ProviderId, Health>,
    active: Option<ProviderId>,
    limiter: Arc<RateLimiter>,
}

impl Poller {
    pub fn new(cfg: PollerConfig, endpoints: Vec<Endpoint>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(cfg.user_agent.clone())
            .timeout(cfg.timeout)
            .build()
            .expect("http client");
        let limiter = Arc::new(RateLimiter::new(cfg.min_gap));
        Self { client, cfg, endpoints, health: HashMap::new(), active: None, limiter }
    }

    pub fn set_endpoints(&mut self, endpoints: Vec<Endpoint>) {
        self.endpoints = endpoints;
    }

    pub fn active(&self) -> Option<ProviderId> {
        self.active
    }

    fn eligible(&self, idx: usize, now: Instant) -> bool {
        let id = self.endpoints[idx].id;
        let h = self.health.get(&id).cloned().unwrap_or_default();
        if h.backoff_until.map(|t| now < t).unwrap_or(false) {
            return false;
        }
        // A higher-priority provider than the active one is only retried every `preferred_retry`.
        if let Some(active) = self.active {
            if let Some(active_idx) = self.endpoints.iter().position(|e| e.id == active) {
                if idx < active_idx {
                    if let Some(t) = h.last_attempt {
                        if now.duration_since(t) < self.cfg.preferred_retry {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    async fn fetch(&self, ep: &Endpoint, lat: f64, lon: f64, nm: f64) -> Result<ParsedResponse, FetchError> {
        let url = build_url(&ep.template, lat, lon, nm);
        self.limiter.acquire().await;
        let resp = self.client.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                FetchError::Timeout
            } else {
                FetchError::Network(e.without_url().to_string())
            }
        })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(FetchError::Http(status.as_u16()));
        }
        let body = resp.text().await.map_err(|e| if e.is_timeout() { FetchError::Timeout } else { FetchError::Network(e.without_url().to_string()) })?;
        parse_for(ep.id, &body).map_err(|e| FetchError::Parse(e.to_string()))
    }

    /// Try providers in priority order, skipping those in backoff, switching at
    /// once on failure. `Err` only if every eligible provider failed.
    pub async fn poll(&mut self, lat: f64, lon: f64, nm: f64) -> Result<PollSuccess, Vec<(ProviderId, FetchError)>> {
        let mut errors = Vec::new();
        for idx in 0..self.endpoints.len() {
            let now = Instant::now();
            if !self.eligible(idx, now) {
                continue;
            }
            let ep = self.endpoints[idx].clone();
            self.health.entry(ep.id).or_default().last_attempt = Some(now);
            match self.fetch(&ep, lat, lon, nm).await {
                Ok(response) => {
                    let h = self.health.entry(ep.id).or_default();
                    h.fails = 0;
                    h.backoff_until = None;
                    h.last_error = None;
                    h.ever_ok = true;
                    if self.active != Some(ep.id) {
                        tracing::info!(provider = ep.id.name(), "provider active");
                    }
                    self.active = Some(ep.id);
                    return Ok(PollSuccess { provider: ep.id, response, fetched_at: Instant::now() });
                }
                Err(e) => {
                    let h = self.health.entry(ep.id).or_default();
                    h.fails += 1;
                    h.backoff_until = Some(Instant::now() + backoff_for(h.fails, self.cfg.backoff_base, self.cfg.backoff_max));
                    h.last_error = Some(e.to_string());
                    tracing::warn!(provider = ep.id.name(), error = %e, fails = h.fails, "provider failed");
                    errors.push((ep.id, e));
                }
            }
        }
        if !errors.is_empty() {
            self.active = None;
        }
        Err(errors)
    }

    /// Earliest instant any provider could be tried again (for the poll loop's sleep).
    pub fn next_available_in(&self) -> Duration {
        let now = Instant::now();
        self.endpoints
            .iter()
            .filter_map(|e| self.health.get(&e.id).and_then(|h| h.backoff_until))
            .map(|t| t.saturating_duration_since(now))
            .min()
            .unwrap_or(Duration::ZERO)
    }

    pub fn status(&self) -> Vec<ProviderStatus> {
        let now = Instant::now();
        self.endpoints
            .iter()
            .map(|e| {
                let h = self.health.get(&e.id).cloned().unwrap_or_default();
                let retry = h.backoff_until.map(|t| t.saturating_duration_since(now).as_secs_f64()).unwrap_or(0.0);
                let state = if retry > 0.0 {
                    HealthState::Backoff
                } else if h.ever_ok && h.fails == 0 {
                    HealthState::Ok
                } else if h.fails > 0 {
                    HealthState::Backoff
                } else {
                    HealthState::Idle
                };
                ProviderStatus {
                    id: e.id,
                    name: e.id.name().to_string(),
                    state,
                    active: self.active == Some(e.id),
                    consecutive_failures: h.fails,
                    last_error: h.last_error,
                    retry_in_s: retry,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_from_2_to_60() {
        let b = Duration::from_secs(2);
        let m = Duration::from_secs(60);
        let secs: Vec<u64> = (1..=8).map(|f| backoff_for(f, b, m).as_secs()).collect();
        assert_eq!(secs, vec![2, 4, 8, 16, 32, 60, 60, 60]);
        assert_eq!(backoff_for(0, b, m), Duration::ZERO);
        assert_eq!(backoff_for(500, b, m), m);
    }

    #[tokio::test(start_paused = true)]
    async fn rate_limiter_spaces_requests() {
        let l = RateLimiter::new(Duration::from_secs(1));
        let t0 = Instant::now();
        l.acquire().await;
        l.acquire().await;
        l.acquire().await;
        assert!(Instant::now() - t0 >= Duration::from_secs(2));
    }
}
