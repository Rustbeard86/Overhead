//! Replay and demo sources. The app is fully demoable with no network.
//!
//! Replay files are either
//!  * `{"frames":[{"t":<seconds>,"body":<provider response>}, ...], "home": {"lat","lon"}?}`, or
//!  * a single raw provider response (the aircraft are advanced by dead
//!    reckoning from the file's content, looping every 10 minutes; synthetic motion).
//! Demo mode (no home configured) generates the synthetic demo scene.

use crate::geo::{destination, LatLon, KT_MS};
use crate::model::ParsedResponse;
use crate::provider::parse_adsbx_v2;
use crate::synth::{demo_planes, frame_json, SynthPlane, DEMO_HOME};
use serde_json::Value;
use std::path::Path;

pub enum ReplaySource {
    Demo { planes: Vec<SynthPlane> },
    Frames { frames: Vec<(f64, ParsedResponse)>, home: Option<LatLon> },
    Single { base: ParsedResponse },
}

const SINGLE_LOOP_S: f64 = 600.0;

impl ReplaySource {
    pub fn demo() -> Self {
        ReplaySource::Demo { planes: demo_planes() }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_str(&text)
    }

    pub fn from_str(text: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if let Some(frames) = v.get("frames").and_then(|f| f.as_array()) {
            let mut out = Vec::new();
            for f in frames {
                let t = f.get("t").and_then(|t| t.as_f64()).unwrap_or(0.0);
                let body = f.get("body").ok_or("frame without body")?;
                out.push((t, parse_adsbx_v2(&body.to_string()).map_err(|e| e.to_string())?));
            }
            if out.is_empty() {
                return Err("replay file has no frames".into());
            }
            out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let home = v.get("home").and_then(|h| Some(LatLon::new(h.get("lat")?.as_f64()?, h.get("lon")?.as_f64()?)));
            return Ok(ReplaySource::Frames { frames: out, home });
        }
        Ok(ReplaySource::Single { base: parse_adsbx_v2(text).map_err(|e| e.to_string())? })
    }

    /// Where a replay is centred if no home is configured.
    pub fn default_home(&self) -> LatLon {
        match self {
            ReplaySource::Frames { home: Some(h), .. } => *h,
            ReplaySource::Single { base } => {
                // centre of mass of the recorded aircraft
                let n = base.aircraft.len().max(1) as f64;
                LatLon::new(base.aircraft.iter().map(|a| a.lat).sum::<f64>() / n, base.aircraft.iter().map(|a| a.lon).sum::<f64>() / n)
            }
            _ => DEMO_HOME,
        }
    }

    /// The response a provider would have returned `elapsed` seconds into the replay.
    pub fn frame_at(&self, elapsed: f64) -> ParsedResponse {
        match self {
            ReplaySource::Demo { planes } => {
                let body = frame_json(DEMO_HOME, planes, elapsed, 0.0);
                parse_adsbx_v2(&body).expect("synthetic frame parses")
            }
            ReplaySource::Frames { frames, .. } => {
                let last_t = frames.last().map(|f| f.0).unwrap_or(0.0);
                let span = last_t.max(1.0);
                let t = elapsed.rem_euclid(span);
                let idx = frames.iter().rposition(|f| f.0 <= t).unwrap_or(0);
                frames[idx].1.clone()
            }
            ReplaySource::Single { base } => {
                let dt = elapsed.rem_euclid(SINGLE_LOOP_S);
                let mut r = base.clone();
                for a in &mut r.aircraft {
                    if a.on_ground {
                        continue;
                    }
                    if let (Some(gs), Some(trk)) = (a.ground_speed_kt, a.track_deg) {
                        let p = destination(LatLon::new(a.lat, a.lon), trk, gs * KT_MS * dt);
                        a.lat = p.lat;
                        a.lon = p.lon;
                    }
                }
                r
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_frames_move() {
        let s = ReplaySource::demo();
        let a = s.frame_at(0.0);
        let b = s.frame_at(30.0);
        assert_eq!(a.aircraft.len(), b.aircraft.len());
        let pa = a.aircraft.iter().find(|x| x.hex == "5c0001" || x.callsign.as_deref() == Some("BAW117")).unwrap();
        let pb = b.aircraft.iter().find(|x| x.hex == pa.hex).unwrap();
        assert!((pa.lon - pb.lon).abs() > 0.001);
    }

    #[test]
    fn frames_file_loops() {
        let text = std::fs::read_to_string(format!("{}/../../tests/fixtures/demo_replay.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let s = ReplaySource::from_str(&text).unwrap();
        assert!(matches!(s, ReplaySource::Frames { .. }));
        assert!((s.default_home().lat - DEMO_HOME.lat).abs() < 1e-9);
        let a = s.frame_at(3.0);
        let b = s.frame_at(3.0 + 480.0);
        assert_eq!(a.aircraft.len(), b.aircraft.len());
        assert!(s.frame_at(3.0).aircraft.len() > 20);
    }

    #[test]
    fn single_response_advances() {
        let text = std::fs::read_to_string(format!("{}/../../tests/fixtures/live_adsbfi_london_200nm_busy.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let s = ReplaySource::from_str(&text).unwrap();
        let a = s.frame_at(0.0);
        let b = s.frame_at(60.0);
        let moved = a.aircraft.iter().zip(&b.aircraft).filter(|(x, y)| (x.lat - y.lat).abs() > 1e-4 || (x.lon - y.lon).abs() > 1e-4).count();
        assert!(moved > 5);
    }

    #[test]
    fn bad_files_error() {
        assert!(ReplaySource::from_str("nope").is_err());
        assert!(ReplaySource::from_str(r#"{"frames":[]}"#).is_err());
        assert!(ReplaySource::load(std::path::Path::new("/nonexistent/x.json")).is_err());
    }
}
