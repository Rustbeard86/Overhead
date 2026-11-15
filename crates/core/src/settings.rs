//! User settings, persisted as JSON in the app config directory.

use crate::model::ProviderId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Home {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DistUnit { Nm, Km, Mi }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AltUnit { Ft, M }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme { Phosphor, Amber, Daylight }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Units {
    pub dist: DistUnit,
    pub alt: AltUnit,
}
impl Default for Units {
    fn default() -> Self { Self { dist: DistUnit::Nm, alt: AltUnit::Ft } }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct QuietHours {
    pub enabled: bool,
    /// Minutes after local midnight.
    pub start_min: u32,
    pub end_min: u32,
}
impl Default for QuietHours {
    fn default() -> Self { Self { enabled: false, start_min: 22 * 60, end_min: 7 * 60 } }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AlertRules {
    pub flyover: bool,
    pub flyover_km: f64,
    pub flyover_max_alt_ft: f64,
    pub military: bool,
    pub emergency: bool,
    pub watch_enabled: bool,
    /// Callsign prefixes, registrations, hex codes or ICAO type codes (case-insensitive).
    pub watchlist: Vec<String>,
    pub desktop_notifications: bool,
    pub cooldown_min: f64,
    pub quiet: QuietHours,
}
impl Default for AlertRules {
    fn default() -> Self {
        Self {
            flyover: true,
            flyover_km: 3.0,
            flyover_max_alt_ft: 10_000.0,
            military: true,
            emergency: true,
            watch_enabled: false,
            watchlist: vec![],
            desktop_notifications: false,
            cooldown_min: 15.0,
            quiet: QuietHours::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Privacy {
    pub enabled: bool,
    pub offset_km: f64,
    pub bearing_deg: f64,
}
impl Default for Privacy {
    fn default() -> Self { Self { enabled: false, offset_km: 5.0, bearing_deg: 137.0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WindowPrefs {
    pub always_on_top: bool,
    pub frameless: bool,
    pub kiosk: bool,
    pub tray: bool,
}
impl Default for WindowPrefs {
    fn default() -> Self { Self { always_on_top: false, frameless: false, kiosk: false, tray: true } }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub home: Option<Home>,
    pub view_radius_nm: f64,
    pub poll_interval_s: f64,
    pub trail_min: f64,
    pub provider_order: Vec<ProviderId>,
    pub units: Units,
    pub theme: Theme,
    pub alerts: AlertRules,
    pub privacy: Privacy,
    pub window: WindowPrefs,
    /// Play a replay file instead of polling live providers.
    pub replay_enabled: bool,
    pub replay_file: Option<String>,
    pub first_run_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            home: None,
            view_radius_nm: 40.0,
            poll_interval_s: 3.0,
            trail_min: 5.0,
            provider_order: ProviderId::default_order(),
            units: Units::default(),
            theme: Theme::Phosphor,
            alerts: AlertRules::default(),
            privacy: Privacy::default(),
            window: WindowPrefs::default(),
            replay_enabled: false,
            replay_file: None,
            first_run_done: false,
        }
    }
}

impl Settings {
    /// Clamp values to safe ranges. Poll interval never goes under 2 s.
    pub fn sanitize(&mut self) {
        self.view_radius_nm = self.view_radius_nm.clamp(5.0, 240.0);
        self.poll_interval_s = self.poll_interval_s.clamp(2.0, 60.0);
        self.trail_min = self.trail_min.clamp(1.0, 15.0);
        self.alerts.flyover_km = self.alerts.flyover_km.clamp(0.2, 50.0);
        self.alerts.cooldown_min = self.alerts.cooldown_min.clamp(0.0, 1440.0);
        self.privacy.offset_km = self.privacy.offset_km.clamp(0.0, 50.0);
        self.privacy.bearing_deg = self.privacy.bearing_deg.rem_euclid(360.0);
        self.alerts.quiet.start_min %= 1440;
        self.alerts.quiet.end_min %= 1440;
        if let Some(h) = self.home {
            if !(-90.0..=90.0).contains(&h.lat) || !(-180.0..=180.0).contains(&h.lon) || !h.lat.is_finite() || !h.lon.is_finite() {
                self.home = None;
            }
        }
        // de-duplicate provider order, keep it non-empty
        let mut seen = Vec::new();
        for p in self.provider_order.drain(..) {
            if !seen.contains(&p) {
                seen.push(p);
            }
        }
        if seen.is_empty() {
            seen = ProviderId::default_order();
        }
        self.provider_order = seen;
    }

    /// Radius to request from providers: view radius + 10 nm, padded for the privacy offset.
    pub fn query_radius_nm(&self) -> f64 {
        let mut r = self.view_radius_nm + 10.0;
        if self.privacy.enabled {
            r += self.privacy.offset_km * 1000.0 / crate::geo::NM_M;
        }
        r.min(250.0)
    }
}

pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

pub fn load(dir: &Path) -> Settings {
    let mut s: Settings = std::fs::read_to_string(settings_path(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    s.sanitize();
    s
}

pub fn save(dir: &Path, s: &Settings) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join("settings.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s).map_err(std::io::Error::other)?)?;
    std::fs::rename(tmp, settings_path(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_defaults() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load(d.path()), {
            let mut s = Settings::default();
            s.sanitize();
            s
        });
        let mut s = Settings::default();
        s.home = Some(Home { lat: 51.5, lon: -0.1 });
        s.view_radius_nm = 60.0;
        s.theme = Theme::Amber;
        save(d.path(), &s).unwrap();
        assert_eq!(load(d.path()), s);
    }

    #[test]
    fn partial_and_corrupt_files_are_tolerated() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(settings_path(d.path()), r#"{"view_radius_nm": 25.0}"#).unwrap();
        let s = load(d.path());
        assert_eq!(s.view_radius_nm, 25.0);
        assert_eq!(s.poll_interval_s, 3.0);
        std::fs::write(settings_path(d.path()), "{{{{ nope").unwrap();
        assert_eq!(load(d.path()).view_radius_nm, 40.0);
    }

    #[test]
    fn poll_never_under_two_seconds() {
        let mut s = Settings::default();
        s.poll_interval_s = 0.1;
        s.sanitize();
        assert_eq!(s.poll_interval_s, 2.0);
    }

    #[test]
    fn query_radius_and_privacy_padding() {
        let mut s = Settings::default();
        assert_eq!(s.query_radius_nm(), 50.0);
        s.privacy.enabled = true;
        s.privacy.offset_km = 1.852 * 5.0;
        assert!((s.query_radius_nm() - 55.0).abs() < 1e-9);
        s.view_radius_nm = 240.0;
        s.sanitize();
        assert_eq!(s.query_radius_nm(), 250.0);
    }

    #[test]
    fn bad_home_dropped_and_providers_deduped() {
        let mut s = Settings::default();
        s.home = Some(Home { lat: 200.0, lon: 0.0 });
        s.provider_order = vec![ProviderId::AdsbFi, ProviderId::AdsbFi];
        s.sanitize();
        assert!(s.home.is_none());
        assert_eq!(s.provider_order, vec![ProviderId::AdsbFi]);
        s.provider_order.clear();
        s.sanitize();
        assert!(!s.provider_order.is_empty());
    }
}
