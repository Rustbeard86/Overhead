//! Alert rules: flyover, military, emergency squawks, watch-list.
//! Pure and deterministic: time is passed in, so it is unit-testable.

use crate::settings::AlertRules;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AlertKind { Flyover, Military, Emergency, Watch }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AlertEvent {
    pub kind: AlertKind,
    pub hex: String,
    pub callsign: Option<String>,
    pub title: String,
    pub detail: String,
    pub t_ms: i64,
    /// Shown as toast / desktop notification? False during quiet hours.
    pub notify: bool,
}

/// What the alert engine needs to know about a plane.
#[derive(Clone, Debug, Default)]
pub struct AlertInput {
    pub hex: String,
    pub callsign: Option<String>,
    pub registration: Option<String>,
    pub type_code: Option<String>,
    pub dist_km: f64,
    pub alt_ft: Option<f64>,
    pub on_ground: bool,
    pub military: bool,
    pub squawk: Option<String>,
}

#[derive(Default)]
pub struct AlertEngine {
    last: HashMap<(AlertKind, String), f64>,
}

pub fn in_quiet_hours(rules: &AlertRules, minute_of_day: u32) -> bool {
    let q = &rules.quiet;
    if !q.enabled || q.start_min == q.end_min {
        return false;
    }
    if q.start_min < q.end_min {
        (q.start_min..q.end_min).contains(&minute_of_day)
    } else {
        minute_of_day >= q.start_min || minute_of_day < q.end_min
    }
}

fn watch_match(list: &[String], a: &AlertInput) -> Option<String> {
    for w in list {
        let w = w.trim().to_lowercase();
        if w.is_empty() {
            continue;
        }
        let hit = a.hex.to_lowercase() == w
            || a.registration.as_deref().map(|r| r.to_lowercase() == w).unwrap_or(false)
            || a.type_code.as_deref().map(|t| t.to_lowercase() == w).unwrap_or(false)
            || a.callsign.as_deref().map(|c| c.to_lowercase().starts_with(&w)).unwrap_or(false);
        if hit {
            return Some(w.to_uppercase());
        }
    }
    None
}

impl AlertEngine {
    /// `now_s`: monotonic seconds; `now_ms`: wall clock for the event; `minute_of_day`: local time.
    pub fn evaluate(&mut self, rules: &AlertRules, now_s: f64, now_ms: i64, minute_of_day: u32, planes: &[AlertInput]) -> Vec<AlertEvent> {
        let quiet = in_quiet_hours(rules, minute_of_day);
        let cooldown = rules.cooldown_min * 60.0;
        let mut out = Vec::new();
        for a in planes {
            let name = a.callsign.clone().unwrap_or_else(|| a.hex.to_uppercase());
            let mut hits: Vec<(AlertKind, String, String)> = Vec::new();
            if rules.emergency {
                let why = match a.squawk.as_deref() {
                    Some("7500") => Some("Squawk 7500 (hijack)"),
                    Some("7600") => Some("Squawk 7600 (radio failure)"),
                    Some("7700") => Some("Squawk 7700 (general emergency)"),
                    _ => None,
                };
                if let Some(w) = why {
                    hits.push((AlertKind::Emergency, format!("{name}: emergency"), w.to_string()));
                }
            }
            if rules.military && a.military && !a.on_ground {
                hits.push((AlertKind::Military, format!("{name}: military aircraft"), a.type_code.clone().unwrap_or_default()));
            }
            if rules.flyover && !a.on_ground && a.dist_km <= rules.flyover_km {
                if let Some(alt) = a.alt_ft {
                    if alt <= rules.flyover_max_alt_ft {
                        hits.push((AlertKind::Flyover, format!("{name}: flyover"), format!("{:.1} km away at {:.0} ft", a.dist_km, alt)));
                    }
                }
            }
            if rules.watch_enabled {
                if let Some(w) = watch_match(&rules.watchlist, a) {
                    hits.push((AlertKind::Watch, format!("{name}: watch list"), format!("matched {w}")));
                }
            }
            for (kind, title, detail) in hits {
                let key = (kind, a.hex.clone());
                if let Some(t) = self.last.get(&key) {
                    if now_s - t < cooldown {
                        continue;
                    }
                }
                self.last.insert(key, now_s);
                // Emergencies stay audible during quiet hours.
                let notify = !quiet || kind == AlertKind::Emergency;
                out.push(AlertEvent { kind, hex: a.hex.clone(), callsign: a.callsign.clone(), title, detail, t_ms: now_ms, notify });
            }
        }
        // forget very old cooldown entries
        if self.last.len() > 2000 {
            self.last.retain(|_, t| now_s - *t < cooldown.max(60.0));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::QuietHours;

    fn plane(hex: &str) -> AlertInput {
        AlertInput { hex: hex.into(), callsign: Some("TST1".into()), dist_km: 20.0, alt_ft: Some(30_000.0), ..Default::default() }
    }

    #[test]
    fn flyover_thresholds() {
        let r = AlertRules::default();
        let mut e = AlertEngine::default();
        let mut p = plane("a");
        p.dist_km = 2.9;
        p.alt_ft = Some(9_900.0);
        assert_eq!(e.evaluate(&r, 0.0, 0, 600, &[p.clone()]).len(), 1);
        let mut e = AlertEngine::default();
        p.dist_km = 3.1;
        assert!(e.evaluate(&r, 0.0, 0, 600, &[p.clone()]).is_empty());
        p.dist_km = 2.0;
        p.alt_ft = Some(10_500.0);
        assert!(e.evaluate(&r, 0.0, 0, 600, &[p.clone()]).is_empty());
        p.alt_ft = Some(5_000.0);
        p.on_ground = true;
        assert!(e.evaluate(&r, 0.0, 0, 600, &[p]).is_empty());
    }

    #[test]
    fn cooldown_is_per_rule_and_plane() {
        let r = AlertRules::default();
        let mut e = AlertEngine::default();
        let mut p = plane("a");
        p.dist_km = 1.0;
        p.alt_ft = Some(3000.0);
        p.military = true;
        let first = e.evaluate(&r, 0.0, 0, 600, &[p.clone()]);
        assert_eq!(first.len(), 2); // flyover + military
        assert!(e.evaluate(&r, 60.0, 0, 600, &[p.clone()]).is_empty());
        // other plane is independent
        let mut q = p.clone();
        q.hex = "b".into();
        assert_eq!(e.evaluate(&r, 61.0, 0, 600, &[q]).len(), 2);
        // after cooldown (15 min) it fires again
        assert_eq!(e.evaluate(&r, 15.0 * 60.0 + 1.0, 0, 600, &[p]).len(), 2);
    }

    #[test]
    fn emergency_squawks() {
        let r = AlertRules::default();
        for (sq, text) in [("7500", "hijack"), ("7600", "radio"), ("7700", "emergency")] {
            let mut e = AlertEngine::default();
            let mut p = plane("a");
            p.squawk = Some(sq.into());
            let ev = e.evaluate(&r, 0.0, 0, 600, &[p]);
            assert_eq!(ev.len(), 1);
            assert_eq!(ev[0].kind, AlertKind::Emergency);
            assert!(ev[0].detail.contains(text));
        }
        let mut e = AlertEngine::default();
        let mut p = plane("a");
        p.squawk = Some("1200".into());
        assert!(e.evaluate(&r, 0.0, 0, 600, &[p]).is_empty());
    }

    #[test]
    fn watchlist_matches_callsign_prefix_type_reg_hex() {
        let mut r = AlertRules::default();
        r.watch_enabled = true;
        r.watchlist = vec!["BAW".into(), "a388".into(), "g-ofoo".into(), "ABC123".into()];
        let mk = |cs: &str, t: &str, reg: &str, hex: &str| AlertInput {
            hex: hex.into(),
            callsign: Some(cs.into()),
            type_code: Some(t.into()),
            registration: Some(reg.into()),
            dist_km: 30.0,
            alt_ft: Some(30_000.0),
            ..Default::default()
        };
        let mut e = AlertEngine::default();
        assert_eq!(e.evaluate(&r, 0.0, 0, 0, &[mk("BAW117", "A320", "G-X", "1")]).len(), 1);
        assert_eq!(e.evaluate(&r, 0.0, 0, 0, &[mk("XYZ1", "A388", "G-X", "2")]).len(), 1);
        assert_eq!(e.evaluate(&r, 0.0, 0, 0, &[mk("XYZ1", "A320", "G-OFOO", "3")]).len(), 1);
        assert_eq!(e.evaluate(&r, 0.0, 0, 0, &[mk("XYZ1", "A320", "G-X", "abc123")]).len(), 1);
        assert!(e.evaluate(&r, 0.0, 0, 0, &[mk("XYZ1", "A320", "G-X", "4")]).is_empty());
        r.watch_enabled = false;
        assert!(e.evaluate(&r, 1e6, 0, 0, &[mk("BAW9", "A320", "G-X", "5")]).is_empty());
    }

    #[test]
    fn quiet_hours_wrap_midnight_and_spare_emergencies() {
        let mut r = AlertRules::default();
        r.quiet = QuietHours { enabled: true, start_min: 22 * 60, end_min: 7 * 60 };
        assert!(in_quiet_hours(&r, 23 * 60));
        assert!(in_quiet_hours(&r, 3 * 60));
        assert!(!in_quiet_hours(&r, 12 * 60));
        assert!(!in_quiet_hours(&r, 7 * 60));
        let mut e = AlertEngine::default();
        let mut p = plane("a");
        p.military = true;
        let ev = e.evaluate(&r, 0.0, 0, 23 * 60, &[p.clone()]);
        assert_eq!(ev.len(), 1);
        assert!(!ev[0].notify);
        p.hex = "b".into();
        p.squawk = Some("7700".into());
        let ev = e.evaluate(&r, 0.0, 0, 23 * 60, &[p]);
        assert!(ev.iter().find(|x| x.kind == AlertKind::Emergency).unwrap().notify);
    }

    #[test]
    fn disabled_rules_are_silent() {
        let mut r = AlertRules::default();
        r.flyover = false;
        r.military = false;
        r.emergency = false;
        let mut e = AlertEngine::default();
        let mut p = plane("a");
        p.dist_km = 0.5;
        p.alt_ft = Some(1000.0);
        p.military = true;
        p.squawk = Some("7700".into());
        assert!(e.evaluate(&r, 0.0, 0, 0, &[p]).is_empty());
    }
}
