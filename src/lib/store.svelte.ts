import { toast } from 'svelte-sonner';
import { backend, isTauri, type Backend } from './ipc';
import { Scene, linkAlphaFor } from './scene';
import { applyTheme } from './theme';
import { defaultSettings } from './types';
import type { AircraftView, AlertEvent, EnrichmentEvent, Settings, Snapshot, ViewName } from './types';

export const scene = new Scene();

class App {
  snapshot = $state.raw<Snapshot | null>(null);
  settings = $state<Settings>(defaultSettings());
  view = $state<ViewName>('radar');
  hoverHex = $state<string | null>(null);
  pinnedHex = $state<string | null>(null);
  enrich = $state.raw<Record<string, EnrichmentEvent>>({});
  panelOpen = $state(false);
  settingsOpen = $state(false);
  todayOpen = $state(false);
  onboardingOpen = $state(false);
  chromeHidden = $state(false);
  ready = $state(false);
  sort = $state<'dist' | 'alt' | 'speed'>('dist');
  mapNotice = $state<string | null>(null);

  #backend: Backend | null = null;
  #saveTimer: ReturnType<typeof setTimeout> | undefined;

  featuredHex = $derived(this.snapshot?.featured ?? null);
  /** Plane shown in the Overhead card: the pinned plane, else the featured one. */
  cardPlane = $derived.by<AircraftView | null>(() => {
    const s = this.snapshot;
    if (!s) return null;
    const hex = this.pinnedHex ?? s.featured;
    return s.aircraft.find((a) => a.hex === hex) ?? null;
  });
  inView = $derived(this.snapshot?.aircraft.filter((a) => a.in_view) ?? []);

  async init() {
    const b = await backend();
    this.#backend = b;
    this.settings = await b.getSettings();
    applyTheme(this.settings.theme);
    const q = new URLSearchParams(location.search);
    const v = q.get('view');
    if (v === 'radar' || v === 'map' || v === 'sky') this.view = v;
    if (q.get('panel') === '1') this.panelOpen = true;
    await b.onSnapshot((s) => this.#onSnapshot(s));
    await b.onEnrichment((e) => {
      this.enrich = { ...this.enrich, [e.hex]: e };
    });
    await b.onAlert((a) => this.#onAlert(a));
    this.ready = true;
    if (!this.settings.first_run_done && !q.has('home')) this.onboardingOpen = true;
  }

  #onSnapshot(s: Snapshot) {
    this.snapshot = s;
    scene.linkAlpha = linkAlphaFor(s.data_age_s);
    scene.ingest(s, performance.now());
    this.pinnedHex = s.pinned;
  }

  #onAlert(a: AlertEvent) {
    if (a.notify) {
      const fn = a.kind === 'emergency' ? toast.error : a.kind === 'military' ? toast.warning : toast.info;
      fn(a.title, { description: a.detail, duration: a.kind === 'emergency' ? 12000 : 7000, onClick: () => this.pin(a.hex) } as never);
      if (this.settings.alerts.desktop_notifications && isTauri) {
        import('@tauri-apps/plugin-notification')
          .then(async (n) => {
            let ok = await n.isPermissionGranted();
            if (!ok) ok = (await n.requestPermission()) === 'granted';
            if (ok) n.sendNotification({ title: a.title, body: a.detail });
          })
          .catch(() => {});
      }
    }
  }

  setView(v: ViewName) {
    this.view = v;
  }

  hover(hex: string | null) {
    this.hoverHex = hex;
  }

  /** Click: pin a plane (or unpin when it is already pinned). */
  async pin(hex: string | null) {
    if (hex && hex === this.pinnedHex) hex = null;
    this.pinnedHex = hex;
    await this.#backend?.pin(hex);
    if (hex) await this.#backend?.select(hex);
  }

  async update(patch: (s: Settings) => void) {
    const next = structuredClone($state.snapshot(this.settings)) as Settings;
    patch(next);
    this.settings = next;
    if (next.theme) applyTheme(next.theme);
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => this.#backend?.setSettings($state.snapshot(this.settings) as Settings), 150);
  }

  async setHome(lat: number, lon: number) {
    await this.update((s) => {
      s.home = { lat, lon };
      s.first_run_done = true;
    });
  }

  async finishOnboarding() {
    await this.update((s) => {
      s.first_run_done = true;
    });
    this.onboardingOpen = false;
  }

  today() {
    return this.#backend!.today();
  }
  exportCsv() {
    return this.#backend!.exportCsv();
  }
  win() {
    return this.#backend;
  }

  cycle(dir: 1 | -1) {
    const list = this.inView.filter((a) => !a.on_ground);
    if (!list.length) return;
    const i = list.findIndex((a) => a.hex === (this.pinnedHex ?? this.featuredHex));
    const next = list[(i + dir + list.length) % list.length];
    this.pin(next.hex);
  }
}

export const app = new App();
