//! Geometry: haversine, bearing, local ENU (azimuthal-equidistant) projection,
//! elevation angle, closest approach.

pub const R_EARTH_M: f64 = 6_371_000.0;
pub const NM_M: f64 = 1852.0;
pub const FT_M: f64 = 0.3048;
pub const KT_MS: f64 = NM_M / 3600.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }
}

/// Great-circle distance in meters.
pub fn haversine_m(a: LatLon, b: LatLon) -> f64 {
    let (p1, p2) = (a.lat.to_radians(), b.lat.to_radians());
    let dp = p2 - p1;
    let dl = (b.lon - a.lon).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * R_EARTH_M * h.sqrt().min(1.0).asin()
}

/// Initial bearing from `a` to `b`, degrees clockwise from north, [0, 360).
pub fn bearing_deg(a: LatLon, b: LatLon) -> f64 {
    let (p1, p2) = (a.lat.to_radians(), b.lat.to_radians());
    let dl = (b.lon - a.lon).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

/// Point reached from `a` travelling `dist_m` on initial bearing `brg_deg`.
pub fn destination(a: LatLon, brg_deg: f64, dist_m: f64) -> LatLon {
    let d = dist_m / R_EARTH_M;
    let t = brg_deg.to_radians();
    let p1 = a.lat.to_radians();
    let l1 = a.lon.to_radians();
    let p2 = (p1.sin() * d.cos() + p1.cos() * d.sin() * t.cos()).asin();
    let l2 = l1 + (t.sin() * d.sin() * p1.cos()).atan2(d.cos() - p1.sin() * p2.sin());
    LatLon { lat: p2.to_degrees(), lon: ((l2.to_degrees() + 540.0) % 360.0) - 180.0 }
}

/// East/north offset (meters) of `p` from `home`. Azimuthal equidistant:
/// distance from home is exact, direction is the true bearing.
pub fn enu(home: LatLon, p: LatLon) -> (f64, f64) {
    let d = haversine_m(home, p);
    if d < 1e-6 {
        return (0.0, 0.0);
    }
    let b = bearing_deg(home, p).to_radians();
    (d * b.sin(), d * b.cos())
}

/// Inverse of [`enu`].
pub fn enu_inverse(home: LatLon, e: f64, n: f64) -> LatLon {
    let d = (e * e + n * n).sqrt();
    if d < 1e-6 {
        return home;
    }
    let b = e.atan2(n).to_degrees();
    destination(home, (b + 360.0) % 360.0, d)
}

/// Elevation angle (degrees) of a target at ground distance `d_m` and height
/// `h_m` above the observer, spherical earth, no refraction.
pub fn elevation_deg(d_m: f64, h_m: f64) -> f64 {
    let alpha = d_m / R_EARTH_M;
    let num = (R_EARTH_M + h_m) * alpha.cos() - R_EARTH_M;
    let den = (R_EARTH_M + h_m) * alpha.sin();
    if den.abs() < 1e-9 {
        return if h_m >= 0.0 { 90.0 } else { -90.0 };
    }
    // atan2 keeps directly-overhead (den -> 0+) at +90 and is well behaved.
    num.atan2(den).to_degrees()
}

/// Straight-line distance (m) from observer to target.
pub fn slant_m(d_m: f64, h_m: f64) -> f64 {
    let alpha = d_m / R_EARTH_M;
    let r = R_EARTH_M + h_m;
    (R_EARTH_M * R_EARTH_M + r * r - 2.0 * R_EARTH_M * r * alpha.cos()).max(0.0).sqrt()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Approach {
    /// Seconds from now until closest approach (negative = already passed).
    pub t_s: f64,
    /// Horizontal distance at closest approach, meters.
    pub d_min_m: f64,
}

/// Closest approach on the ENU plane for position `p` (m) and velocity `v` (m/s)
/// relative to the observer at the origin. `None` if effectively stationary.
pub fn closest_approach(p: (f64, f64), v: (f64, f64)) -> Option<Approach> {
    let v2 = v.0 * v.0 + v.1 * v.1;
    if v2 < 1e-6 {
        return None;
    }
    let t = -(p.0 * v.0 + p.1 * v.1) / v2;
    let (x, y) = (p.0 + v.0 * t, p.1 + v.1 * t);
    Some(Approach { t_s: t, d_min_m: (x * x + y * y).sqrt() })
}

/// Ground velocity (east, north) in m/s from speed in knots and track in degrees.
pub fn velocity_enu(gs_kt: f64, track_deg: f64) -> (f64, f64) {
    let s = gs_kt * KT_MS;
    let t = track_deg.to_radians();
    (s * t.sin(), s * t.cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b} (tol {tol})");
    }

    #[test]
    fn haversine_known() {
        // London -> Paris ~ 343.5 km
        let d = haversine_m(LatLon::new(51.5074, -0.1278), LatLon::new(48.8566, 2.3522));
        close(d / 1000.0, 343.5, 1.5);
    }

    #[test]
    fn bearing_cardinals() {
        let h = LatLon::new(0.0, 0.0);
        close(bearing_deg(h, LatLon::new(1.0, 0.0)), 0.0, 1e-6);
        close(bearing_deg(h, LatLon::new(0.0, 1.0)), 90.0, 1e-6);
        close(bearing_deg(h, LatLon::new(-1.0, 0.0)), 180.0, 1e-6);
        close(bearing_deg(h, LatLon::new(0.0, -1.0)), 270.0, 1e-6);
    }

    #[test]
    fn enu_roundtrip() {
        let h = LatLon::new(51.5, -0.12);
        let p = LatLon::new(51.7, 0.2);
        let (e, n) = enu(h, p);
        let q = enu_inverse(h, e, n);
        close(q.lat, p.lat, 1e-6);
        close(q.lon, p.lon, 1e-6);
        close((e * e + n * n).sqrt(), haversine_m(h, p), 1e-6);
    }

    #[test]
    fn elevation_overhead_and_horizon() {
        close(elevation_deg(0.0, 10_000.0), 90.0, 1e-6);
        // 10 km up, 10 km away: ~45 deg (earth curvature barely matters)
        close(elevation_deg(10_000.0, 10_000.0), 44.9, 0.2);
        // far away and low goes below the horizon
        assert!(elevation_deg(300_000.0, 3_000.0) < 0.0);
        // ground level, distant: slightly negative from curvature
        assert!(elevation_deg(50_000.0, 0.0) < 0.0);
    }

    #[test]
    fn slant_matches_pythagoras_nearby() {
        close(slant_m(3000.0, 4000.0), 5000.0, 2.0);
    }

    #[test]
    fn closest_approach_direct_pass() {
        // 10 km west, flying east at 200 m/s -> passes overhead in 50 s
        let a = closest_approach((-10_000.0, 0.0), (200.0, 0.0)).unwrap();
        close(a.t_s, 50.0, 1e-9);
        close(a.d_min_m, 0.0, 1e-9);
    }

    #[test]
    fn closest_approach_offset_pass() {
        let a = closest_approach((-10_000.0, 5_000.0), (200.0, 0.0)).unwrap();
        close(a.t_s, 50.0, 1e-9);
        close(a.d_min_m, 5_000.0, 1e-9);
    }

    #[test]
    fn closest_approach_receding_is_negative() {
        let a = closest_approach((10_000.0, 0.0), (200.0, 0.0)).unwrap();
        assert!(a.t_s < 0.0);
    }

    #[test]
    fn stationary_has_no_approach() {
        assert!(closest_approach((100.0, 100.0), (0.0, 0.0)).is_none());
    }

    #[test]
    fn velocity_components() {
        let (e, n) = velocity_enu(100.0, 90.0);
        close(e, 100.0 * KT_MS, 1e-9);
        close(n, 0.0, 1e-9);
    }
}
