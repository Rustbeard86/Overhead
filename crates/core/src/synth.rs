//! Synthetic traffic generator. Produces ADSBx-v2-style JSON frames of fake
//! aircraft on known tracks. Used for unit tests (known elevation and
//! closest-approach values), for demo mode, and to generate synthetic fixtures.
//! Everything here is SYNTHETIC: it is never presented as live data.

use crate::geo::{enu_inverse, LatLon, FT_M, KT_MS};
use serde_json::{json, Value};

pub const DEMO_HOME: LatLon = LatLon { lat: 51.5074, lon: -0.1278 };

#[derive(Clone, Debug)]
pub struct SynthPlane {
    pub hex: &'static str,
    pub callsign: &'static str,
    pub reg: &'static str,
    pub type_code: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
    pub squawk: &'static str,
    pub flags: u32,
    /// Position at t=0, ENU meters from home.
    pub p0: (f64, f64),
    /// Ground velocity, m/s (east, north).
    pub v: (f64, f64),
    pub alt0_ft: f64,
    pub vrate_fpm: f64,
    /// Wrap period for demo loops; 0 = no wrap.
    pub period_s: f64,
    pub on_ground: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PlaneState {
    pub e: f64,
    pub n: f64,
    pub alt_ft: f64,
    pub gs_kt: f64,
    pub track_deg: f64,
}

impl SynthPlane {
    pub fn state_at(&self, t: f64) -> PlaneState {
        let tt = if self.period_s > 0.0 { t.rem_euclid(self.period_s) } else { t };
        let e = self.p0.0 + self.v.0 * tt;
        let n = self.p0.1 + self.v.1 * tt;
        let alt = if self.on_ground { 0.0 } else { (self.alt0_ft + self.vrate_fpm * tt / 60.0).max(0.0) };
        let sp = (self.v.0 * self.v.0 + self.v.1 * self.v.1).sqrt();
        let track = if sp < 0.01 { 0.0 } else { (self.v.0.atan2(self.v.1).to_degrees() + 360.0) % 360.0 };
        PlaneState { e, n, alt_ft: alt, gs_kt: sp / KT_MS, track_deg: track }
    }
}

fn plane(
    hex: &'static str,
    callsign: &'static str,
    reg: &'static str,
    type_code: &'static str,
    desc: &'static str,
    category: &'static str,
    p0: (f64, f64),
    v: (f64, f64),
    alt0_ft: f64,
    vrate_fpm: f64,
) -> SynthPlane {
    SynthPlane {
        hex,
        callsign,
        reg,
        type_code,
        desc,
        category,
        squawk: "2000",
        flags: 0,
        p0,
        v,
        alt0_ft,
        vrate_fpm,
        period_s: 0.0,
        on_ground: false,
    }
}

/// The four named test scenarios with analytically known results.
pub fn named_scenarios() -> Vec<SynthPlane> {
    vec![
        // Direct overhead pass: 20 km west, 200 m/s east, 3000 m up. Overhead at t=100 s.
        plane("5c0001", "TST001", "N-DIRECT", "A320", "AIRBUS A-320", "A3", (-20_000.0, 0.0), (200.0, 0.0), 3000.0 / FT_M, 0.0),
        // 5 km offset pass (passes 5 km north of home).
        plane("5c0002", "TST002", "N-OFFSET", "B738", "BOEING 737-800", "A3", (-20_000.0, 5_000.0), (200.0, 0.0), 6000.0, 0.0),
        // Descending approach from the south toward home, 130 m/s, 3 deg-ish slope.
        plane("5c0003", "TST003", "N-APPROA", "E190", "EMBRAER 190", "A3", (0.0, -15_000.0), (0.0, 130.0), 5000.0, -700.0),
        // Hover: helicopter holding 1.5 km east, 800 ft.
        plane("5c0004", "TSTHEL", "N-HOVER", "EC35", "EUROCOPTER EC135", "A7", (1_500.0, 800.0), (0.0, 0.0), 800.0, 0.0),
    ]
}

/// A busy demo scene: the four scenarios (looping), a military flight, an
/// emergency squawk, a heavy, a GA aircraft, ground traffic and background
/// traffic on deterministic pseudo-random crossing tracks.
pub fn demo_planes() -> Vec<SynthPlane> {
    let mut v = Vec::new();
    // Looping versions of the named scenarios so something always passes overhead.
    let mut a = named_scenarios();
    a[0].callsign = "BAW117";
    a[0].reg = "G-EUPT";
    a[0].period_s = 260.0;
    a[0].p0 = (-26_000.0, 0.0);
    a[1].callsign = "EZY81KP";
    a[1].reg = "G-UZHA";
    a[1].type_code = "A20N";
    a[1].desc = "AIRBUS A-320neo";
    a[1].period_s = 300.0;
    a[1].p0 = (-30_000.0, 4_000.0);
    a[2].callsign = "CFE24D";
    a[2].reg = "G-LCYJ";
    a[2].p0 = (0.0, -24_000.0);
    a[2].period_s = 200.0;
    a[3].callsign = "GXPOL";
    a[3].reg = "G-POLC";
    a[3].period_s = 0.0;
    v.extend(a);

    let mut heavy = plane("4ca001", "VIR3N", "G-VLUX", "B789", "BOEING 787-9", "A5", (-60_000.0, -22_000.0), (230.0, 80.0), 9500.0, 800.0);
    heavy.period_s = 480.0;
    v.push(heavy);
    let mut a380 = plane("4ca002", "UAE7", "A6-EOA", "A388", "AIRBUS A-380-800", "A5", (45_000.0, 30_000.0), (-215.0, -95.0), 4200.0, -900.0);
    a380.period_s = 400.0;
    v.push(a380);
    let mut mil = plane("43c001", "RRR7216", "ZZ336", "A400", "AIRBUS A-400M", "A5", (50_000.0, -10_000.0), (-170.0, 25.0), 12000.0, 0.0);
    mil.flags = 1;
    mil.squawk = "4201";
    mil.period_s = 520.0;
    v.push(mil);
    let mut emg = plane("4ca003", "RYR42XE", "EI-DWB", "B738", "BOEING 737-800", "A3", (-12_000.0, 40_000.0), (60.0, -190.0), 11000.0, -1800.0);
    emg.squawk = "7700";
    emg.period_s = 420.0;
    v.push(emg);
    let mut ga = plane("400f01", "GOSEY", "G-OSEY", "C172", "CESSNA 172 SKYHAWK", "A1", (22_000.0, -8_000.0), (-30.0, 12.0), 2300.0, 0.0);
    ga.period_s = 1500.0;
    v.push(ga);
    // Ground traffic near the home position (an airfield).
    for (i, (hex, cs, reg, ty)) in [("400a01", "BAW19C", "G-TTNA", "A320"), ("400a02", "EZY62RH", "G-UZLK", "A319")].iter().enumerate() {
        let mut g = plane(hex, cs, reg, ty, "", "A3", (9_000.0 + 200.0 * i as f64, -6_000.0), (0.0, 0.0), 0.0, 0.0);
        g.on_ground = true;
        v.push(g);
    }

    // Background crossing traffic.
    const CALLS: [&str; 16] = [
        "DLH4RA", "AFR1380", "KLM1002", "SAS4631", "IBE3162", "EIN156", "SWR238", "AAL100", "UAL925", "QTR8", "ETD14", "TAP1359", "AEE601", "NAX1D", "WZZ4372", "LOT282",
    ];
    const TYPES: [(&str, &str, &str); 7] = [
        ("A320", "AIRBUS A-320", "A3"),
        ("B738", "BOEING 737-800", "A3"),
        ("A21N", "AIRBUS A-321neo", "A3"),
        ("E195", "EMBRAER 195", "A3"),
        ("B77W", "BOEING 777-300ER", "A5"),
        ("A359", "AIRBUS A-350-900", "A5"),
        ("DH8D", "DE HAVILLAND DASH 8-400", "A3"),
    ];
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for (i, cs) in CALLS.iter().enumerate() {
        let brg = rnd() * std::f64::consts::TAU;
        let r0 = 65_000.0;
        let offset = (rnd() - 0.5) * 70_000.0;
        // start on the circle, aim across the scene with a lateral offset
        let (sx, sy) = (r0 * brg.sin(), r0 * brg.cos());
        let (px, py) = (brg.cos() * offset, -brg.sin() * offset);
        let (tx, ty) = (px, py);
        let (dx, dy) = (tx - sx, ty - sy);
        let len = (dx * dx + dy * dy).sqrt();
        let speed = 120.0 + rnd() * 110.0;
        let vel = (dx / len * speed, dy / len * speed);
        let (ty_code, desc, cat) = TYPES[(rnd() * TYPES.len() as f64) as usize % TYPES.len()];
        let alt = 4000.0 + rnd() * 34_000.0;
        let vr = if alt < 12_000.0 { -1200.0 + rnd() * 2400.0 } else { 0.0 };
        let hexes = ["4d0001", "4d0002", "4d0003", "4d0004", "4d0005", "4d0006", "4d0007", "4d0008", "4d0009", "4d000a", "4d000b", "4d000c", "4d000d", "4d000e", "4d000f", "4d0010"];
        let mut p = plane(hexes[i], cs, "", ty_code, desc, cat, (sx, sy), vel, alt, vr);
        p.period_s = (2.0 * len / speed).max(60.0) * 0.5 + (rnd() * 30.0);
        // wrap exactly when the aircraft has crossed the far side
        p.period_s = 2.0 * r0 / speed * 1.05;
        // desynchronize phases
        let phase = rnd() * p.period_s;
        p.p0 = (sx + vel.0 * phase, sy + vel.1 * phase);
        // keep altitude sensible for the phase offset
        p.alt0_ft = (alt + vr * phase / 60.0).clamp(1500.0, 41_000.0);
        v.push(p);
    }
    v
}

fn record(home: LatLon, p: &SynthPlane, t: f64) -> Value {
    let s = p.state_at(t);
    let ll = enu_inverse(home, s.e, s.n);
    let mut o = json!({
        "hex": p.hex,
        "type": "adsb_icao",
        "flight": format!("{:<8}", p.callsign),
        "lat": (ll.lat * 1e6).round() / 1e6,
        "lon": (ll.lon * 1e6).round() / 1e6,
        "category": p.category,
        "squawk": p.squawk,
        "seen_pos": ((t * 7.0).rem_euclid(2.0) * 1000.0).round() / 1000.0,
        "synthetic": true,
    });
    let m = o.as_object_mut().unwrap();
    if !p.reg.is_empty() {
        m.insert("r".into(), json!(p.reg));
    }
    if !p.type_code.is_empty() {
        m.insert("t".into(), json!(p.type_code));
    }
    if !p.desc.is_empty() {
        m.insert("desc".into(), json!(p.desc));
    }
    if p.flags != 0 {
        m.insert("dbFlags".into(), json!(p.flags));
    }
    if p.on_ground {
        m.insert("alt_baro".into(), json!("ground"));
        m.insert("gs".into(), json!(0.0));
    } else {
        m.insert("alt_baro".into(), json!(s.alt_ft.round()));
        m.insert("alt_geom".into(), json!((s.alt_ft + 125.0).round()));
        m.insert("gs".into(), json!((s.gs_kt * 10.0).round() / 10.0));
        m.insert("track".into(), json!((s.track_deg * 100.0).round() / 100.0));
        m.insert("baro_rate".into(), json!(p.vrate_fpm.round()));
    }
    o
}

/// One ADSBx-v2-style response for the given planes at scenario time `t`.
pub fn frame_json(home: LatLon, planes: &[SynthPlane], t: f64, now_unix: f64) -> String {
    let list: Vec<Value> = planes.iter().map(|p| record(home, p, t)).collect();
    json!({"now": now_unix, "resultCount": list.len(), "ptime": 0, "aircraft": list, "synthetic": true}).to_string()
}

/// A recorded-replay file body (`{"frames":[{"t":..,"body":..}]}`) of the demo scene.
pub fn demo_replay_file(home: LatLon, seconds: f64, step: f64) -> String {
    let planes = demo_planes();
    let mut frames = Vec::new();
    let mut t = 0.0;
    while t <= seconds {
        let body: Value = serde_json::from_str(&frame_json(home, &planes, t, 1_800_000_000.0 + t)).unwrap();
        frames.push(json!({"t": t, "body": body}));
        t += step;
    }
    json!({"synthetic": true, "home": {"lat": home.lat, "lon": home.lon}, "frames": frames}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{closest_approach, elevation_deg, velocity_enu};

    #[test]
    fn direct_overhead_pass_values() {
        let p = &named_scenarios()[0];
        let s0 = p.state_at(0.0);
        let ca = closest_approach((s0.e, s0.n), velocity_enu(s0.gs_kt, s0.track_deg)).unwrap();
        assert!((ca.t_s - 100.0).abs() < 0.01);
        assert!(ca.d_min_m < 1.0);
        let s = p.state_at(100.0);
        assert!(s.e.abs() < 1e-6);
        assert!((elevation_deg(0.0, s.alt_ft * FT_M) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn offset_pass_values() {
        let p = &named_scenarios()[1];
        let s0 = p.state_at(0.0);
        let ca = closest_approach((s0.e, s0.n), velocity_enu(s0.gs_kt, s0.track_deg)).unwrap();
        assert!((ca.d_min_m - 5000.0).abs() < 1.0);
        // elevation at closest approach: 5 km away, 6000 ft (1828.8 m): ~20 deg
        let el = elevation_deg(5000.0, 6000.0 * FT_M);
        assert!((el - 20.1).abs() < 0.3, "{el}");
    }

    #[test]
    fn frames_parse_and_roundtrip_position() {
        let planes = demo_planes();
        let body = frame_json(DEMO_HOME, &planes, 12.0, 1.8e9);
        let r = crate::provider::parse_adsbx_v2(&body).unwrap();
        assert_eq!(r.aircraft.len(), planes.len());
        let a = r.aircraft.iter().find(|a| a.hex == "5c0004").unwrap();
        let (e, n) = crate::geo::enu(DEMO_HOME, LatLon::new(a.lat, a.lon));
        assert!((e - 1500.0).abs() < 1.0 && (n - 800.0).abs() < 1.0);
        assert!(r.aircraft.iter().any(|a| a.on_ground));
        assert!(r.aircraft.iter().any(|a| a.squawk.as_deref() == Some("7700")));
        assert!(r.aircraft.iter().any(|a| a.is_military()));
    }
}
