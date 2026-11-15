//! Featured-plane scoring and hysteresis. All constants live in `ScoreParams`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreParams {
    /// Slant distance (km) at which the proximity score halves.
    pub prox_half_km: f64,
    pub prox_max: f64,
    /// Max bonus for approaching with a small closest-approach distance.
    pub approach_max: f64,
    pub approach_window_s: f64,
    pub approach_dmin_km: f64,
    pub military: f64,
    pub emergency: f64,
    pub heavy: f64,
    pub helicopter: f64,
    pub rare: f64,
    pub watch: f64,
    pub low_alt_max: f64,
    pub low_alt_ceiling_ft: f64,
    /// Aircraft on the ground are scaled by this factor.
    pub ground_factor: f64,
    /// Another plane must beat the current one by this ratio ...
    pub switch_ratio: f64,
    /// ... for this long before the featured plane changes.
    pub switch_hold_s: f64,
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self {
            prox_half_km: 6.0,
            prox_max: 100.0,
            approach_max: 40.0,
            approach_window_s: 900.0,
            approach_dmin_km: 5.0,
            military: 30.0,
            emergency: 80.0,
            heavy: 10.0,
            helicopter: 15.0,
            rare: 15.0,
            watch: 25.0,
            low_alt_max: 10.0,
            low_alt_ceiling_ft: 10_000.0,
            ground_factor: 0.08,
            switch_ratio: 1.2,
            switch_hold_s: 3.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScoreInput {
    pub slant_km: f64,
    /// Seconds to closest approach and its distance (km), if known.
    pub approach: Option<(f64, f64)>,
    pub alt_ft: Option<f64>,
    pub on_ground: bool,
    pub military: bool,
    pub emergency: bool,
    pub heavy: bool,
    pub helicopter: bool,
    pub rare: bool,
    pub watch: bool,
}

pub fn score(i: &ScoreInput, p: &ScoreParams) -> f64 {
    let x = i.slant_km / p.prox_half_km;
    let mut s = p.prox_max / (1.0 + x * x);
    if let Some((t, dmin_km)) = i.approach {
        if (0.0..=p.approach_window_s).contains(&t) && dmin_km < p.approach_dmin_km {
            s += p.approach_max * (1.0 - dmin_km / p.approach_dmin_km) * (1.0 - t / p.approach_window_s);
        }
    }
    if i.military { s += p.military; }
    if i.emergency { s += p.emergency; }
    if i.heavy { s += p.heavy; }
    if i.helicopter { s += p.helicopter; }
    if i.rare { s += p.rare; }
    if i.watch { s += p.watch; }
    if let Some(alt) = i.alt_ft {
        if !i.on_ground {
            s += p.low_alt_max * (1.0 - alt / p.low_alt_ceiling_ft).clamp(0.0, 1.0);
        }
    }
    if i.on_ground { s *= p.ground_factor; }
    s
}

/// Picks the featured plane with hysteresis.
#[derive(Debug, Default)]
pub struct FeaturedSelector {
    current: Option<String>,
    challenger: Option<(String, f64)>, // (hex, since)
}

impl FeaturedSelector {
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// `scores` are (hex, score) for planes in range. `now` is seconds on any monotonic clock.
    pub fn update(&mut self, now: f64, scores: &[(String, f64)], p: &ScoreParams) -> Option<String> {
        let best = scores.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let Some(best) = best else {
            self.current = None;
            self.challenger = None;
            return None;
        };
        let cur_score = self.current.as_ref().and_then(|c| scores.iter().find(|s| &s.0 == c)).map(|s| s.1);
        match (&self.current, cur_score) {
            (None, _) | (Some(_), None) => {
                // nothing featured or it left range: take the best immediately
                self.current = Some(best.0.clone());
                self.challenger = None;
            }
            (Some(cur), Some(cs)) => {
                if best.0 != *cur && best.1 > cs * p.switch_ratio {
                    match &self.challenger {
                        Some((h, since)) if *h == best.0 => {
                            if now - since >= p.switch_hold_s {
                                self.current = Some(best.0.clone());
                                self.challenger = None;
                            }
                        }
                        _ => self.challenger = Some((best.0.clone(), now)),
                    }
                } else {
                    self.challenger = None;
                }
            }
        }
        self.current.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(slant: f64) -> ScoreInput {
        ScoreInput { slant_km: slant, alt_ft: Some(30_000.0), ..Default::default() }
    }

    #[test]
    fn closer_scores_higher() {
        let p = ScoreParams::default();
        assert!(score(&input(2.0), &p) > score(&input(10.0), &p));
        assert!(score(&input(10.0), &p) > score(&input(60.0), &p));
    }

    #[test]
    fn approach_bonus_only_when_close_and_soon() {
        let p = ScoreParams::default();
        let base = score(&input(20.0), &p);
        let mut a = input(20.0);
        a.approach = Some((120.0, 0.5));
        assert!(score(&a, &p) > base + 20.0);
        a.approach = Some((-30.0, 0.5)); // already passed
        assert!((score(&a, &p) - base).abs() < 1e-9);
        a.approach = Some((120.0, 9.0)); // too far off
        assert!((score(&a, &p) - base).abs() < 1e-9);
        a.approach = Some((2000.0, 0.1)); // too far ahead
        assert!((score(&a, &p) - base).abs() < 1e-9);
    }

    #[test]
    fn special_bonuses() {
        let p = ScoreParams::default();
        let base = score(&input(15.0), &p);
        for f in [
            |i: &mut ScoreInput| i.military = true,
            |i: &mut ScoreInput| i.emergency = true,
            |i: &mut ScoreInput| i.heavy = true,
            |i: &mut ScoreInput| i.helicopter = true,
            |i: &mut ScoreInput| i.rare = true,
            |i: &mut ScoreInput| i.watch = true,
        ] {
            let mut i = input(15.0);
            f(&mut i);
            assert!(score(&i, &p) > base);
        }
        let mut low = input(15.0);
        low.alt_ft = Some(1500.0);
        assert!(score(&low, &p) > base);
    }

    #[test]
    fn ground_traffic_is_demoted() {
        let p = ScoreParams::default();
        let mut g = input(1.0);
        g.on_ground = true;
        assert!(score(&g, &p) < score(&input(20.0), &p));
    }

    #[test]
    fn emergency_beats_a_nearer_plane() {
        let p = ScoreParams::default();
        let mut e = input(18.0);
        e.emergency = true;
        assert!(score(&e, &p) > score(&input(6.0), &p));
    }

    #[test]
    fn hysteresis_needs_20_percent_for_3_seconds() {
        let p = ScoreParams::default();
        let mut sel = FeaturedSelector::default();
        let a = |s: f64| vec![("a".to_string(), 100.0), ("b".to_string(), s)];
        assert_eq!(sel.update(0.0, &a(10.0), &p).as_deref(), Some("a"));
        // b only 10% better: never switches
        for t in 1..10 {
            assert_eq!(sel.update(t as f64, &a(110.0), &p).as_deref(), Some("a"));
        }
        // b 30% better: holds for 3 s first
        assert_eq!(sel.update(10.0, &a(130.0), &p).as_deref(), Some("a"));
        assert_eq!(sel.update(12.0, &a(130.0), &p).as_deref(), Some("a"));
        assert_eq!(sel.update(13.1, &a(130.0), &p).as_deref(), Some("b"));
    }

    #[test]
    fn challenger_must_hold_continuously() {
        let p = ScoreParams::default();
        let mut sel = FeaturedSelector::default();
        let a = |s: f64| vec![("a".to_string(), 100.0), ("b".to_string(), s)];
        sel.update(0.0, &a(10.0), &p);
        sel.update(1.0, &a(130.0), &p);
        sel.update(2.0, &a(105.0), &p); // dips: resets
        sel.update(3.0, &a(130.0), &p);
        assert_eq!(sel.update(5.0, &a(130.0), &p).as_deref(), Some("a"));
        assert_eq!(sel.update(6.1, &a(130.0), &p).as_deref(), Some("b"));
    }

    #[test]
    fn leaves_range_takes_best_immediately() {
        let p = ScoreParams::default();
        let mut sel = FeaturedSelector::default();
        sel.update(0.0, &[("a".into(), 50.0), ("b".into(), 10.0)], &p);
        assert_eq!(sel.update(1.0, &[("b".into(), 10.0)], &p).as_deref(), Some("b"));
        assert_eq!(sel.update(2.0, &[], &p), None);
    }
}
