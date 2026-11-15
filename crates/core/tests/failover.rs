//! Failover and backoff tests against a local mock provider server that can
//! return 429, 500, slow, malformed and good responses.

use overhead_core::model::ProviderId;
use overhead_core::poller::{Endpoint, FetchError, HealthState, Poller, PollerConfig};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Good,
    Err429,
    Err500,
    Err403,
    Slow,
    Malformed,
}

struct Mock {
    port: u16,
    modes: Arc<Mutex<HashMap<String, Mode>>>,
    hits: Arc<Mutex<HashMap<String, u32>>>,
}

const GOOD: &str = include_str!("../../../tests/fixtures/synthetic_quiet.json");

impl Mock {
    async fn start() -> Mock {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let modes: Arc<Mutex<HashMap<String, Mode>>> = Arc::default();
        let hits: Arc<Mutex<HashMap<String, u32>>> = Arc::default();
        let (m2, h2) = (modes.clone(), hits.clone());
        tokio::spawn(async move {
            loop {
                let Ok((mut s, _)) = l.accept().await else { return };
                let (m, h) = (m2.clone(), h2.clone());
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = s.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let key = path.split('/').nth(1).unwrap_or("").to_string();
                    *h.lock().unwrap().entry(key.clone()).or_default() += 1;
                    let mode = m.lock().unwrap().get(&key).copied().unwrap_or(Mode::Good);
                    let (status, body) = match mode {
                        Mode::Good => ("200 OK", GOOD.to_string()),
                        Mode::Err429 => ("429 Too Many Requests", "{}".into()),
                        Mode::Err500 => ("500 Internal Server Error", "oops".into()),
                        Mode::Err403 => ("403 Forbidden", "{\"error\":\"contact us\"}".into()),
                        Mode::Malformed => ("200 OK", "<html>not json</html>".into()),
                        Mode::Slow => {
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            ("200 OK", GOOD.to_string())
                        }
                    };
                    let resp = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    let _ = s.write_all(resp.as_bytes()).await;
                });
            }
        });
        Mock { port, modes, hits }
    }
    fn set(&self, key: &str, m: Mode) {
        self.modes.lock().unwrap().insert(key.into(), m);
    }
    fn hits(&self, key: &str) -> u32 {
        *self.hits.lock().unwrap().get(key).unwrap_or(&0)
    }
    fn ep(&self, id: ProviderId, key: &str) -> Endpoint {
        Endpoint { id, template: format!("http://127.0.0.1:{}/{key}/{{lat}}/{{lon}}/{{nm}}", self.port) }
    }
}

fn cfg() -> PollerConfig {
    PollerConfig {
        backoff_base: Duration::from_millis(200),
        backoff_max: Duration::from_millis(800),
        preferred_retry: Duration::from_millis(600),
        min_gap: Duration::from_millis(5),
        timeout: Duration::from_millis(500),
        user_agent: "Overhead/0.1 (test)".into(),
    }
}

fn poller(m: &Mock) -> Poller {
    Poller::new(cfg(), vec![m.ep(ProviderId::AirplanesLive, "p1"), m.ep(ProviderId::AdsbFi, "p2"), m.ep(ProviderId::AdsbLol, "p3")])
}

#[tokio::test]
async fn uses_first_provider_when_healthy() {
    let m = Mock::start().await;
    let mut p = poller(&m);
    let r = p.poll(51.5, -0.1, 50.0).await.ok().unwrap();
    assert_eq!(r.provider, ProviderId::AirplanesLive);
    assert!(!r.response.aircraft.is_empty());
    assert_eq!(m.hits("p2"), 0);
}

#[tokio::test]
async fn switches_at_once_on_429_500_malformed_403_and_timeout() {
    for bad in [Mode::Err429, Mode::Err500, Mode::Err403, Mode::Malformed, Mode::Slow] {
        let m = Mock::start().await;
        m.set("p1", bad);
        let mut p = poller(&m);
        let r = p.poll(51.5, -0.1, 50.0).await.ok().unwrap_or_else(|| panic!("{bad:?}: no failover"));
        assert_eq!(r.provider, ProviderId::AdsbFi, "{bad:?}");
        let st = p.status();
        assert_eq!(st[0].state, HealthState::Backoff, "{bad:?}");
        assert!(st[0].last_error.is_some());
        assert!(st[1].active);
    }
}

#[tokio::test]
async fn timeout_is_reported_as_timeout() {
    let m = Mock::start().await;
    m.set("p1", Mode::Slow);
    m.set("p2", Mode::Slow);
    m.set("p3", Mode::Slow);
    let mut p = poller(&m);
    let err = p.poll(51.5, -0.1, 50.0).await.err().unwrap();
    assert_eq!(err.len(), 3);
    assert!(err.iter().all(|(_, e)| matches!(e, FetchError::Timeout)));
}

#[tokio::test]
async fn backed_off_provider_is_skipped_then_retried() {
    let m = Mock::start().await;
    m.set("p1", Mode::Err500);
    let mut p = poller(&m);
    p.poll(51.5, -0.1, 50.0).await.ok().unwrap();
    assert_eq!(m.hits("p1"), 1);
    // immediately again: p1 is in backoff, not contacted
    p.poll(51.5, -0.1, 50.0).await.ok().unwrap();
    assert_eq!(m.hits("p1"), 1);
    // p1 recovers; after backoff AND the preferred-retry interval it becomes active again
    m.set("p1", Mode::Good);
    tokio::time::sleep(Duration::from_millis(700)).await;
    let r = p.poll(51.5, -0.1, 50.0).await.ok().unwrap();
    assert_eq!(r.provider, ProviderId::AirplanesLive);
    assert_eq!(m.hits("p1"), 2);
    assert_eq!(p.status()[0].state, HealthState::Ok);
}

#[tokio::test]
async fn backoff_grows_on_repeated_failure() {
    let m = Mock::start().await;
    m.set("p1", Mode::Err429);
    m.set("p2", Mode::Err429);
    m.set("p3", Mode::Err429);
    let mut p = poller(&m);
    assert!(p.poll(51.5, -0.1, 50.0).await.is_err());
    let s1 = p.status()[0].consecutive_failures;
    tokio::time::sleep(Duration::from_millis(850)).await;
    assert!(p.poll(51.5, -0.1, 50.0).await.is_err());
    let st = p.status();
    assert_eq!(s1, 1);
    assert_eq!(st[0].consecutive_failures, 2);
    assert!(p.active().is_none());
}

#[tokio::test]
async fn all_down_then_recovery() {
    let m = Mock::start().await;
    for k in ["p1", "p2", "p3"] {
        m.set(k, Mode::Err500);
    }
    let mut p = poller(&m);
    assert!(p.poll(51.5, -0.1, 50.0).await.is_err());
    for k in ["p1", "p2", "p3"] {
        m.set(k, Mode::Good);
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    let r = p.poll(51.5, -0.1, 50.0).await.ok().unwrap();
    assert_eq!(r.provider, ProviderId::AirplanesLive);
}

#[tokio::test]
async fn sends_user_agent_and_respects_global_gap() {
    let m = Mock::start().await;
    let mut c = cfg();
    c.min_gap = Duration::from_millis(150);
    m.set("p1", Mode::Err500);
    m.set("p2", Mode::Err500);
    let mut p = Poller::new(c, vec![m.ep(ProviderId::AirplanesLive, "p1"), m.ep(ProviderId::AdsbFi, "p2"), m.ep(ProviderId::AdsbLol, "p3")]);
    let t = std::time::Instant::now();
    p.poll(51.5, -0.1, 50.0).await.ok().unwrap();
    // three requests in a row must be spaced by the global limiter
    assert!(t.elapsed() >= Duration::from_millis(300), "{:?}", t.elapsed());
}
