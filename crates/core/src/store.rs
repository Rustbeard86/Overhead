//! SQLite persistence for the Today log. 90-day retention, CSV export.

use chrono::{Local, TimeZone, Timelike};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Sighting {
    pub id: Option<i64>,
    pub hex: String,
    pub callsign: Option<String>,
    pub registration: Option<String>,
    pub type_code: Option<String>,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
    pub min_dist_km: f64,
    pub closest_ms: i64,
    pub min_alt_ft: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TodayStats {
    pub count: u32,
    pub closest_km: Option<f64>,
    pub lowest_ft: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TodayData {
    pub stats: TodayStats,
    /// Sightings per local hour of day, 24 entries.
    pub hourly: Vec<u32>,
    pub sightings: Vec<Sighting>,
}

pub struct Store {
    conn: Mutex<Connection>,
}

pub const RETENTION_DAYS: i64 = 90;

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS sightings (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               hex TEXT NOT NULL,
               callsign TEXT, registration TEXT, type_code TEXT,
               first_seen_ms INTEGER NOT NULL,
               last_seen_ms INTEGER NOT NULL,
               min_dist_km REAL NOT NULL,
               closest_ms INTEGER NOT NULL,
               min_alt_ft REAL
             );
             CREATE INDEX IF NOT EXISTS idx_sightings_first ON sightings(first_seen_ms);",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Insert (id None) or update a sighting. Returns its row id.
    pub fn upsert(&self, s: &Sighting) -> rusqlite::Result<i64> {
        let c = self.conn.lock().unwrap();
        match s.id {
            Some(id) => {
                c.execute(
                    "UPDATE sightings SET callsign=?2, registration=?3, type_code=?4, last_seen_ms=?5,
                     min_dist_km=?6, closest_ms=?7, min_alt_ft=?8 WHERE id=?1",
                    params![id, s.callsign, s.registration, s.type_code, s.last_seen_ms, s.min_dist_km, s.closest_ms, s.min_alt_ft],
                )?;
                Ok(id)
            }
            None => {
                c.execute(
                    "INSERT INTO sightings (hex,callsign,registration,type_code,first_seen_ms,last_seen_ms,min_dist_km,closest_ms,min_alt_ft)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![s.hex, s.callsign, s.registration, s.type_code, s.first_seen_ms, s.last_seen_ms, s.min_dist_km, s.closest_ms, s.min_alt_ft],
                )?;
                Ok(c.last_insert_rowid())
            }
        }
    }

    fn row(r: &rusqlite::Row) -> rusqlite::Result<Sighting> {
        Ok(Sighting {
            id: Some(r.get(0)?),
            hex: r.get(1)?,
            callsign: r.get(2)?,
            registration: r.get(3)?,
            type_code: r.get(4)?,
            first_seen_ms: r.get(5)?,
            last_seen_ms: r.get(6)?,
            min_dist_km: r.get(7)?,
            closest_ms: r.get(8)?,
            min_alt_ft: r.get(9)?,
        })
    }

    pub fn since(&self, since_ms: i64) -> rusqlite::Result<Vec<Sighting>> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare(
            "SELECT id,hex,callsign,registration,type_code,first_seen_ms,last_seen_ms,min_dist_km,closest_ms,min_alt_ft
             FROM sightings WHERE first_seen_ms >= ?1 ORDER BY first_seen_ms DESC",
        )?;
        let rows = st.query_map(params![since_ms], Self::row)?;
        rows.collect()
    }

    pub fn stats_since(&self, since_ms: i64) -> rusqlite::Result<TodayStats> {
        let c = self.conn.lock().unwrap();
        c.query_row(
            "SELECT COUNT(DISTINCT hex), MIN(min_dist_km), MIN(min_alt_ft) FROM sightings WHERE first_seen_ms >= ?1",
            params![since_ms],
            |r| Ok(TodayStats { count: r.get::<_, i64>(0)? as u32, closest_km: r.get(1)?, lowest_ft: r.get(2)? }),
        )
    }

    pub fn last_seen(&self) -> rusqlite::Result<Option<Sighting>> {
        let c = self.conn.lock().unwrap();
        let mut st = c.prepare(
            "SELECT id,hex,callsign,registration,type_code,first_seen_ms,last_seen_ms,min_dist_km,closest_ms,min_alt_ft
             FROM sightings ORDER BY last_seen_ms DESC, id DESC LIMIT 1",
        )?;
        let mut rows = st.query_map([], Self::row)?;
        rows.next().transpose()
    }

    /// Delete rows older than the retention window. Returns rows removed.
    pub fn prune(&self, now_ms: i64) -> rusqlite::Result<usize> {
        let cutoff = now_ms - RETENTION_DAYS * 86_400_000;
        self.conn.lock().unwrap().execute("DELETE FROM sightings WHERE last_seen_ms < ?1", params![cutoff])
    }

    pub fn today(&self, now_ms: i64) -> rusqlite::Result<TodayData> {
        let start = local_day_start_ms(now_ms);
        let sightings = self.since(start)?;
        let mut hourly = vec![0u32; 24];
        for s in &sightings {
            if let Some(dt) = Local.timestamp_millis_opt(s.first_seen_ms).single() {
                hourly[dt.hour() as usize] += 1;
            }
        }
        Ok(TodayData { stats: self.stats_since(start)?, hourly, sightings })
    }

    /// CSV of every stored sighting (or since `since_ms`), oldest first.
    pub fn export_csv(&self, since_ms: i64) -> rusqlite::Result<String> {
        let mut rows = self.since(since_ms)?;
        rows.reverse();
        let mut out = String::from("hex,callsign,registration,type,first_seen,last_seen,closest_km,closest_time,min_altitude_ft\n");
        let esc = |s: &str| if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
        let iso = |ms: i64| Local.timestamp_millis_opt(ms).single().map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
        for s in rows {
            out.push_str(&format!(
                "{},{},{},{},{},{},{:.2},{},{}\n",
                esc(&s.hex),
                esc(s.callsign.as_deref().unwrap_or("")),
                esc(s.registration.as_deref().unwrap_or("")),
                esc(s.type_code.as_deref().unwrap_or("")),
                iso(s.first_seen_ms),
                iso(s.last_seen_ms),
                s.min_dist_km,
                iso(s.closest_ms),
                s.min_alt_ft.map(|a| format!("{a:.0}")).unwrap_or_default()
            ));
        }
        Ok(out)
    }
}

pub fn local_day_start_ms(now_ms: i64) -> i64 {
    let dt = Local.timestamp_millis_opt(now_ms).single().unwrap_or_else(Local::now);
    dt.date_naive().and_hms_opt(0, 0, 0).and_then(|n| Local.from_local_datetime(&n).earliest()).map(|d| d.timestamp_millis()).unwrap_or(now_ms - 86_400_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(hex: &str, first: i64, dist: f64) -> Sighting {
        Sighting {
            id: None,
            hex: hex.into(),
            callsign: Some("TST1".into()),
            registration: None,
            type_code: Some("A320".into()),
            first_seen_ms: first,
            last_seen_ms: first + 1000,
            min_dist_km: dist,
            closest_ms: first,
            min_alt_ft: Some(5000.0),
        }
    }

    #[test]
    fn upsert_updates_and_stats() {
        let st = Store::in_memory().unwrap();
        let now = chrono::Local::now().timestamp_millis();
        let id = st.upsert(&s("a", now, 8.0)).unwrap();
        let mut u = s("a", now, 3.0);
        u.id = Some(id);
        u.min_alt_ft = Some(2000.0);
        st.upsert(&u).unwrap();
        st.upsert(&s("b", now, 12.0)).unwrap();
        let d = st.today(now + 5000).unwrap();
        assert_eq!(d.stats.count, 2);
        assert_eq!(d.stats.closest_km, Some(3.0));
        assert_eq!(d.stats.lowest_ft, Some(2000.0));
        assert_eq!(d.sightings.len(), 2);
        assert_eq!(d.hourly.iter().sum::<u32>(), 2);
        assert_eq!(st.last_seen().unwrap().unwrap().hex, "b");
    }

    #[test]
    fn retention_prunes_90_days() {
        let st = Store::in_memory().unwrap();
        let now = 2_000_000_000_000i64;
        st.upsert(&s("old", now - 91 * 86_400_000, 5.0)).unwrap();
        st.upsert(&s("new", now - 89 * 86_400_000, 5.0)).unwrap();
        assert_eq!(st.prune(now).unwrap(), 1);
        let left = st.since(0).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].hex, "new");
    }

    #[test]
    fn csv_export_shape_and_escaping() {
        let st = Store::in_memory().unwrap();
        let mut a = s("a", 1_700_000_000_000, 4.256);
        a.callsign = Some("A,\"B\"".into());
        st.upsert(&a).unwrap();
        st.upsert(&s("b", 1_700_000_100_000, 9.0)).unwrap();
        let csv = st.export_csv(0).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("hex,callsign,"));
        assert!(lines[1].starts_with("a,\"A,\"\"B\"\"\","));
        assert!(lines[1].contains(",4.26,"));
        assert!(lines[2].starts_with("b,"));
    }

    #[test]
    fn file_store_persists() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("t.db");
        {
            let st = Store::open(&p).unwrap();
            st.upsert(&s("a", 5, 1.0)).unwrap();
        }
        assert_eq!(Store::open(&p).unwrap().since(0).unwrap().len(), 1);
    }
}
