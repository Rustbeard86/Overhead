//! The engine: merges provider data into tracks, computes derived values
//! (distance, bearing, elevation, closest approach, score), runs alerts,
//! persists sightings and emits `snapshot` / `enrichment` / `alert` events.
//!
//! `EngineCore` is a synchronous, deterministic state machine (testable with
//! synthetic traffic). `Engine` wraps it with the async poll loop.

use crate::alerts::{AlertEngine, AlertEvent, AlertInput};
use crate::enrich::{NoRoutes, Photo, PhotoCfg, PhotoLookup, RouteLookup, Tables};
use crate::geo::{self, closest_approach, elevation_deg, enu, enu_inverse, slant_m, velocity_enu, LatLon, FT_M};
use crate::model::{Aircraft, Category, ParsedResponse, ProviderId};
use crate::poller::{Endpoint, FetchError, HealthState, Poller, PollerConfig, ProviderStatus};
use crate::replay::ReplaySource;
use crate::score::{score, FeaturedSelector, ScoreInput, ScoreParams};
use crate::settings::{self, Home, Settings};
use crate::store::{Sighting, Store, TodayData, TodayStats};
use crate::synth::DEMO_HOME;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

pub const STALE_S: f64 = 60.0;
pub const REMOVE_S: f64 = 90.0;
const MAX_TRAIL_POINTS: usize = 90;
const VISIT_GAP_S: f64 = 600.0;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Badge { Military, Emergency, Heavy, Helicopter, Rare, Watch }

#[derive(Clone, Debug, Serialize)]
pub struct AircraftView {
    pub hex: String,
    pub callsign: Option<String>,
    pub registration: Option<String>,
    pub type_code: Option<String>,
    pub type_name: Option<String>,
    pub airline: Option<String>,
    pub airline_color: Option<String>,
    pub category: Category,
    pub lat: f64,
    pub lon: f64,
    /// Position of the last fix, ENU meters from home.
    pub e_m: f64,
    pub n_m: f64,
    /// Ground velocity (ENU, m/s) for dead reckoning; 0 when unknown or on the ground.
    pub ve_ms: f64,
    pub vn_ms: f64,
    pub alt_ft: Option<f64>,
    pub alt_geom_ft: Option<f64>,
    pub on_ground: bool,
    pub gs_kt: Option<f64>,
    pub track_deg: Option<f64>,
    pub vrate_fpm: Option<f64>,
    pub squawk: Option<String>,
    /// Age of the fix at snapshot time, seconds.
    pub seen_pos_s: f64,
    pub dist_km: f64,
    pub bearing_deg: f64,
    pub elev_deg: Option<f64>,
    pub slant_km: f64,
    /// Closest approach, only reported when it is 0..15 min ahead.
    pub ca_t_s: Option<f64>,
    pub ca_d_km: Option<f64>,
    pub score: f64,
    pub badges: Vec<Badge>,
    pub stale: bool,
    pub in_view: bool,
    /// [east_m, north_m, age_s, alt_ft] oldest first.
    pub trail: Vec<[f64; 4]>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind { Live, Replay, Demo }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LinkState {
    /// Nothing received yet.
    Connecting,
    Live,
    /// Last poll failed but recent data is still being drawn.
    Degraded,
    /// No data for over 60 s.
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LastSeen {
    pub hex: String,
    pub callsign: Option<String>,
    pub type_code: Option<String>,
    pub t_ms: i64,
    pub min_dist_km: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub seq: u64,
    pub t_ms: i64,
    pub source: SourceKind,
    pub link: LinkState,
    pub provider: Option<String>,
    pub data_age_s: Option<f64>,
    pub providers: Vec<ProviderStatus>,
    pub home: Option<Home>,
    pub home_is_demo: bool,
    pub view_radius_nm: f64,
    pub aircraft: Vec<AircraftView>,
    pub featured: Option<String>,
    pub pinned: Option<String>,
    pub last_seen: Option<LastSeen>,
    pub today: TodayStats,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnrichmentEvent {
    pub hex: String,
    pub photo: Option<Photo>,
}

pub trait Emitter: Send + Sync + 'static {
    fn snapshot(&self, s: &Snapshot);
    fn enrichment(&self, e: &EnrichmentEvent);
    fn alert(&self, a: &AlertEvent);
}

// ------------------------------------------------------------------ core

#[derive(Clone)]
struct TrailPt {
    t: f64,
    e: f64,
    n: f64,
    alt_ft: f64,
    lat: f64,
    lon: f64,
}

struct Track {
    ac: Aircraft,
    fix_time: f64,
    trail: VecDeque<TrailPt>,
    /// Sighting row for the current visit.
    row: Option<Sighting>,
    row_dirty_at: f64,
}

pub struct EngineCore {
    pub settings: Settings,
    tables: Tables,
    tracks: HashMap<String, Track>,
    selector: FeaturedSelector,
    pub pinned: Option<String>,
    alerts: AlertEngine,
    pub params: ScoreParams,
    seq: u64,
    pub source: SourceKind,
    pub providers: Vec<ProviderStatus>,
    pub active_provider: Option<String>,
    last_ok_unix: Option<f64>,
    last_poll_failed: bool,
    last_seen: Option<LastSeen>,
    today: TodayStats,
    today_at: f64,
    /// Effective home (real or demo).
    home: LatLon,
    home_is_demo: bool,
}

impl EngineCore {
    pub fn new(settings: Settings, source: SourceKind) -> Self {
        let mut c = Self {
            settings,
            tables: Tables::load(),
            tracks: HashMap::new(),
            selector: FeaturedSelector::default(),
            pinned: None,
            alerts: AlertEngine::default(),
            params: ScoreParams::default(),
            seq: 0,
            source,
            providers: vec![],
            active_provider: None,
            last_ok_unix: None,
            last_poll_failed: false,
            last_seen: None,
            today: TodayStats::default(),
            today_at: f64::MIN,
            home: DEMO_HOME,
            home_is_demo: true,
        };
        c.refresh_home(None);
        c
    }

    /// Home = configured home, else the replay file's/demo centre.
    pub fn refresh_home(&mut self, fallback: Option<LatLon>) {
        match self.settings.home {
            Some(h) => {
                self.home = LatLon::new(h.lat, h.lon);
                self.home_is_demo = false;
            }
            None => {
                self.home = fallback.unwrap_or(DEMO_HOME);
                self.home_is_demo = true;
            }
        }
    }

    pub fn home(&self) -> LatLon {
        self.home
    }

    pub fn set_settings(&mut self, s: Settings, fallback: Option<LatLon>) {
        let home_changed = s.home != self.settings.home;
        self.settings = s;
        self.refresh_home(fallback);
        if home_changed {
            self.tracks.clear();
            self.selector = FeaturedSelector::default();
            self.pinned = None;
        }
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Merge one provider response taken at wall-clock `now` (unix seconds).
    pub fn ingest(&mut self, resp: &ParsedResponse, now: f64) {
        for a in &resp.aircraft {
            let fix = now - a.seen_pos_s;
            let home = self.home;
            let (e, n) = enu(home, LatLon::new(a.lat, a.lon));
            let trail_s = self.settings.trail_min * 60.0;
            let t = self.tracks.entry(a.hex.clone()).or_insert_with(|| Track {
                ac: a.clone(),
                fix_time: f64::MIN,
                trail: VecDeque::new(),
                row: None,
                row_dirty_at: f64::MIN,
            });
            let newer = fix > t.fix_time + 0.01;
            if newer {
                t.fix_time = fix;
                t.ac = a.clone();
                let moved = t.trail.back().map(|p| ((p.e - e).powi(2) + (p.n - n).powi(2)).sqrt() >= 80.0 || fix - p.t >= 8.0).unwrap_or(true);
                let same = t.trail.back().map(|p| p.lat == a.lat && p.lon == a.lon).unwrap_or(false);
                if moved && !same {
                    t.trail.push_back(TrailPt { t: fix, e, n, alt_ft: a.altitude_ft().unwrap_or(0.0), lat: a.lat, lon: a.lon });
                }
            } else {
                // older/same fix: refresh identity fields only
                if t.ac.callsign.is_none() { t.ac.callsign = a.callsign.clone(); }
                if t.ac.registration.is_none() { t.ac.registration = a.registration.clone(); }
                if t.ac.type_code.is_none() { t.ac.type_code = a.type_code.clone(); }
            }
            while t.trail.front().map(|p| now - p.t > trail_s).unwrap_or(false) {
                t.trail.pop_front();
            }
        }
        self.last_ok_unix = Some(now);
        self.last_poll_failed = false;
        self.expire(now);
    }

    pub fn note_failure(&mut self, now: f64) {
        self.last_poll_failed = true;
        self.expire(now);
    }

    fn expire(&mut self, now: f64) {
        self.tracks.retain(|_, t| now - t.fix_time <= REMOVE_S);
    }

    pub fn set_today(&mut self, stats: TodayStats, last: Option<LastSeen>, now: f64) {
        self.today = stats;
        self.today_at = now;
        if let Some(l) = last {
            self.last_seen = Some(l);
        }
    }

    pub fn today_stale(&self, now: f64) -> bool {
        now - self.today_at > 15.0
    }

    fn link(&self, now: f64) -> LinkState {
        match self.last_ok_unix {
            None => {
                if self.last_poll_failed { LinkState::Unavailable } else { LinkState::Connecting }
            }
            Some(t) => {
                let age = now - t;
                if age > STALE_S {
                    LinkState::Unavailable
                } else if self.last_poll_failed {
                    LinkState::Degraded
                } else {
                    LinkState::Live
                }
            }
        }
    }

    fn badges_for(&self, a: &Aircraft, cat: Category, watch: bool) -> Vec<Badge> {
        let mut b = Vec::new();
        if a.is_military() || cat == Category::Military { b.push(Badge::Military); }
        if a.emergency_squawk().is_some() { b.push(Badge::Emergency); }
        if self.tables.is_heavy(a, cat) { b.push(Badge::Heavy); }
        if cat == Category::Helicopter { b.push(Badge::Helicopter); }
        if self.tables.is_rare(a) { b.push(Badge::Rare); }
        if watch { b.push(Badge::Watch); }
        b
    }

    fn watch_hit(&self, a: &Aircraft) -> bool {
        if !self.settings.alerts.watch_enabled {
            return false;
        }
        self.settings.alerts.watchlist.iter().any(|w| {
            let w = w.trim().to_lowercase();
            !w.is_empty()
                && (a.hex == w
                    || a.registration.as_deref().map(|r| r.to_lowercase() == w).unwrap_or(false)
                    || a.type_code.as_deref().map(|t| t.to_lowercase() == w).unwrap_or(false)
                    || a.callsign.as_deref().map(|c| c.to_lowercase().starts_with(&w)).unwrap_or(false))
        })
    }

    /// Build the full snapshot at wall-clock `now`. `mono` is a monotonic seconds
    /// clock used only for the featured-plane hysteresis.
    pub fn snapshot(&mut self, now: f64, mono: f64) -> (Snapshot, Vec<AlertEvent>) {
        let home = self.home;
        let radius_m = self.settings.view_radius_nm * geo::NM_M;
        let margin_m = 10.0 * geo::NM_M;
        let mut views: Vec<AircraftView> = Vec::with_capacity(self.tracks.len());

        for t in self.tracks.values() {
            let a = &t.ac;
            let age = (now - t.fix_time).max(0.0);
            let (e0, n0) = enu(home, LatLon::new(a.lat, a.lon));
            let moving = !a.on_ground && a.ground_speed_kt.is_some() && a.track_deg.is_some();
            let (ve, vn) = if moving { velocity_enu(a.ground_speed_kt.unwrap(), a.track_deg.unwrap()) } else { (0.0, 0.0) };
            // extrapolate to `now` (capped) for derived values
            let dt = age.min(STALE_S);
            let (e, n) = (e0 + ve * dt, n0 + vn * dt);
            let dist_m = (e * e + n * n).sqrt();
            if dist_m > radius_m + margin_m {
                continue;
            }
            let bearing = if dist_m < 1.0 { 0.0 } else { (e.atan2(n).to_degrees() + 360.0) % 360.0 };
            let alt_ft = a.altitude_ft();
            let h_m = alt_ft.unwrap_or(0.0) * FT_M;
            let elev = if a.on_ground { None } else { alt_ft.map(|_| elevation_deg(dist_m, h_m)) };
            let slant = slant_m(dist_m, h_m) / 1000.0;
            let ca = if moving { closest_approach((e, n), (ve, vn)) } else { None };
            let ca_view = ca.filter(|c| (0.0..=900.0).contains(&c.t_s));
            let cat = self.tables.category(a);
            let watch = self.watch_hit(a);
            let badges = self.badges_for(a, cat, watch);
            let sc = score(
                &ScoreInput {
                    slant_km: slant,
                    approach: ca.map(|c| (c.t_s, c.d_min_m / 1000.0)),
                    alt_ft,
                    on_ground: a.on_ground,
                    military: badges.contains(&Badge::Military),
                    emergency: badges.contains(&Badge::Emergency),
                    heavy: badges.contains(&Badge::Heavy),
                    helicopter: badges.contains(&Badge::Helicopter),
                    rare: badges.contains(&Badge::Rare),
                    watch,
                },
                &self.params,
            );
            // trail, thinned
            let stride = (t.trail.len() / MAX_TRAIL_POINTS).max(1);
            let mut trail: Vec<[f64; 4]> = t
                .trail
                .iter()
                .step_by(stride)
                .map(|p| [p.e.round(), p.n.round(), (now - p.t).max(0.0).round(), p.alt_ft.round()])
                .collect();
            if let Some(last) = t.trail.back() {
                let l = [last.e.round(), last.n.round(), (now - last.t).max(0.0).round(), last.alt_ft.round()];
                if trail.last() != Some(&l) { trail.push(l); }
            }
            let airline = a.callsign.as_deref().and_then(|c| self.tables.airline_for_callsign(c));
            views.push(AircraftView {
                hex: a.hex.clone(),
                callsign: a.callsign.clone(),
                registration: a.registration.clone(),
                type_code: a.type_code.clone(),
                type_name: self.tables.type_name(a),
                airline: airline.map(|x| x.n.clone()),
                airline_color: airline.and_then(|x| x.c.clone()),
                category: cat,
                lat: a.lat,
                lon: a.lon,
                e_m: e0.round(),
                n_m: n0.round(),
                ve_ms: (ve * 10.0).round() / 10.0,
                vn_ms: (vn * 10.0).round() / 10.0,
                alt_ft: a.alt_baro_ft.or(a.alt_geom_ft),
                alt_geom_ft: a.alt_geom_ft,
                on_ground: a.on_ground,
                gs_kt: a.ground_speed_kt,
                track_deg: a.track_deg,
                vrate_fpm: a.vertical_rate_fpm,
                squawk: a.squawk.clone(),
                seen_pos_s: (age * 10.0).round() / 10.0,
                dist_km: dist_m / 1000.0,
                bearing_deg: bearing,
                elev_deg: elev,
                slant_km: slant,
                ca_t_s: ca_view.map(|c| c.t_s),
                ca_d_km: ca_view.map(|c| c.d_min_m / 1000.0),
                score: sc,
                badges,
                stale: age > STALE_S,
                in_view: dist_m <= radius_m,
                trail,
            });
        }
        views.sort_by(|a, b| a.dist_km.partial_cmp(&b.dist_km).unwrap_or(std::cmp::Ordering::Equal));

        // featured plane
        let scores: Vec<(String, f64)> = views.iter().filter(|v| v.in_view && !v.stale).map(|v| (v.hex.clone(), v.score)).collect();
        let sel = self.selector.update(mono, &scores, &self.params);
        if let Some(p) = &self.pinned {
            if !views.iter().any(|v| &v.hex == p && v.in_view) {
                self.pinned = None; // out of range: unpin
            }
        }
        let featured = self.pinned.clone().or(sel);

        // alerts + sightings
        let alert_in: Vec<AlertInput> = views
            .iter()
            .filter(|v| v.in_view && !v.stale)
            .map(|v| AlertInput {
                hex: v.hex.clone(),
                callsign: v.callsign.clone(),
                registration: v.registration.clone(),
                type_code: v.type_code.clone(),
                dist_km: v.dist_km,
                alt_ft: v.alt_ft,
                on_ground: v.on_ground,
                military: v.badges.contains(&Badge::Military),
                squawk: v.squawk.clone(),
            })
            .collect();
        let minute = local_minute_of_day();
        let alerts = self.alerts.evaluate(&self.settings.alerts, mono, (now * 1000.0) as i64, minute, &alert_in);

        if let Some(best) = views.iter().filter(|v| v.in_view && !v.on_ground && !v.stale).min_by(|a, b| a.dist_km.partial_cmp(&b.dist_km).unwrap_or(std::cmp::Ordering::Equal)) {
            self.last_seen = Some(LastSeen {
                hex: best.hex.clone(),
                callsign: best.callsign.clone(),
                type_code: best.type_code.clone(),
                t_ms: (now * 1000.0) as i64,
                min_dist_km: best.dist_km,
            });
        }

        let data_age = self.last_ok_unix.map(|t| (now - t).max(0.0));
        self.seq += 1;
        let snap = Snapshot {
            seq: self.seq,
            t_ms: (now * 1000.0) as i64,
            source: self.source,
            link: self.link(now),
            provider: self.active_provider.clone(),
            data_age_s: data_age,
            providers: self.providers.clone(),
            home: Some(Home { lat: home.lat, lon: home.lon }),
            home_is_demo: self.home_is_demo,
            view_radius_nm: self.settings.view_radius_nm,
            aircraft: views,
            featured,
            pinned: self.pinned.clone(),
            last_seen: self.last_seen.clone(),
            today: self.today.clone(),
        };
        (snap, alerts)
    }

    /// Sightings to persist for this poll (in-view, airborne planes). Returns rows to upsert;
    /// feed ids back with [`EngineCore::set_row_id`].
    pub fn sightings_due(&mut self, snap: &Snapshot, now: f64) -> Vec<(String, Sighting)> {
        let mut out = Vec::new();
        for v in snap.aircraft.iter().filter(|v| v.in_view && !v.on_ground && !v.stale) {
            let Some(t) = self.tracks.get_mut(&v.hex) else { continue };
            let now_ms = (now * 1000.0) as i64;
            // new visit after a long gap
            if let Some(r) = &t.row {
                if now_ms - r.last_seen_ms > (VISIT_GAP_S * 1000.0) as i64 {
                    t.row = None;
                }
            }
            let mut dirty = false;
            let row = t.row.get_or_insert_with(|| {
                dirty = true;
                Sighting {
                    id: None,
                    hex: v.hex.clone(),
                    callsign: v.callsign.clone(),
                    registration: v.registration.clone(),
                    type_code: v.type_code.clone(),
                    first_seen_ms: now_ms,
                    last_seen_ms: now_ms,
                    min_dist_km: v.dist_km,
                    closest_ms: now_ms,
                    min_alt_ft: v.alt_ft,
                }
            });
            if v.dist_km < row.min_dist_km {
                row.min_dist_km = v.dist_km;
                row.closest_ms = now_ms;
                dirty = true;
            }
            if let Some(alt) = v.alt_ft {
                if row.min_alt_ft.map(|m| alt < m).unwrap_or(true) {
                    row.min_alt_ft = Some(alt);
                    dirty = true;
                }
            }
            if row.callsign.is_none() && v.callsign.is_some() {
                row.callsign = v.callsign.clone();
                dirty = true;
            }
            row.last_seen_ms = now_ms;
            if dirty || now - t.row_dirty_at > 30.0 {
                t.row_dirty_at = now;
                out.push((v.hex.clone(), row.clone()));
            }
        }
        out
    }

    pub fn set_row_id(&mut self, hex: &str, id: i64) {
        if let Some(r) = self.tracks.get_mut(hex).and_then(|t| t.row.as_mut()) {
            r.id = Some(id);
        }
    }

    /// Registration and callsign for enrichment lookups.
    pub fn ident(&self, hex: &str) -> Option<(Option<String>, Option<String>)> {
        self.tracks.get(hex).map(|t| (t.ac.registration.clone(), t.ac.callsign.clone()))
    }

    pub fn track_latlon(&self, hex: &str) -> Option<LatLon> {
        self.tracks.get(hex).map(|t| LatLon::new(t.ac.lat, t.ac.lon))
    }
}

fn local_minute_of_day() -> u32 {
    use chrono::Timelike;
    let n = chrono::Local::now();
    n.hour() * 60 + n.minute()
}

pub fn unix_now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

// ------------------------------------------------------------------ async runner

pub struct EngineConfig {
    pub data_dir: PathBuf,
    pub replay_file: Option<PathBuf>,
    pub endpoints: Option<Vec<Endpoint>>,
    pub poller: PollerConfig,
    pub photo: PhotoCfg,
}

impl EngineConfig {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir, replay_file: None, endpoints: None, poller: PollerConfig::default(), photo: PhotoCfg::default() }
    }
}

enum Cmd {
    SetSettings(Box<Settings>),
    Pin(Option<String>),
    Select(String),
    Shutdown,
}

type PollResult = (Poller, Result<crate::poller::PollSuccess, Vec<(ProviderId, FetchError)>>);

#[derive(Clone)]
pub struct EngineHandle {
    tx: mpsc::UnboundedSender<Cmd>,
    store: Arc<Store>,
    settings: Arc<std::sync::Mutex<Settings>>,
    data_dir: PathBuf,
}

impl EngineHandle {
    pub fn set_settings(&self, s: Settings) {
        let _ = self.tx.send(Cmd::SetSettings(Box::new(s)));
    }
    pub fn pin(&self, hex: Option<String>) {
        let _ = self.tx.send(Cmd::Pin(hex));
    }
    /// Ask for enrichment (photo) for a clicked/hovered plane.
    pub fn select(&self, hex: String) {
        let _ = self.tx.send(Cmd::Select(hex));
    }
    pub fn shutdown(&self) {
        let _ = self.tx.send(Cmd::Shutdown);
    }
    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
    pub fn today(&self) -> Result<TodayData, String> {
        self.store.today((unix_now() * 1000.0) as i64).map_err(|e| e.to_string())
    }
    pub fn export_csv(&self) -> Result<String, String> {
        self.store.export_csv(0).map_err(|e| e.to_string())
    }
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

pub struct Engine;

impl Engine {
    pub fn spawn(cfg: EngineConfig, emitter: Arc<dyn Emitter>) -> EngineHandle {
        let settings = settings::load(&cfg.data_dir);
        let store = Arc::new(Store::open(&cfg.data_dir.join("overhead.db")).unwrap_or_else(|e| {
            tracing::error!(error = %e, "could not open sighting database; using memory");
            Store::in_memory().expect("in-memory sqlite")
        }));
        let _ = store.prune((unix_now() * 1000.0) as i64);
        let (tx, rx) = mpsc::unbounded_channel();
        let shared = Arc::new(std::sync::Mutex::new(settings.clone()));
        let handle = EngineHandle { tx, store: store.clone(), settings: shared.clone(), data_dir: cfg.data_dir.clone() };
        tokio::spawn(run(cfg, settings, store, shared, emitter, rx));
        handle
    }
}

fn endpoints_for(settings: &Settings, over: &Option<Vec<Endpoint>>) -> Vec<Endpoint> {
    if let Some(o) = over {
        return o.clone();
    }
    settings.provider_order.iter().map(|id| Endpoint::default_for(*id)).collect()
}

async fn run(
    cfg: EngineConfig,
    settings: Settings,
    store: Arc<Store>,
    shared: Arc<std::sync::Mutex<Settings>>,
    emitter: Arc<dyn Emitter>,
    mut rx: mpsc::UnboundedReceiver<Cmd>,
) {
    let started = Instant::now();
    let mut replay_override = cfg.replay_file.clone();
    let mut replay: Option<ReplaySource> = None;
    let mut replay_started = Instant::now();

    let load_replay = |settings: &Settings, over: &Option<PathBuf>| -> Option<ReplaySource> {
        if let Some(p) = over.as_ref().cloned().or_else(|| settings.replay_file.as_ref().filter(|_| settings.replay_enabled).map(PathBuf::from)) {
            match ReplaySource::load(&p) {
                Ok(r) => return Some(r),
                Err(e) => tracing::error!(error = %e, "replay file failed to load; falling back to demo"),
            }
            return Some(ReplaySource::demo());
        }
        if settings.home.is_none() {
            return Some(ReplaySource::demo());
        }
        None
    };
    replay = load_replay(&settings, &replay_override).or(replay);

    let kind_of = |r: &Option<ReplaySource>, over: &Option<PathBuf>, s: &Settings| match r {
        None => SourceKind::Live,
        Some(ReplaySource::Demo { .. }) if over.is_none() && !(s.replay_enabled && s.replay_file.is_some()) => SourceKind::Demo,
        Some(_) => SourceKind::Replay,
    };

    let mut core = EngineCore::new(settings.clone(), kind_of(&replay, &replay_override, &settings));
    core.refresh_home(replay.as_ref().map(|r| r.default_home()));
    let tables_photo = Arc::new(PhotoLookup::new(cfg.photo.clone(), Some(cfg.data_dir.clone())));
    let routes: Arc<dyn RouteLookup> = Arc::new(NoRoutes);
    let _ = &routes;

    let mut poller = Some(Poller::new(cfg.poller.clone(), endpoints_for(&settings, &cfg.endpoints)));
    let (poll_tx, mut poll_rx) = mpsc::unbounded_channel::<PollResult>();
    let (enr_tx, mut enr_rx) = mpsc::unbounded_channel::<EnrichmentEvent>();
    let mut enrichments: HashMap<String, Option<Photo>> = HashMap::new();
    let mut enriching: std::collections::HashSet<String> = Default::default();
    let mut polling = false;
    let mut next_poll = Instant::now();
    let mut pending_endpoints: Option<Vec<Endpoint>> = None;
    let mut last_featured: Option<String> = None;

    macro_rules! emit_snapshot {
        () => {{
            let now = unix_now();
            let mono = started.elapsed().as_secs_f64();
            if core.today_stale(now) {
                let t0 = crate::store::local_day_start_ms((now * 1000.0) as i64);
                let stats = store.stats_since(t0).unwrap_or_default();
                let last = store.last_seen().ok().flatten().map(|s| LastSeen { hex: s.hex, callsign: s.callsign, type_code: s.type_code, t_ms: s.last_seen_ms, min_dist_km: s.min_dist_km });
                core.set_today(stats, last, now);
            }
            let (snap, alerts) = core.snapshot(now, mono);
            let due = core.sightings_due(&snap, now);
            for (hex, row) in due {
                match store.upsert(&row) {
                    Ok(id) => core.set_row_id(&hex, id),
                    Err(e) => tracing::warn!(error = %e, "sighting write failed"),
                }
            }
            emitter.snapshot(&snap);
            for a in &alerts {
                emitter.alert(a);
            }
            // enrichment for the featured plane
            if snap.featured != last_featured {
                last_featured = snap.featured.clone();
            }
            if let Some(h) = &snap.featured {
                request_enrichment(h, &core, &mut enrichments, &mut enriching, &tables_photo, &enr_tx, &*emitter);
            }
        }};
    }

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { break };
                match cmd {
                    Cmd::Shutdown => break,
                    Cmd::Pin(h) => { core.pinned = h.clone(); emit_snapshot!(); }
                    Cmd::Select(h) => {
                        request_enrichment(&h, &core, &mut enrichments, &mut enriching, &tables_photo, &enr_tx, &*emitter);
                    }
                    Cmd::SetSettings(s) => {
                        let mut s = *s;
                        s.sanitize();
                        if let Err(e) = settings::save(&cfg.data_dir, &s) {
                            tracing::warn!(error = %e, "could not save settings");
                        }
                        let radius_changed = s.view_radius_nm != core.settings.view_radius_nm || s.home != core.settings.home || s.privacy != core.settings.privacy || s.replay_enabled != core.settings.replay_enabled || s.replay_file != core.settings.replay_file;
                        let order_changed = s.provider_order != core.settings.provider_order;
                        if s.replay_enabled != core.settings.replay_enabled || s.replay_file != core.settings.replay_file || s.home.is_some() != core.settings.home.is_some() {
                            replay_override = cfg.replay_file.clone();
                            replay = load_replay(&s, &replay_override);
                            replay_started = Instant::now();
                        }
                        core.source = kind_of(&replay, &replay_override, &s);
                        core.set_settings(s.clone(), replay.as_ref().map(|r| r.default_home()));
                        *shared.lock().unwrap() = s.clone();
                        if order_changed {
                            let eps = endpoints_for(&s, &cfg.endpoints);
                            match poller.as_mut() { Some(p) => p.set_endpoints(eps), None => pending_endpoints = Some(eps) }
                        }
                        if radius_changed { next_poll = Instant::now(); }
                        emit_snapshot!();
                    }
                }
            }
            res = poll_rx.recv() => {
                let Some((p, r)) = res else { continue };
                polling = false;
                poller = Some(p);
                if let (Some(p), Some(eps)) = (poller.as_mut(), pending_endpoints.take()) { p.set_endpoints(eps); }
                let now = unix_now();
                match r {
                    Ok(ok) => {
                        core.providers = poller.as_ref().map(|p| p.status()).unwrap_or_default();
                        core.active_provider = Some(ok.provider.name().to_string());
                        core.ingest(&ok.response, now);
                    }
                    Err(_) => {
                        core.providers = poller.as_ref().map(|p| p.status()).unwrap_or_default();
                        core.active_provider = None;
                        core.note_failure(now);
                    }
                }
                emit_snapshot!();
            }
            e = enr_rx.recv() => {
                if let Some(e) = e {
                    enriching.remove(&e.hex);
                    enrichments.insert(e.hex.clone(), e.photo.clone());
                    emitter.enrichment(&e);
                }
            }
            _ = tokio::time::sleep_until(next_poll) => {
                let interval = Duration::from_secs_f64(core.settings.poll_interval_s.max(2.0));
                next_poll = Instant::now() + interval;
                if let Some(src) = &replay {
                    // replay/demo: no network
                    let el = replay_started.elapsed().as_secs_f64();
                    let frame = src.frame_at(el);
                    core.providers = vec![];
                    core.active_provider = Some(match core.source { SourceKind::Demo => "demo".into(), _ => "replay".into() });
                    core.ingest(&frame, unix_now());
                    emit_snapshot!();
                } else if !polling {
                    if core.settings.home.is_none() {
                        emit_snapshot!();
                    } else if let Some(mut p) = poller.take() {
                        polling = true;
                        let home = core.home();
                        let pr = &core.settings.privacy;
                        let q = if pr.enabled { geo::destination(home, pr.bearing_deg, pr.offset_km * 1000.0) } else { home };
                        let nm = core.settings.query_radius_nm();
                        let tx = poll_tx.clone();
                        tokio::spawn(async move {
                            let r = p.poll(q.lat, q.lon, nm).await;
                            let _ = tx.send((p, r));
                        });
                    }
                }
            }
        }
    }
    let _ = (HealthState::Idle, enu_inverse as fn(LatLon, f64, f64) -> LatLon);
}

fn request_enrichment(
    hex: &str,
    core: &EngineCore,
    done: &mut HashMap<String, Option<Photo>>,
    inflight: &mut std::collections::HashSet<String>,
    photos: &Arc<PhotoLookup>,
    tx: &mpsc::UnboundedSender<EnrichmentEvent>,
    emitter: &dyn Emitter,
) {
    if let Some(p) = done.get(hex) {
        // already known: re-send so a newly selected card gets it immediately
        emitter.enrichment(&EnrichmentEvent { hex: hex.to_string(), photo: p.clone() });
        return;
    }
    if inflight.contains(hex) || core.source != SourceKind::Live {
        if core.source != SourceKind::Live {
            // replay/demo never touches the network for photos
            done.insert(hex.to_string(), None);
            emitter.enrichment(&EnrichmentEvent { hex: hex.to_string(), photo: None });
        }
        return;
    }
    let Some((reg, _)) = core.ident(hex) else { return };
    inflight.insert(hex.to_string());
    let (photos, tx, hex) = (photos.clone(), tx.clone(), hex.to_string());
    tokio::spawn(async move {
        let photo = photos.lookup(&hex, reg.as_deref()).await;
        let _ = tx.send(EnrichmentEvent { hex, photo });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{demo_planes, frame_json, named_scenarios};

    fn core_with(planes: &[crate::synth::SynthPlane], t: f64) -> (EngineCore, f64) {
        let mut s = Settings::default();
        s.home = Some(Home { lat: DEMO_HOME.lat, lon: DEMO_HOME.lon });
        let mut c = EngineCore::new(s, SourceKind::Replay);
        let now = 1_800_000_000.0 + t;
        let body = frame_json(DEMO_HOME, planes, t, now);
        let resp = crate::provider::parse_adsbx_v2(&body).unwrap();
        c.ingest(&resp, now);
        (c, now)
    }

    #[test]
    fn overhead_pass_features_that_plane_with_90_degree_elevation() {
        let planes = named_scenarios();
        let (mut c, now) = core_with(&planes[..1], 100.0);
        let (snap, alerts) = c.snapshot(now, 0.0);
        let v = &snap.aircraft[0];
        assert!(v.dist_km < 0.1, "{}", v.dist_km);
        assert!(v.elev_deg.unwrap() > 88.0);
        assert_eq!(snap.featured.as_deref(), Some(v.hex.as_str()));
        // flyover alert: within 3 km and under 10,000 ft (3000 m = 9843 ft)
        assert!(alerts.iter().any(|a| a.kind == crate::alerts::AlertKind::Flyover));
    }

    #[test]
    fn approach_reports_closest_approach_in_window() {
        let planes = named_scenarios();
        let (mut c, now) = core_with(&planes[1..2], 70.0); // offset pass, 30 s before closest
        let (snap, _) = c.snapshot(now, 0.0);
        let v = &snap.aircraft[0];
        let t = v.ca_t_s.unwrap();
        assert!((t - 30.0).abs() < 2.0, "t={t}");
        assert!((v.ca_d_km.unwrap() - 5.0).abs() < 0.1);
        // 20 s later (now past closest approach) -> no approach reported
        let (mut c2, now2) = core_with(&planes[1..2], 140.0);
        let (s2, _) = c2.snapshot(now2, 0.0);
        assert!(s2.aircraft[0].ca_t_s.is_none());
    }

    #[test]
    fn busy_demo_scene_badges_and_features() {
        let planes = demo_planes();
        let (mut c, now) = core_with(&planes, 30.0);
        let (snap, _) = c.snapshot(now, 0.0);
        assert!(snap.aircraft.len() >= 10);
        assert!(snap.aircraft.iter().any(|v| v.badges.contains(&Badge::Emergency)));
        assert!(snap.aircraft.iter().any(|v| v.badges.contains(&Badge::Military)));
        assert!(snap.aircraft.iter().any(|v| v.category == Category::Helicopter));
        assert!(snap.aircraft.iter().any(|v| v.on_ground));
        // sorted by distance, featured is in view
        assert!(snap.aircraft.windows(2).all(|w| w[0].dist_km <= w[1].dist_km));
        let f = snap.featured.unwrap();
        assert!(snap.aircraft.iter().any(|v| v.hex == f && v.in_view));
    }

    #[test]
    fn stale_after_60s_removed_after_90s() {
        let planes = named_scenarios();
        let (mut c, now) = core_with(&planes[..1], 10.0);
        assert!(!c.snapshot(now + 30.0, 0.0).0.aircraft[0].stale);
        assert!(c.snapshot(now + 61.0, 0.0).0.aircraft[0].stale);
        c.note_failure(now + 91.0);
        assert_eq!(c.track_count(), 0);
    }

    #[test]
    fn link_states() {
        let planes = named_scenarios();
        let mut c = EngineCore::new(Settings::default(), SourceKind::Live);
        assert_eq!(c.snapshot(1e9, 0.0).0.link, LinkState::Connecting);
        let (mut c, now) = core_with(&planes[..1], 10.0);
        assert_eq!(c.snapshot(now + 1.0, 0.0).0.link, LinkState::Live);
        c.note_failure(now + 3.0);
        assert_eq!(c.snapshot(now + 4.0, 0.0).0.link, LinkState::Degraded);
        assert_eq!(c.snapshot(now + 70.0, 0.0).0.link, LinkState::Unavailable);
    }

    #[test]
    fn pin_holds_until_out_of_range() {
        let planes = demo_planes();
        let (mut c, now) = core_with(&planes, 30.0);
        let (s, _) = c.snapshot(now, 0.0);
        let far = s.aircraft.iter().rev().find(|v| v.in_view).unwrap().hex.clone();
        c.pinned = Some(far.clone());
        let (s, _) = c.snapshot(now + 1.0, 1.0);
        assert_eq!(s.featured.as_deref(), Some(far.as_str()));
        assert_eq!(s.pinned.as_deref(), Some(far.as_str()));
        c.pinned = Some("nosuch".into());
        let (s, _) = c.snapshot(now + 2.0, 2.0);
        assert!(s.pinned.is_none());
    }

    #[test]
    fn trails_accumulate_and_age() {
        let planes = named_scenarios();
        let mut s = Settings::default();
        s.home = Some(Home { lat: DEMO_HOME.lat, lon: DEMO_HOME.lon });
        let mut c = EngineCore::new(s, SourceKind::Replay);
        for i in 0..20 {
            let t = i as f64 * 3.0;
            let now = 1_800_000_000.0 + t;
            let r = crate::provider::parse_adsbx_v2(&frame_json(DEMO_HOME, &planes[..1], t, now)).unwrap();
            c.ingest(&r, now);
        }
        let (snap, _) = c.snapshot(1_800_000_000.0 + 57.0, 0.0);
        let tr = &snap.aircraft[0].trail;
        assert!(tr.len() >= 10, "{}", tr.len());
        assert!(tr.windows(2).all(|w| w[0][2] >= w[1][2]), "oldest first");
    }

    #[test]
    fn sightings_track_min_distance_and_visit_gap() {
        let planes = named_scenarios();
        let mut s = Settings::default();
        s.home = Some(Home { lat: DEMO_HOME.lat, lon: DEMO_HOME.lon });
        let mut c = EngineCore::new(s, SourceKind::Replay);
        let mut last_row = None;
        for t in [80.0, 90.0, 100.0, 110.0] {
            let now = 1_800_000_000.0 + t;
            let r = crate::provider::parse_adsbx_v2(&frame_json(DEMO_HOME, &planes[..1], t, now)).unwrap();
            c.ingest(&r, now);
            let (snap, _) = c.snapshot(now, t);
            for (_, row) in c.sightings_due(&snap, now) {
                last_row = Some(row);
            }
        }
        let row = last_row.unwrap();
        assert!(row.min_dist_km < 0.5, "{}", row.min_dist_km);
        assert!(row.min_alt_ft.unwrap() > 9000.0);
    }

    #[test]
    fn changing_home_resets_state() {
        let planes = named_scenarios();
        let (mut c, _) = core_with(&planes, 10.0);
        assert!(c.track_count() > 0);
        let mut s = c.settings.clone();
        s.home = Some(Home { lat: 40.0, lon: -74.0 });
        c.set_settings(s, None);
        assert_eq!(c.track_count(), 0);
        assert!(!c.home_is_demo);
    }
}
