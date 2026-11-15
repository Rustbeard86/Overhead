// IPC layer: Tauri in the app, a synthetic in-browser engine otherwise
// (dev server and headless screenshots).

import type { AlertEvent, EnrichmentEvent, Settings, Snapshot, TodayData } from './types';
import { defaultSettings } from './types';
import { MockEngine, mockToday } from './mock/engine';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface Backend {
  getSettings(): Promise<Settings>;
  setSettings(s: Settings): Promise<void>;
  pin(hex: string | null): Promise<void>;
  select(hex: string): Promise<void>;
  today(): Promise<TodayData>;
  exportCsv(): Promise<string | null>;
  onSnapshot(cb: (s: Snapshot) => void): Promise<() => void>;
  onEnrichment(cb: (e: EnrichmentEvent) => void): Promise<() => void>;
  onAlert(cb: (a: AlertEvent) => void): Promise<() => void>;
  exitApp?(): Promise<void>;
  minimize?(): Promise<void>;
  startDragging?(): Promise<void>;
}

async function tauriBackend(): Promise<Backend> {
  const { invoke } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const on = <T>(name: string, cb: (p: T) => void) => listen<T>(name, (e) => cb(e.payload));
  return {
    getSettings: () => invoke<Settings>('get_settings'),
    setSettings: (s) => invoke('set_settings', { settings: s }),
    pin: (hex) => invoke('pin', { hex }),
    select: (hex) => invoke('select_plane', { hex }),
    today: () => invoke<TodayData>('today'),
    exportCsv: async () => {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({ defaultPath: 'overhead-sightings.csv', filters: [{ name: 'CSV', extensions: ['csv'] }] });
      if (!path) return null;
      await invoke('export_csv', { path });
      return path;
    },
    onSnapshot: (cb) => on<Snapshot>('snapshot', cb),
    onEnrichment: (cb) => on<EnrichmentEvent>('enrichment', cb),
    onAlert: (cb) => on<AlertEvent>('alert', cb),
    exitApp: () => getCurrentWindow().close(),
    minimize: () => getCurrentWindow().minimize(),
    startDragging: () => getCurrentWindow().startDragging(),
  };
}

function mockBackend(): Backend {
  const q = new URLSearchParams(location.search);
  const t0 = Number(q.get('t') ?? 40);
  const engine = new MockEngine(t0);
  let settings: Settings = defaultSettings();
  try {
    const saved = localStorage.getItem('overhead-mock-settings');
    if (saved) settings = { ...settings, ...JSON.parse(saved) };
  } catch { /* ignore */ }
  if (q.get('theme')) settings.theme = q.get('theme') as Settings['theme'];
  if (q.get('home') === '1') settings = { ...settings, home: { lat: 51.5074, lon: -0.1278 }, first_run_done: true };
  if (q.get('fresh') === '1') settings = { ...defaultSettings(), theme: settings.theme };
  const start = performance.now();
  const snapCbs = new Set<(s: Snapshot) => void>();
  const enrCbs = new Set<(e: EnrichmentEvent) => void>();
  const alertCbs = new Set<(a: AlertEvent) => void>();
  let timer: number | undefined;
  const tick = () => {
    engine.radiusNm = settings.view_radius_nm;
    const s = engine.snapshot(Date.now(), start + (Date.now() - performance.now()));
    snapCbs.forEach((cb) => cb(s));
  };
  const ensure = () => {
    if (timer == null) {
      timer = window.setInterval(tick, (settings.poll_interval_s || 3) * 1000);
      setTimeout(tick, 30);
    }
  };
  return {
    getSettings: async () => settings,
    setSettings: async (s) => {
      settings = s;
      try { localStorage.setItem('overhead-mock-settings', JSON.stringify(s)); } catch { /* ignore */ }
      tick();
    },
    pin: async (hex) => { engine.pinned = hex; tick(); },
    select: async (hex) => { setTimeout(() => enrCbs.forEach((cb) => cb({ hex, photo: null })), 150); },
    today: async () => mockToday(),
    exportCsv: async () => {
      const d = mockToday();
      const csv = 'hex,callsign,type,closest_km,min_altitude_ft\n' + d.sightings.map((r) => `${r.hex},${r.callsign},${r.type_code},${r.min_dist_km.toFixed(2)},${r.min_alt_ft}`).join('\n');
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([csv], { type: 'text/csv' }));
      a.download = 'overhead-sightings-demo.csv';
      a.click();
      return a.download;
    },
    onSnapshot: async (cb) => { snapCbs.add(cb); ensure(); return () => snapCbs.delete(cb); },
    onEnrichment: async (cb) => { enrCbs.add(cb); return () => enrCbs.delete(cb); },
    onAlert: async (cb) => {
      alertCbs.add(cb);
      if (q.get('alerts') === '1') setTimeout(() => cb({ kind: 'emergency', hex: '4ca003', callsign: 'RYR42XE', title: 'RYR42XE: emergency', detail: 'Squawk 7700 (general emergency)', t_ms: Date.now(), notify: true }), 1500);
      return () => alertCbs.delete(cb);
    },
  };
}

let cached: Promise<Backend> | null = null;
export function backend(): Promise<Backend> {
  cached ??= isTauri ? tauriBackend() : Promise.resolve(mockBackend());
  return cached;
}
