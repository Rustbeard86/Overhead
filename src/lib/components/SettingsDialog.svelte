<script lang="ts">
  import { Dialog, Tabs, ToggleGroup } from 'bits-ui';
  import { X, ChevronUp, ChevronDown } from '@lucide/svelte';
  import { app } from '../store.svelte';
  import Row from './Row.svelte';
  import Toggle from './Toggle.svelte';
  import Range from './Range.svelte';
  import HomePicker from './HomePicker.svelte';
  import { parseCoords } from '../coords';
  import { altFromFt, CEILING_RANGES, DIST_RANGES, distFromKm, fmtDist, ftFromAlt, kmFromDist, snapTo } from '../units';
  import { NM_M } from '../geo';
  import type { ProviderId, ThemeName } from '../types';

  const s = $derived(app.settings);
  const nmFromKm = (km: number) => (km * 1000) / NM_M;
  const kmFromNm = (nm: number) => (nm * NM_M) / 1000;
  const clean = (v: number) => String(+v.toFixed(2));
  const dist = $derived(app.settings.units.dist);
  const uAll = $derived(app.settings.units);
  const rad = $derived(DIST_RANGES.radius[dist]);
  const off = $derived(DIST_RANGES.offset[dist]);
  const fly = $derived(DIST_RANGES.flyover[dist]);
  const ceil = $derived(CEILING_RANGES[uAll.alt]);

  let lat = $state<number | null>(null);
  let lon = $state<number | null>(null);
  let seeded = false;
  $effect(() => {
    if (app.settingsOpen && !seeded) {
      lat = s.home?.lat ?? null;
      lon = s.home?.lon ?? null;
      seeded = true;
    }
    if (!app.settingsOpen) seeded = false;
  });
  const num = (v: string) => (v.trim() === '' ? null : Number(v));
  // Pasting \"lat, lon\" (or a Google Maps URL) into either box fills both.
  function setField(which: 'lat' | 'lon', v: string) {
    const pair = parseCoords(v);
    if (pair) {
      lat = pair.lat;
      lon = pair.lon;
    } else if (which === 'lat') lat = num(v);
    else lon = num(v);
  }
  const validHome = $derived(lat != null && lon != null && Math.abs(lat) <= 90 && Math.abs(lon) <= 180);
  const hm = (m: number) => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
  const toMin = (v: string) => {
    const [h, m] = v.split(':').map(Number);
    return (h || 0) * 60 + (m || 0);
  };
  const NAMES: Record<ProviderId, string> = { 'airplanes-live': 'airplanes.live', 'adsb-fi': 'adsb.fi', 'adsb-lol': 'adsb.lol', opensky: 'OpenSky' };
  const ALL: ProviderId[] = ['airplanes-live', 'adsb-fi', 'adsb-lol', 'opensky'];
  function move(i: number, d: number) {
    app.update((x) => {
      const o = x.provider_order;
      const j = i + d;
      if (j < 0 || j >= o.length) return;
      [o[i], o[j]] = [o[j], o[i]];
    });
  }
  const themes: { id: ThemeName; name: string; bg: string; fg: string }[] = [
    { id: 'phosphor', name: 'Phosphor', bg: '#040806', fg: '#3dff8a' },
    { id: 'amber', name: 'Amber', bg: '#0a0603', fg: '#ffb21e' },
    { id: 'daylight', name: 'Daylight', bg: '#e9e5d9', fg: '#0b7a44' },
  ];
</script>

<Dialog.Root bind:open={app.settingsOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="dialog" style="height: min(42rem, calc(100vh - 1.5rem))">
      <div class="flex items-center justify-between border-b border-line px-5 py-3">
        <Dialog.Title class="font-display text-xl font-bold uppercase tracking-[0.12em]">Settings</Dialog.Title>
        <Dialog.Close class="btn btn-icon btn-ghost" aria-label="Close settings"><X size={16} /></Dialog.Close>
      </div>
      <Tabs.Root value="location" class="flex min-h-0 flex-1 flex-col">
        <Tabs.List class="flex gap-0 overflow-x-auto border-b border-line px-3" aria-label="Settings sections">
          {#each [['location', 'Location'], ['display', 'Display'], ['alerts', 'Alerts'], ['data', 'Data'], ['window', 'Window']] as [id, name] (id)}
            <Tabs.Trigger value={id} class="vtab">{name}</Tabs.Trigger>
          {/each}
        </Tabs.List>
        <div class="min-h-0 flex-1 overflow-y-auto px-5 pb-4">
          <Tabs.Content value="location" class="pt-3">
            <HomePicker bind:lat bind:lon />
            <div class="mt-3 grid grid-cols-[1fr_1fr_auto] items-end gap-3">
              <label class="flex flex-col gap-1"><span class="label">Latitude</span>
                <input class="field" inputmode="decimal" value={lat ?? ''} oninput={(e) => setField('lat', e.currentTarget.value)} /></label>
              <label class="flex flex-col gap-1"><span class="label">Longitude</span>
                <input class="field" inputmode="decimal" value={lon ?? ''} oninput={(e) => setField('lon', e.currentTarget.value)} /></label>
              <button class="btn btn-primary" disabled={!validHome} onclick={() => app.setHome(lat!, lon!)}>Apply</button>
            </div>
            <div class="mt-4">
              <Row label="View radius" hint={`Providers are queried ${fmtDist(kmFromNm(10), uAll, 0)} further out.`}>
                <Range label="View radius" unit={dist} fmt={clean} min={rad[0]} max={rad[1]} step={rad[2]} value={snapTo(distFromKm(kmFromNm(s.view_radius_nm), uAll), rad[2])} onchange={(v) => app.update((x) => (x.view_radius_nm = nmFromKm(kmFromDist(v, uAll))))} />
              </Row>
              <Row label="Privacy offset" hint="Query around a fixed displaced point; everything is still computed from your real home locally.">
                <Toggle label="Privacy offset" checked={s.privacy.enabled} onchange={(v) => app.update((x) => (x.privacy.enabled = v))} />
              </Row>
              {#if s.privacy.enabled}
                <Row label="Offset distance">
                  <Range label="Offset distance" unit={dist} fmt={clean} min={off[0]} max={off[1]} step={off[2]} value={snapTo(distFromKm(s.privacy.offset_km, uAll), off[2])} onchange={(v) => app.update((x) => (x.privacy.offset_km = kmFromDist(v, uAll)))} />
                </Row>
              {/if}
            </div>
          </Tabs.Content>

          <Tabs.Content value="display" class="pt-1">
            <Row label="Theme">
              <div class="flex gap-2" role="radiogroup" aria-label="Theme">
                {#each themes as t (t.id)}
                  <button
                    class="flex flex-col items-center gap-1 border p-1.5"
                    style:border-color={s.theme === t.id ? 'var(--accent)' : 'var(--line-strong)'}
                    role="radio" aria-checked={s.theme === t.id} aria-label={t.name}
                    onclick={() => app.update((x) => (x.theme = t.id))}
                  >
                    <span class="grid h-9 w-14 place-items-center" style:background={t.bg}>
                      <span class="h-4 w-4 rounded-full border-2" style:border-color={t.fg}></span>
                    </span>
                    <span class="label">{t.name}</span>
                  </button>
                {/each}
              </div>
            </Row>
            <Row label="Distance units">
              <ToggleGroup.Root type="single" class="seg" value={s.units.dist} onValueChange={(v) => v && app.update((x) => (x.units.dist = v as 'nm' | 'km' | 'mi'))} aria-label="Distance units">
                {#each ['nm', 'km', 'mi'] as v (v)}<ToggleGroup.Item value={v}>{v}</ToggleGroup.Item>{/each}
              </ToggleGroup.Root>
            </Row>
            <Row label="Altitude units">
              <ToggleGroup.Root type="single" class="seg" value={s.units.alt} onValueChange={(v) => v && app.update((x) => (x.units.alt = v as 'ft' | 'm'))} aria-label="Altitude units">
                {#each ['ft', 'm'] as v (v)}<ToggleGroup.Item value={v}>{v}</ToggleGroup.Item>{/each}
              </ToggleGroup.Root>
            </Row>
            <Row label="Trail length" hint="How long each aircraft's trail is drawn.">
              <Range label="Trail length" unit="min" min={1} max={15} step={1} value={s.trail_min} onchange={(v) => app.update((x) => (x.trail_min = v))} />
            </Row>
          </Tabs.Content>

          <Tabs.Content value="alerts" class="pt-1">
            <Row label="Flyover" hint="Alert when an aircraft passes both this close and this low.">
              <Toggle label="Flyover alerts" checked={s.alerts.flyover} onchange={(v) => app.update((x) => (x.alerts.flyover = v))} />
            </Row>
            {#if s.alerts.flyover}
              <Row label="Flyover distance">
                <Range label="Flyover distance" unit={dist} fmt={clean} min={fly[0]} max={fly[1]} step={fly[2]} value={snapTo(distFromKm(s.alerts.flyover_km, uAll), fly[2])} onchange={(v) => app.update((x) => (x.alerts.flyover_km = kmFromDist(v, uAll)))} />
              </Row>
              <Row label="Flyover ceiling">
                <Range label="Flyover ceiling" unit={uAll.alt} fmt={(v) => Math.round(v).toLocaleString('en-US')} min={ceil[0]} max={ceil[1]} step={ceil[2]} value={snapTo(altFromFt(s.alerts.flyover_max_alt_ft, uAll), ceil[2])} onchange={(v) => app.update((x) => (x.alerts.flyover_max_alt_ft = Math.round(ftFromAlt(v, uAll))))} />
              </Row>
            {/if}
            <Row label="Military"><Toggle label="Military alerts" checked={s.alerts.military} onchange={(v) => app.update((x) => (x.alerts.military = v))} /></Row>
            <Row label="Emergency squawks" hint="7500, 7600, 7700. Always audible, even in quiet hours."><Toggle label="Emergency alerts" checked={s.alerts.emergency} onchange={(v) => app.update((x) => (x.alerts.emergency = v))} /></Row>
            <Row label="Watch list" hint="Callsign prefixes, registrations, hex codes or type codes, comma separated.">
              <Toggle label="Watch list" checked={s.alerts.watch_enabled} onchange={(v) => app.update((x) => (x.alerts.watch_enabled = v))} />
            </Row>
            {#if s.alerts.watch_enabled}
              <div class="border-b border-line py-2.5">
                <input class="field" placeholder="BAW, A388, G-EUPT" value={s.alerts.watchlist.join(', ')} onchange={(e) => app.update((x) => (x.alerts.watchlist = e.currentTarget.value.split(',').map((t) => t.trim()).filter(Boolean)))} aria-label="Watch list entries" />
              </div>
            {/if}
            <Row label="Desktop notifications"><Toggle label="Desktop notifications" checked={s.alerts.desktop_notifications} onchange={(v) => app.update((x) => (x.alerts.desktop_notifications = v))} /></Row>
            <Row label="Cooldown" hint="Minimum gap between repeat alerts for the same rule and aircraft.">
              <Range label="Cooldown" unit="min" min={0} max={120} step={5} value={s.alerts.cooldown_min} onchange={(v) => app.update((x) => (x.alerts.cooldown_min = v))} />
            </Row>
            <Row label="Quiet hours" hint="No toasts or notifications. Emergencies still alert.">
              <Toggle label="Quiet hours" checked={s.alerts.quiet.enabled} onchange={(v) => app.update((x) => (x.alerts.quiet.enabled = v))} />
            </Row>
            {#if s.alerts.quiet.enabled}
              <div class="flex items-center gap-3 border-b border-line py-2.5">
                <input type="time" class="field" value={hm(s.alerts.quiet.start_min)} onchange={(e) => app.update((x) => (x.alerts.quiet.start_min = toMin(e.currentTarget.value)))} aria-label="Quiet hours start" />
                <span class="label">to</span>
                <input type="time" class="field" value={hm(s.alerts.quiet.end_min)} onchange={(e) => app.update((x) => (x.alerts.quiet.end_min = toMin(e.currentTarget.value)))} aria-label="Quiet hours end" />
              </div>
            {/if}
          </Tabs.Content>

          <Tabs.Content value="data" class="pt-1">
            <Row label="Poll interval" hint="Time between requests. Never under 2 s, to respect provider limits.">
              <Range label="Poll interval" unit="s" min={2} max={30} step={1} value={s.poll_interval_s} onchange={(v) => app.update((x) => (x.poll_interval_s = v))} />
            </Row>
            <div class="border-b border-line py-2.5">
              <div class="font-display text-sm font-semibold uppercase tracking-[0.08em]">Provider order</div>
              <div class="mb-2 text-xs text-ink3">Tried top to bottom; failures switch to the next at once.</div>
              <ol class="flex flex-col gap-1">
                {#each s.provider_order as id, i (id)}
                  <li class="flex items-center gap-2 border border-line bg-panel2 px-3 py-1.5">
                    <span class="num w-4 text-ink3">{i + 1}</span>
                    <span class="flex-1">{NAMES[id]}</span>
                    <button class="btn btn-icon btn-ghost !h-6 !w-6" aria-label={`Move ${NAMES[id]} up`} disabled={i === 0} onclick={() => move(i, -1)}><ChevronUp size={14} /></button>
                    <button class="btn btn-icon btn-ghost !h-6 !w-6" aria-label={`Move ${NAMES[id]} down`} disabled={i === s.provider_order.length - 1} onclick={() => move(i, 1)}><ChevronDown size={14} /></button>
                  </li>
                {/each}
              </ol>
              {#if ALL.some((id) => !s.provider_order.includes(id))}
                <div class="mt-2 flex flex-wrap gap-2">
                  {#each ALL.filter((id) => !s.provider_order.includes(id)) as id (id)}
                    <button class="btn" onclick={() => app.update((x) => x.provider_order.push(id))}>+ {NAMES[id]}</button>
                  {/each}
                </div>
              {/if}
            </div>
            <Row label="Replay mode" hint="Play a recorded file instead of polling providers.">
              <Toggle label="Replay mode" checked={s.replay_enabled} onchange={(v) => app.update((x) => (x.replay_enabled = v))} />
            </Row>
            {#if s.replay_enabled}
              <div class="border-b border-line py-2.5">
                <input class="field" placeholder="/path/to/replay.json (empty = built-in demo)" value={s.replay_file ?? ''} onchange={(e) => app.update((x) => (x.replay_file = e.currentTarget.value.trim() || null))} aria-label="Replay file path" />
              </div>
            {/if}
          </Tabs.Content>

          <Tabs.Content value="window" class="pt-1">
            <Row label="Always on top"><Toggle label="Always on top" checked={s.window.always_on_top} onchange={(v) => app.update((x) => (x.window.always_on_top = v))} /></Row>
            <Row label="Frameless window" hint="Drag from the top bar."><Toggle label="Frameless window" checked={s.window.frameless} onchange={(v) => app.update((x) => (x.window.frameless = v))} /></Row>
            <Row label="Kiosk mode" hint="Full screen, the interface hides when idle. Esc exits."><Toggle label="Kiosk mode" checked={s.window.kiosk} onchange={(v) => app.update((x) => (x.window.kiosk = v))} /></Row>
            <Row label="Tray icon"><Toggle label="Tray icon" checked={s.window.tray} onchange={(v) => app.update((x) => (x.window.tray = v))} /></Row>
          </Tabs.Content>
        </div>
      </Tabs.Root>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.vtab) {
    height: 2.4rem;
    padding: 0 0.9rem;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--text-sm);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink-2);
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
  }
  :global(.vtab:hover) {
    color: var(--ink);
  }
  :global(.vtab[data-state='active']) {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
</style>
