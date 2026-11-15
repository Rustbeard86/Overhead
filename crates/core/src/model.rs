//! Normalized data model shared by providers, engine, and the UI contract.

use serde::{Deserialize, Serialize};

pub const FLAG_MILITARY: u32 = 1;
pub const FLAG_INTERESTING: u32 = 2;
pub const FLAG_PIA: u32 = 4;
pub const FLAG_LADD: u32 = 8;

/// One aircraft as reported by a provider, normalized. Records without a
/// position never become an `Aircraft`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Aircraft {
    pub hex: String,
    pub callsign: Option<String>,
    pub registration: Option<String>,
    pub type_code: Option<String>,
    pub type_desc: Option<String>,
    pub lat: f64,
    pub lon: f64,
    /// Barometric altitude in feet. `None` when unknown or on the ground.
    pub alt_baro_ft: Option<f64>,
    /// The provider reported `alt_baro: "ground"`.
    pub on_ground: bool,
    pub alt_geom_ft: Option<f64>,
    pub ground_speed_kt: Option<f64>,
    pub track_deg: Option<f64>,
    pub vertical_rate_fpm: Option<f64>,
    pub squawk: Option<String>,
    /// ADS-B emitter category, e.g. "A3", "A5", "A7".
    pub adsb_category: Option<String>,
    /// Seconds since the position was last updated, at fetch time.
    pub seen_pos_s: f64,
    /// dbFlags bit field: 1 military, 2 interesting, 4 PIA, 8 LADD.
    pub flags: u32,
}

impl Aircraft {
    pub fn is_military(&self) -> bool {
        self.flags & FLAG_MILITARY != 0
    }
    pub fn emergency_squawk(&self) -> Option<&'static str> {
        match self.squawk.as_deref() {
            Some("7500") => Some("hijack"),
            Some("7600") => Some("radio failure"),
            Some("7700") => Some("emergency"),
            _ => None,
        }
    }
    /// Altitude used for geometry: geometric if present, else barometric.
    pub fn altitude_ft(&self) -> Option<f64> {
        if self.on_ground {
            return Some(0.0);
        }
        self.alt_geom_ft.or(self.alt_baro_ft)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ParsedResponse {
    /// Provider timestamp, unix seconds.
    pub now_s: f64,
    pub aircraft: Vec<Aircraft>,
    /// Records dropped for lacking a position.
    pub skipped_no_position: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    AirplanesLive,
    AdsbFi,
    AdsbLol,
    Opensky,
}

impl ProviderId {
    pub fn name(self) -> &'static str {
        match self {
            ProviderId::AirplanesLive => "airplanes.live",
            ProviderId::AdsbFi => "adsb.fi",
            ProviderId::AdsbLol => "adsb.lol",
            ProviderId::Opensky => "OpenSky",
        }
    }
    pub fn default_template(self) -> &'static str {
        match self {
            ProviderId::AirplanesLive => "https://api.airplanes.live/v2/point/{lat}/{lon}/{nm}",
            ProviderId::AdsbFi => "https://opendata.adsb.fi/api/v2/lat/{lat}/lon/{lon}/dist/{nm}",
            ProviderId::AdsbLol => "https://api.adsb.lol/v2/lat/{lat}/lon/{lon}/dist/{nm}",
            ProviderId::Opensky => "https://opensky-network.org/api/states/all?lamin={lamin}&lomin={lomin}&lamax={lamax}&lomax={lomax}",
        }
    }
    pub fn default_order() -> Vec<ProviderId> {
        vec![ProviderId::AirplanesLive, ProviderId::AdsbFi, ProviderId::AdsbLol]
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Narrowbody,
    Widebody,
    Light,
    Helicopter,
    Military,
    Unknown,
}
