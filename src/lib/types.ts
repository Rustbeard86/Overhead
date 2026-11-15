// Contract with the Rust core (snake_case JSON). Keep in sync with crates/core.

export type Category = 'narrowbody' | 'widebody' | 'light' | 'helicopter' | 'military' | 'unknown';
export type Badge = 'military' | 'emergency' | 'heavy' | 'helicopter' | 'rare' | 'watch';
export type ThemeName = 'phosphor' | 'amber' | 'daylight';
export type ViewName = 'radar' | 'map' | 'sky';
export type ProviderId = 'airplanes-live' | 'adsb-fi' | 'adsb-lol' | 'opensky';

export interface AircraftView {
  hex: string;
  callsign: string | null;
  registration: string | null;
  type_code: string | null;
  type_name: string | null;
  airline: string | null;
  airline_color: string | null;
  category: Category;
  lat: number;
  lon: number;
  e_m: number;
  n_m: number;
  ve_ms: number;
  vn_ms: number;
  alt_ft: number | null;
  alt_geom_ft: number | null;
  on_ground: boolean;
  gs_kt: number | null;
  track_deg: number | null;
  vrate_fpm: number | null;
  squawk: string | null;
  seen_pos_s: number;
  dist_km: number;
  bearing_deg: number;
  elev_deg: number | null;
  slant_km: number;
  ca_t_s: number | null;
  ca_d_km: number | null;
  score: number;
  badges: Badge[];
  stale: boolean;
  in_view: boolean;
  /** [east_m, north_m, age_s, alt_ft], oldest first */
  trail: [number, number, number, number][];
}

export interface ProviderStatus {
  id: ProviderId;
  name: string;
  state: 'idle' | 'ok' | 'backoff';
  active: boolean;
  consecutive_failures: number;
  last_error: string | null;
  retry_in_s: number;
}

export type LinkState = 'connecting' | 'live' | 'degraded' | 'unavailable';
export type SourceKind = 'live' | 'replay' | 'demo';

export interface LastSeen {
  hex: string;
  callsign: string | null;
  type_code: string | null;
  t_ms: number;
  min_dist_km: number;
}

export interface TodayStats {
  count: number;
  closest_km: number | null;
  lowest_ft: number | null;
}

export interface Snapshot {
  seq: number;
  t_ms: number;
  source: SourceKind;
  link: LinkState;
  provider: string | null;
  data_age_s: number | null;
  providers: ProviderStatus[];
  home: { lat: number; lon: number } | null;
  home_is_demo: boolean;
  view_radius_nm: number;
  aircraft: AircraftView[];
  featured: string | null;
  pinned: string | null;
  last_seen: LastSeen | null;
  today: TodayStats;
}

export interface Photo {
  src: string;
  link: string;
  photographer: string;
  width: number;
  height: number;
}
export interface EnrichmentEvent {
  hex: string;
  photo: Photo | null;
}

export type AlertKind = 'flyover' | 'military' | 'emergency' | 'watch';
export interface AlertEvent {
  kind: AlertKind;
  hex: string;
  callsign: string | null;
  title: string;
  detail: string;
  t_ms: number;
  notify: boolean;
}

export interface Settings {
  home: { lat: number; lon: number } | null;
  view_radius_nm: number;
  poll_interval_s: number;
  trail_min: number;
  provider_order: ProviderId[];
  units: { dist: 'nm' | 'km' | 'mi'; alt: 'ft' | 'm' };
  theme: ThemeName;
  alerts: {
    flyover: boolean;
    flyover_km: number;
    flyover_max_alt_ft: number;
    military: boolean;
    emergency: boolean;
    watch_enabled: boolean;
    watchlist: string[];
    desktop_notifications: boolean;
    cooldown_min: number;
    quiet: { enabled: boolean; start_min: number; end_min: number };
  };
  privacy: { enabled: boolean; offset_km: number; bearing_deg: number };
  window: { always_on_top: boolean; frameless: boolean; kiosk: boolean; tray: boolean };
  replay_enabled: boolean;
  replay_file: string | null;
  first_run_done: boolean;
}

export interface Sighting {
  id: number | null;
  hex: string;
  callsign: string | null;
  registration: string | null;
  type_code: string | null;
  first_seen_ms: number;
  last_seen_ms: number;
  min_dist_km: number;
  closest_ms: number;
  min_alt_ft: number | null;
}
export interface TodayData {
  stats: TodayStats;
  hourly: number[];
  sightings: Sighting[];
}

export const defaultSettings = (): Settings => ({
  home: null,
  view_radius_nm: 40,
  poll_interval_s: 3,
  trail_min: 5,
  provider_order: ['airplanes-live', 'adsb-fi', 'adsb-lol'],
  units: { dist: 'nm', alt: 'ft' },
  theme: 'phosphor',
  alerts: {
    flyover: true,
    flyover_km: 3,
    flyover_max_alt_ft: 10000,
    military: true,
    emergency: true,
    watch_enabled: false,
    watchlist: [],
    desktop_notifications: false,
    cooldown_min: 15,
    quiet: { enabled: false, start_min: 22 * 60, end_min: 7 * 60 },
  },
  privacy: { enabled: false, offset_km: 5, bearing_deg: 137 },
  window: { always_on_top: false, frameless: false, kiosk: false, tray: true },
  replay_enabled: false,
  replay_file: null,
  first_run_done: false,
});
