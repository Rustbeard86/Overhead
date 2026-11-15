//! Provider adapters: ADSBx-v2-style parser (airplanes.live, adsb.fi, adsb.lol)
//! and a separate OpenSky adapter.
//!
//! Quirks recorded from live captures (see DECISIONS.md):
//! * adsb.fi / airplanes.live: aircraft array is `aircraft`, `now` is unix seconds (float).
//!   adsb.lol: array is `ac`, `now` is unix *milliseconds*.
//! * `alt_baro` is a number (ft) or the string `"ground"`.
//! * `flight` is space-padded to 8 chars; adsb.lol emits `"@@@@@@@@"` for unknown.
//! * `seen_pos` may be absent (fall back to `seen`, then 0); `track`, `gs`, `squawk`,
//!   `r`, `t` are all optional. Some aircraft (TWR/GND service vehicles) have type `TWR`/`GND`.
//! * `dbFlags` bit 1 = military (present only on some providers).

use crate::model::{Aircraft, ParsedResponse, ProviderId};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected response shape: {0}")]
    Shape(&'static str),
}

fn num(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn text(v: &Value, key: &str) -> Option<String> {
    let s = v.get(key)?.as_str()?.trim();
    if s.is_empty() || s.chars().all(|c| c == '@') {
        None
    } else {
        Some(s.to_string())
    }
}

/// Parse one ADSBx-v2-style response body.
pub fn parse_adsbx_v2(body: &str) -> Result<ParsedResponse, ParseError> {
    let root: Value = serde_json::from_str(body)?;
    let list = root
        .get("aircraft")
        .or_else(|| root.get("ac"))
        .and_then(|a| a.as_array())
        .ok_or(ParseError::Shape("missing aircraft/ac array"))?;
    let mut now = num(&root, "now").unwrap_or(0.0);
    if now > 1e11 {
        now /= 1000.0; // milliseconds
    }

    let mut out = Vec::with_capacity(list.len());
    let mut skipped = 0;
    for r in list {
        let Some(hex) = r.get("hex").and_then(|h| h.as_str()) else {
            skipped += 1;
            continue;
        };
        let (Some(lat), Some(lon)) = (num(r, "lat"), num(r, "lon")) else {
            skipped += 1;
            continue;
        };
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            skipped += 1;
            continue;
        }
        let (alt_baro_ft, on_ground) = match r.get("alt_baro") {
            Some(Value::String(s)) if s.eq_ignore_ascii_case("ground") => (None, true),
            Some(v) => (v.as_f64(), false),
            None => (None, false),
        };
        let squawk = text(r, "squawk");
        out.push(Aircraft {
            hex: hex.trim_start_matches('~').to_lowercase(),
            callsign: text(r, "flight"),
            registration: text(r, "r"),
            type_code: text(r, "t"),
            type_desc: text(r, "desc"),
            lat,
            lon,
            alt_baro_ft,
            on_ground,
            alt_geom_ft: num(r, "alt_geom"),
            ground_speed_kt: num(r, "gs"),
            track_deg: num(r, "track").or_else(|| num(r, "true_heading")),
            vertical_rate_fpm: num(r, "baro_rate").or_else(|| num(r, "geom_rate")),
            squawk,
            adsb_category: text(r, "category"),
            seen_pos_s: num(r, "seen_pos").or_else(|| num(r, "seen")).unwrap_or(0.0),
            flags: r.get("dbFlags").and_then(|f| f.as_u64()).unwrap_or(0) as u32,
        });
    }
    Ok(ParsedResponse { now_s: now, aircraft: out, skipped_no_position: skipped })
}

/// Parse an OpenSky `/states/all` response.
/// State vector indices: 0 icao24, 1 callsign, 3 time_position, 4 last_contact,
/// 5 lon, 6 lat, 7 baro_altitude (m), 8 on_ground, 9 velocity (m/s),
/// 10 true_track, 11 vertical_rate (m/s), 13 geo_altitude (m), 14 squawk.
pub fn parse_opensky(body: &str) -> Result<ParsedResponse, ParseError> {
    let root: Value = serde_json::from_str(body)?;
    let now = root.get("time").and_then(|t| t.as_f64()).unwrap_or(0.0);
    let states = match root.get("states") {
        Some(Value::Array(a)) => a.as_slice(),
        Some(Value::Null) | None => &[],
        _ => return Err(ParseError::Shape("states is not an array")),
    };
    const M_TO_FT: f64 = 3.280_839_9;
    const MS_TO_KT: f64 = 1.943_844_5;
    const MS_TO_FPM: f64 = 196.850_4;
    let mut out = Vec::new();
    let mut skipped = 0;
    for s in states {
        let Some(a) = s.as_array() else {
            skipped += 1;
            continue;
        };
        let f = |i: usize| a.get(i).and_then(|v| v.as_f64());
        let (Some(lon), Some(lat)) = (f(5), f(6)) else {
            skipped += 1;
            continue;
        };
        let Some(hex) = a.first().and_then(|h| h.as_str()) else {
            skipped += 1;
            continue;
        };
        let callsign = a.get(1).and_then(|c| c.as_str()).map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
        let on_ground = a.get(8).and_then(|g| g.as_bool()).unwrap_or(false);
        let tpos = f(3).or_else(|| f(4)).unwrap_or(now);
        out.push(Aircraft {
            hex: hex.to_lowercase(),
            callsign,
            lat,
            lon,
            alt_baro_ft: if on_ground { None } else { f(7).map(|m| m * M_TO_FT) },
            on_ground,
            alt_geom_ft: f(13).map(|m| m * M_TO_FT),
            ground_speed_kt: f(9).map(|v| v * MS_TO_KT),
            track_deg: f(10),
            vertical_rate_fpm: f(11).map(|v| v * MS_TO_FPM),
            squawk: a.get(14).and_then(|s| s.as_str()).map(|s| s.to_string()),
            seen_pos_s: (now - tpos).max(0.0),
            ..Default::default()
        });
    }
    Ok(ParsedResponse { now_s: now, aircraft: out, skipped_no_position: skipped })
}

pub fn parse_for(id: ProviderId, body: &str) -> Result<ParsedResponse, ParseError> {
    match id {
        ProviderId::Opensky => parse_opensky(body),
        _ => parse_adsbx_v2(body),
    }
}

/// Fill a URL template for a point query.
pub fn build_url(template: &str, lat: f64, lon: f64, nm: f64) -> String {
    let nm = nm.ceil().clamp(1.0, 250.0) as u32;
    // OpenSky takes a bounding box instead of a radius.
    let dlat = nm as f64 / 60.0;
    let dlon = dlat / lat.to_radians().cos().abs().max(0.05);
    template
        .replace("{lat}", &format!("{lat:.4}"))
        .replace("{lon}", &format!("{lon:.4}"))
        .replace("{nm}", &nm.to_string())
        .replace("{lamin}", &format!("{:.4}", lat - dlat))
        .replace("{lamax}", &format!("{:.4}", lat + dlat))
        .replace("{lomin}", &format!("{:.4}", lon - dlon))
        .replace("{lomax}", &format!("{:.4}", lon + dlon))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(name: &str) -> String {
        let p = format!("{}/../../tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
    }

    #[test]
    fn live_adsbfi_parses() {
        let r = parse_adsbx_v2(&fx("live_adsbfi_london_40nm.json")).unwrap();
        assert!(!r.aircraft.is_empty());
        assert!(r.now_s > 1.7e9 && r.now_s < 3e9);
        let a = r.aircraft.iter().find(|a| a.hex == "400d5a").unwrap();
        assert_eq!(a.callsign.as_deref(), Some("GDBCG"));
        assert_eq!(a.registration.as_deref(), Some("G-DBCG"));
        assert_eq!(a.type_code.as_deref(), Some("A319"));
        assert!(a.on_ground);
        assert_eq!(a.alt_baro_ft, None);
        assert_eq!(a.squawk.as_deref(), Some("2000"));
    }

    #[test]
    fn live_adsblol_ac_array_and_ms_now() {
        let r = parse_adsbx_v2(&fx("live_adsblol_london_40nm.json")).unwrap();
        assert!(!r.aircraft.is_empty());
        assert!(r.now_s > 1.7e9 && r.now_s < 3e9, "ms timestamp normalized: {}", r.now_s);
        // "@@@@@@@@" junk callsign is dropped
        assert!(r.aircraft.iter().find(|a| a.hex == "406540").unwrap().callsign.is_none());
    }

    #[test]
    fn live_busy_all_have_position() {
        let r = parse_adsbx_v2(&fx("live_adsbfi_london_200nm_busy.json")).unwrap();
        assert!(r.aircraft.len() > 10);
        for a in &r.aircraft {
            assert!(a.lat.abs() <= 90.0 && a.lon.abs() <= 180.0);
            assert_eq!(a.hex, a.hex.to_lowercase());
            if let Some(c) = &a.callsign {
                assert_eq!(c, c.trim());
            }
        }
    }

    #[test]
    fn synthetic_fixtures() {
        let r = parse_adsbx_v2(&fx("synthetic_emergency.json")).unwrap();
        let a = r.aircraft.iter().find(|a| a.squawk.as_deref() == Some("7700")).unwrap();
        assert_eq!(a.emergency_squawk(), Some("emergency"));

        let r = parse_adsbx_v2(&fx("synthetic_military.json")).unwrap();
        assert!(r.aircraft.iter().any(|a| a.is_military()));

        let r = parse_adsbx_v2(&fx("synthetic_missing_fields.json")).unwrap();
        // 1 record has no position, one has no hex: both skipped; others survive.
        assert_eq!(r.skipped_no_position, 2);
        assert!(r.aircraft.iter().any(|a| a.callsign.is_none() && a.ground_speed_kt.is_none()));
        assert!(r.aircraft.iter().all(|a| a.seen_pos_s >= 0.0));

        let r = parse_adsbx_v2(&fx("synthetic_quiet.json")).unwrap();
        assert!(r.aircraft.len() <= 2);
        let r = parse_adsbx_v2(&fx("synthetic_busy.json")).unwrap();
        assert!(r.aircraft.len() >= 25);
        let r = parse_adsbx_v2(&fx("synthetic_ground.json")).unwrap();
        assert!(r.aircraft.iter().all(|a| a.on_ground && a.alt_baro_ft.is_none()));
    }

    #[test]
    fn malformed_is_an_error_not_a_panic() {
        assert!(parse_adsbx_v2("not json").is_err());
        assert!(parse_adsbx_v2("{\"now\":1}").is_err());
        assert!(parse_adsbx_v2("{\"aircraft\":{}}").is_err());
        assert!(parse_adsbx_v2("").is_err());
    }

    #[test]
    fn out_of_range_positions_skipped() {
        let r = parse_adsbx_v2(r#"{"now":1,"aircraft":[{"hex":"abc","lat":123.0,"lon":0.0}]}"#).unwrap();
        assert!(r.aircraft.is_empty());
        assert_eq!(r.skipped_no_position, 1);
    }

    #[test]
    fn opensky_adapter() {
        let body = r#"{"time":1700000000,"states":[
          ["4ca7b5","RYR1AB  ","Ireland",1699999998,1699999999,-0.2,51.4,3000.0,false,120.0,90.0,-5.0,null,3100.0,"1234",false,0],
          ["abcdef",null,"X",null,1699999990,null,null,null,true,0.0,0.0,0.0,null,null,null,false,0]
        ]}"#;
        let r = parse_opensky(body).unwrap();
        assert_eq!(r.aircraft.len(), 1);
        assert_eq!(r.skipped_no_position, 1);
        let a = &r.aircraft[0];
        assert_eq!(a.callsign.as_deref(), Some("RYR1AB"));
        assert!((a.alt_baro_ft.unwrap() - 9842.5).abs() < 1.0);
        assert!((a.ground_speed_kt.unwrap() - 233.3).abs() < 0.5);
        assert!((a.seen_pos_s - 2.0).abs() < 1e-9);
        assert!(parse_opensky("{\"time\":1,\"states\":null}").unwrap().aircraft.is_empty());
    }

    #[test]
    fn url_building() {
        let u = build_url(ProviderId::AdsbFi.default_template(), 51.5, -0.12, 50.0);
        assert_eq!(u, "https://opendata.adsb.fi/api/v2/lat/51.5000/lon/-0.1200/dist/50");
        let u = build_url(ProviderId::AirplanesLive.default_template(), 51.5, -0.12, 400.0);
        assert!(u.ends_with("/250"), "radius clamps to provider max: {u}");
        let u = build_url(ProviderId::Opensky.default_template(), 50.0, 8.0, 60.0);
        assert!(u.contains("lamin=49.0000") && u.contains("lamax=51.0000"));
    }
}
