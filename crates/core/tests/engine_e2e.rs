#![allow(clippy::field_reassign_with_default)]
//! End-to-end: the async engine in demo mode and against the mock provider.
use overhead_core::alerts::AlertEvent;
use overhead_core::engine::{Emitter, Engine, EngineConfig, EnrichmentEvent, LinkState, Snapshot, SourceKind};
use overhead_core::model::ProviderId;
use overhead_core::poller::{Endpoint, PollerConfig};
use overhead_core::settings::{Home, Settings};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct Rec {
    snaps: Mutex<Vec<Snapshot>>,
    alerts: Mutex<Vec<AlertEvent>>,
}
impl Emitter for Rec {
    fn snapshot(&self, s: &Snapshot) { self.snaps.lock().unwrap().push(s.clone()); }
    fn enrichment(&self, _e: &EnrichmentEvent) {}
    fn alert(&self, a: &AlertEvent) { self.alerts.lock().unwrap().push(a.clone()); }
}

#[tokio::test]
async fn demo_mode_runs_without_network_and_without_home() {
    let dir = tempfile::tempdir().unwrap();
    let rec = Arc::new(Rec::default());
    let h = Engine::spawn(EngineConfig::new(dir.path().to_path_buf()), rec.clone());
    tokio::time::sleep(Duration::from_millis(600)).await;
    let snaps = rec.snaps.lock().unwrap().clone();
    let s = snaps.last().expect("a snapshot was emitted");
    assert_eq!(s.source, SourceKind::Demo);
    assert!(s.home_is_demo);
    assert!(s.aircraft.len() >= 10);
    assert_eq!(s.link, LinkState::Live);
    assert!(s.featured.is_some());
    h.shutdown();
}

#[tokio::test]
async fn setting_home_persists_settings_and_switches_to_live_polling() {
    let dir = tempfile::tempdir().unwrap();
    let rec = Arc::new(Rec::default());
    // endpoint that nothing listens on: live mode must report failure honestly
    let mut cfg = EngineConfig::new(dir.path().to_path_buf());
    cfg.endpoints = Some(vec![Endpoint { id: ProviderId::AdsbFi, template: "http://127.0.0.1:9/{lat}/{lon}/{nm}".into() }]);
    cfg.poller = PollerConfig { timeout: Duration::from_millis(300), min_gap: Duration::from_millis(5), ..Default::default() };
    let h = Engine::spawn(cfg, rec.clone());
    let mut s = Settings::default();
    s.home = Some(Home { lat: 51.0, lon: 0.0 });
    h.set_settings(s);
    tokio::time::sleep(Duration::from_millis(900)).await;
    let saved = overhead_core::settings::load(dir.path());
    assert_eq!(saved.home, Some(Home { lat: 51.0, lon: 0.0 }));
    let snaps = rec.snaps.lock().unwrap().clone();
    let last = snaps.last().unwrap();
    assert_eq!(last.source, SourceKind::Live);
    assert!(!last.home_is_demo);
    assert!(last.aircraft.is_empty(), "no fake data in live mode");
    assert_eq!(last.link, LinkState::Unavailable);
    h.shutdown();
}
