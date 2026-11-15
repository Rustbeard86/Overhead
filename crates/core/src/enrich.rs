//! Enrichment: bundled type/airline tables, Planespotters photo lookup, and a
//! no-op `RouteLookup`. Everything here is best-effort: failures return `None`
//! and the UI hides the row (never a placeholder).

use crate::model::{Aircraft, Category};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize)]
pub struct TypeInfo {
    pub n: String,
    pub c: String,
    #[serde(default)]
    pub r: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AirlineInfo {
    pub n: String,
    #[serde(default)]
    pub c: Option<String>,
}

pub struct Tables {
    types: HashMap<String, TypeInfo>,
    airlines: HashMap<String, AirlineInfo>,
}

impl Tables {
    pub fn load() -> Self {
        let types = serde_json::from_str(include_str!("../data/types.json")).expect("types.json");
        let airlines = serde_json::from_str(include_str!("../data/airlines.json")).expect("airlines.json");
        Self { types, airlines }
    }

    pub fn type_info(&self, code: &str) -> Option<&TypeInfo> {
        self.types.get(&code.to_uppercase())
    }

    /// ICAO callsigns are 3 letters + flight designator ("BAW117", "EZY81KP").
    pub fn airline_for_callsign(&self, callsign: &str) -> Option<&AirlineInfo> {
        let cs = callsign.trim().to_uppercase();
        let b = cs.as_bytes();
        if b.len() >= 4 && b[..3].iter().all(|c| c.is_ascii_alphabetic()) && b[3].is_ascii_digit() {
            return self.airlines.get(&cs[..3]);
        }
        None
    }

    pub fn category(&self, a: &Aircraft) -> Category {
        if a.is_military() {
            return Category::Military;
        }
        if let Some(t) = a.type_code.as_deref().and_then(|t| self.type_info(t)) {
            return match t.c.as_str() {
                "narrowbody" => Category::Narrowbody,
                "widebody" => Category::Widebody,
                "light" => Category::Light,
                "helicopter" => Category::Helicopter,
                "military" => Category::Military,
                _ => Category::Unknown,
            };
        }
        match a.adsb_category.as_deref() {
            Some("A7") => Category::Helicopter,
            Some("A5") | Some("A6") => Category::Widebody,
            Some("A3") | Some("A4") => Category::Narrowbody,
            Some("A1") | Some("A2") | Some("B1") | Some("B4") => Category::Light,
            _ => Category::Unknown,
        }
    }

    pub fn is_rare(&self, a: &Aircraft) -> bool {
        a.type_code.as_deref().and_then(|t| self.type_info(t)).map(|t| t.r == 1).unwrap_or(false)
    }

    pub fn is_heavy(&self, a: &Aircraft, cat: Category) -> bool {
        cat == Category::Widebody || a.adsb_category.as_deref() == Some("A5")
    }

    pub fn type_name(&self, a: &Aircraft) -> Option<String> {
        a.type_code
            .as_deref()
            .and_then(|t| self.type_info(t))
            .map(|t| t.n.clone())
            .or_else(|| a.type_desc.clone().filter(|d| !d.is_empty()))
    }
}

// ---------------------------------------------------------------- photos

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Photo {
    pub src: String,
    pub link: String,
    pub photographer: String,
    pub width: u32,
    pub height: u32,
}

/// Parse a Planespotters `/pub/photos/{hex|reg}/..` response.
pub fn parse_planespotters(body: &str) -> Option<Photo> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let p = v.get("photos")?.as_array()?.first()?;
    let img = p.get("thumbnail_large").or_else(|| p.get("thumbnail"))?;
    let src = img.get("src")?.as_str()?.to_string();
    let size = img.get("size");
    let dim = |k: &str| size.and_then(|s| s.get(k)).and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    Some(Photo {
        src,
        link: p.get("link")?.as_str()?.to_string(),
        photographer: p.get("photographer").and_then(|x| x.as_str()).unwrap_or("Unknown").to_string(),
        width: dim("width"),
        height: dim("height"),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CacheEntry {
    fetched_s: u64,
    photo: Option<Photo>,
}

/// Cache policy. Planespotters' terms restrict how long thumbnails may be
/// kept, so the default positive TTL is 24 h (the brief asked for 30 days; see
/// DECISIONS.md). Misses are cached for 6 h.
#[derive(Clone, Debug)]
pub struct PhotoCfg {
    pub ttl: Duration,
    pub negative_ttl: Duration,
    pub timeout: Duration,
    pub max_concurrent: usize,
    pub base_url: String,
}
impl Default for PhotoCfg {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(24 * 3600),
            negative_ttl: Duration::from_secs(6 * 3600),
            timeout: Duration::from_secs(3),
            max_concurrent: 2,
            base_url: "https://api.planespotters.net/pub/photos".into(),
        }
    }
}

pub struct PhotoLookup {
    client: reqwest::Client,
    cfg: PhotoCfg,
    sem: Arc<tokio::sync::Semaphore>,
    cache: Mutex<HashMap<String, CacheEntry>>,
    path: Option<PathBuf>,
}

fn now_s() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl PhotoLookup {
    pub fn new(cfg: PhotoCfg, cache_dir: Option<PathBuf>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Overhead/0.1")
            .timeout(cfg.timeout)
            .build()
            .expect("http client");
        let path = cache_dir.map(|d| d.join("photo_cache.json"));
        let cache = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self { client, sem: Arc::new(tokio::sync::Semaphore::new(cfg.max_concurrent)), cfg, cache: Mutex::new(cache), path }
    }

    fn cached(&self, key: &str) -> Option<Option<Photo>> {
        let c = self.cache.lock().ok()?;
        let e = c.get(key)?;
        let ttl = if e.photo.is_some() { self.cfg.ttl } else { self.cfg.negative_ttl };
        (now_s().saturating_sub(e.fetched_s) < ttl.as_secs()).then(|| e.photo.clone())
    }

    fn store(&self, key: String, photo: Option<Photo>) {
        if let Ok(mut c) = self.cache.lock() {
            c.insert(key, CacheEntry { fetched_s: now_s(), photo });
            // drop expired entries so the file stays small
            let max = self.cfg.ttl.as_secs();
            c.retain(|_, e| now_s().saturating_sub(e.fetched_s) < max);
            if let Some(p) = &self.path {
                if let Ok(t) = serde_json::to_string(&*c) {
                    let _ = std::fs::write(p, t);
                }
            }
        }
    }

    async fn fetch(&self, url: &str) -> Option<Photo> {
        let _permit = self.sem.acquire().await.ok()?;
        let resp = self.client.get(url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        parse_planespotters(&resp.text().await.ok()?)
    }

    /// Look up by registration first, then ICAO hex. `None` on any failure.
    pub async fn lookup(&self, hex: &str, registration: Option<&str>) -> Option<Photo> {
        let key = hex.to_lowercase();
        if let Some(hit) = self.cached(&key) {
            return hit;
        }
        let mut photo = None;
        if let Some(reg) = registration.filter(|r| !r.is_empty()) {
            photo = self.fetch(&format!("{}/reg/{}", self.cfg.base_url, reg)).await;
        }
        if photo.is_none() {
            photo = self.fetch(&format!("{}/hex/{}", self.cfg.base_url, key)).await;
        }
        self.store(key, photo.clone());
        photo
    }
}

// ---------------------------------------------------------------- routes

/// Routes are out of scope for v1; the trait is the seam for later.
pub trait RouteLookup: Send + Sync {
    fn route(&self, callsign: &str) -> Option<Route>;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Route {
    pub origin: String,
    pub destination: String,
}

pub struct NoRoutes;
impl RouteLookup for NoRoutes {
    fn route(&self, _callsign: &str) -> Option<Route> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_load_and_resolve() {
        let t = Tables::load();
        assert_eq!(t.type_info("a388").unwrap().n, "Airbus A380-800");
        assert!(t.type_info("zzzz").is_none());
        assert_eq!(t.airline_for_callsign("BAW117").unwrap().n, "British Airways");
        assert_eq!(t.airline_for_callsign("ezy81kp ").unwrap().n, "easyJet");
        assert!(t.airline_for_callsign("GDBCG").is_none());
        assert!(t.airline_for_callsign("N123AB").is_none());
        assert!(t.airline_for_callsign("").is_none());
    }

    #[test]
    fn categories() {
        let t = Tables::load();
        let mk = |ty: &str, cat: &str, flags: u32| Aircraft {
            type_code: Some(ty.into()),
            adsb_category: Some(cat.into()),
            flags,
            ..Default::default()
        };
        assert_eq!(t.category(&mk("A320", "A3", 0)), Category::Narrowbody);
        assert_eq!(t.category(&mk("B77W", "A5", 0)), Category::Widebody);
        assert_eq!(t.category(&mk("C172", "A1", 0)), Category::Light);
        assert_eq!(t.category(&mk("EC35", "A7", 0)), Category::Helicopter);
        assert_eq!(t.category(&mk("A320", "A3", 1)), Category::Military);
        // unknown type falls back to ADS-B emitter category
        assert_eq!(t.category(&mk("QQQQ", "A7", 0)), Category::Helicopter);
        assert_eq!(t.category(&Aircraft::default()), Category::Unknown);
        assert!(t.is_rare(&mk("A388", "A5", 0)));
        assert!(!t.is_rare(&mk("A320", "A3", 0)));
    }

    #[test]
    fn planespotters_parse() {
        let body = r#"{"photos":[{"id":"1","thumbnail":{"src":"https://t.plnspttrs.net/a_200.jpg","size":{"width":200,"height":133}},
          "thumbnail_large":{"src":"https://t.plnspttrs.net/a_280.jpg","size":{"width":420,"height":280}},
          "link":"https://www.planespotters.net/photo/1/x","photographer":"Jane Doe"}]}"#;
        let p = parse_planespotters(body).unwrap();
        assert_eq!(p.photographer, "Jane Doe");
        assert!(p.src.ends_with("a_280.jpg"));
        assert_eq!((p.width, p.height), (420, 280));
        assert!(parse_planespotters(r#"{"photos":[]}"#).is_none());
        assert!(parse_planespotters("garbage").is_none());
        assert!(parse_planespotters(r#"{"photos":[{"link":"x"}]}"#).is_none());
    }

    #[tokio::test]
    async fn photo_lookup_failure_is_none_and_negative_cached() {
        let cfg = PhotoCfg { base_url: "http://127.0.0.1:9".into(), timeout: Duration::from_millis(300), ..Default::default() };
        let dir = tempfile::tempdir().unwrap();
        let l = PhotoLookup::new(cfg, Some(dir.path().to_path_buf()));
        assert!(l.lookup("abc123", Some("G-TEST")).await.is_none());
        // second call answered from the negative cache (no network needed)
        assert_eq!(l.cached("abc123"), Some(None));
        assert!(NoRoutes.route("BAW1").is_none());
    }
}
