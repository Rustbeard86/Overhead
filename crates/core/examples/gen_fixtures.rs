//! Generates the SYNTHETIC fixtures in tests/fixtures (run from repo root).
use overhead_core::geo::LatLon;
use overhead_core::synth::{demo_planes, demo_replay_file, frame_json, DEMO_HOME};
use std::fs;

fn main() {
    let dir = std::path::Path::new("tests/fixtures");
    fs::create_dir_all(dir).unwrap();
    let all = demo_planes();
    let w = |name: &str, s: String| fs::write(dir.join(name), s).unwrap();

    w("synthetic_busy.json", frame_json(DEMO_HOME, &all, 30.0, 1_800_000_030.0));
    w("synthetic_quiet.json", frame_json(DEMO_HOME, &all[3..4], 30.0, 1_800_000_030.0));
    let emg: Vec<_> = all.iter().filter(|p| p.squawk == "7700").cloned().collect();
    w("synthetic_emergency.json", frame_json(DEMO_HOME, &emg, 30.0, 1_800_000_030.0));
    let mil: Vec<_> = all.iter().filter(|p| p.flags & 1 != 0).cloned().collect();
    w("synthetic_military.json", frame_json(DEMO_HOME, &mil, 30.0, 1_800_000_030.0));
    let gnd: Vec<_> = all.iter().filter(|p| p.on_ground).cloned().collect();
    w("synthetic_ground.json", frame_json(DEMO_HOME, &gnd, 30.0, 1_800_000_030.0));
    // Hand-written: missing fields, one record without position, one without hex.
    w("synthetic_missing_fields.json", r#"{"synthetic":true,"now":1800000030.0,"aircraft":[
 {"hex":"aa0001","lat":51.6,"lon":-0.2},
 {"hex":"aa0002","flight":"        ","lat":51.55,"lon":-0.1,"alt_baro":"ground"},
 {"hex":"aa0003","flight":"NOPOS   ","alt_baro":12000,"gs":300.0,"track":90.0},
 {"lat":51.5,"lon":-0.1,"flight":"NOHEX   "},
 {"hex":"~aa0005","flight":"TISB1   ","lat":51.52,"lon":-0.15,"alt_baro":4500,"seen":3.2}
]}"#.to_string());
    // Recorded-style replay file for demo/replay mode: 8 minutes at 3 s steps.
    let _ = LatLon::new(0.0, 0.0);
    w("demo_replay.json", demo_replay_file(DEMO_HOME, 480.0, 3.0));
}
