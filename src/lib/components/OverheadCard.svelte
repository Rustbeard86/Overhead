<script lang="ts">
  import { fade } from 'svelte/transition';
  import { X, Pin } from '@lucide/svelte';
  import { app } from '../store.svelte';
  import { compass } from '../geo';
  import { fmtAgo, fmtAlt, fmtDist, fmtDur, fmtSpeed, fmtVRate } from '../units';
  import GlyphIcon from './GlyphIcon.svelte';
  import { openExternal } from '../external';
  import { CATEGORY_LABEL } from '../glyphs';
  import type { AircraftView, Badge } from '../types';

  let { compact = false }: { compact?: boolean } = $props();

  const p = $derived(app.cardPlane);
  const u = $derived(app.settings.units);
  const photo = $derived(p ? app.enrich[p.hex]?.photo ?? null : null);
  const pinned = $derived(!!p && app.pinnedHex === p.hex);
  const snap = $derived(app.snapshot);
  let photoFailed = $state<Record<string, boolean>>({});

  const BADGE_LABEL: Record<Badge, string> = {
    military: 'Military', emergency: 'Emergency', heavy: 'Heavy', helicopter: 'Helicopter', rare: 'Rare type', watch: 'Watch list',
  };

  function ca(v: AircraftView): string {
    if (v.ca_t_s == null || v.ca_d_km == null) return '';
    const when = v.ca_t_s < 5 ? 'now' : `in ${fmtDur(v.ca_t_s)}`;
    return `${when} at ${fmtDist(v.ca_d_km, u)}`;
  }
  function squawkNote(v: AircraftView): string {
    return v.squawk === '7500' ? 'hijack' : v.squawk === '7600' ? 'radio failure' : v.squawk === '7700' ? 'emergency' : '';
  }
  const trend = (v: AircraftView) => (v.on_ground ? 0 : (v.vrate_fpm ?? 0) > 250 ? 1 : (v.vrate_fpm ?? 0) < -250 ? -1 : 0);

  type Stat = { k: string; v: string; sub?: string; tone?: string };
  const stats = $derived.by<Stat[]>(() => {
    if (!p) return [];
    const out: Stat[] = [];
    const alt = p.on_ground ? 'On ground' : fmtAlt(p.alt_ft, u);
    if (alt) out.push({ k: 'Altitude', v: alt });
    const spd = fmtSpeed(p.gs_kt, u);
    if (spd && !p.on_ground) out.push({ k: 'Speed', v: spd });
    if (p.track_deg != null && !p.on_ground) out.push({ k: 'Heading', v: `${Math.round(p.track_deg).toString().padStart(3, '0')}°`, sub: compass(p.track_deg) });
    const vr = fmtVRate(p.vrate_fpm, u);
    if (vr && !p.on_ground) out.push({ k: 'Vertical', v: vr, tone: trend(p) === 1 ? 'up' : trend(p) === -1 ? 'down' : '' });
    out.push({ k: 'Distance', v: fmtDist(p.dist_km, u) });
    out.push({ k: 'Bearing', v: `${Math.round(p.bearing_deg).toString().padStart(3, '0')}°`, sub: compass(p.bearing_deg) });
    if (p.elev_deg != null) out.push({ k: 'Elevation', v: `${Math.round(p.elev_deg)}°`, sub: p.elev_deg < 0 ? 'below horizon' : undefined });
    if (p.squawk) out.push({ k: 'Squawk', v: p.squawk, sub: squawkNote(p), tone: squawkNote(p) ? 'danger' : '' });
    return out;
  });
  const livery = $derived(p?.airline_color ?? null);
</script>

<section class="relative flex flex-col overflow-hidden border border-line bg-panel" style:--livery-raw={livery ?? 'var(--accent)'} aria-live="polite" aria-label="Featured aircraft">
  {#if snap && !p}
    <!-- Idle / loading / empty states -->
    <div class="flex flex-col gap-3 p-4" in:fade={{ duration: 200 }}>
      {#if snap.link === 'connecting'}
        <span class="label">Acquiring data</span>
        <div class="h-28 animate-pulse border border-line bg-panel2"></div>
        <div class="h-8 w-2/3 animate-pulse bg-panel2"></div>
        <div class="h-4 w-1/2 animate-pulse bg-panel2"></div>
      {:else}
        <span class="label">{snap.link === 'unavailable' ? 'Data unavailable' : 'Quiet skies'}</span>
        <div class="grid h-28 place-items-center border border-dashed border-line2 bg-panel2 text-ink3">
          <svg viewBox="-50 -50 100 100" width="64" height="64" aria-hidden="true">
            <circle r="44" fill="none" stroke="currentColor" stroke-opacity=".35" stroke-width="2" />
            <circle r="24" fill="none" stroke="currentColor" stroke-opacity=".35" stroke-width="2" />
            <path d="M0 0 L40 -20" stroke="var(--accent)" stroke-width="2.5" />
            <circle r="3" fill="var(--accent)" />
          </svg>
        </div>
        {#if snap.link === 'unavailable'}
          <p class="text-sm text-ink2">Every provider is failing. Trying again automatically.</p>
        {:else}
          <p class="font-display text-xl font-semibold tracking-wide">Nothing in range</p>
          {#if snap.last_seen}
            <div class="text-sm text-ink2">
              <div class="label mb-1">Last plane seen</div>
              <div class="font-display text-lg font-bold text-ink">{snap.last_seen.callsign ?? snap.last_seen.hex.toUpperCase()}</div>
              <div>
                {#if snap.last_seen.type_code}{snap.last_seen.type_code} · {/if}{fmtAgo(snap.last_seen.t_ms)} · closest {fmtDist(snap.last_seen.min_dist_km, u)}
              </div>
            </div>
          {/if}
        {/if}
      {/if}
    </div>
  {:else if p}
    {#key p.hex}
      <div in:fade={{ duration: 260 }} class="flex flex-col">
        <!-- hero: photo, or a designed fallback -->
        {#if !compact}
          <div class="relative overflow-hidden border-b border-line bg-panel2" style="height: clamp(6.5rem, 19vh, 9.5rem); background: linear-gradient(135deg, color-mix(in srgb, var(--livery) 30%, var(--panel-2)), var(--panel-2) 72%)">
            {#if photo && !photoFailed[photo.src]}
              <img
                src={photo.src} alt={`Photo of ${p.registration ?? p.type_name ?? 'aircraft'}`} class="h-full w-full object-cover"
                onerror={() => (photoFailed = { ...photoFailed, [photo.src]: true })}
              />
              <div class="pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-black/70 to-transparent"></div>
              <button
                class="absolute bottom-1.5 right-2 text-[10px] tracking-wide text-white/90 underline decoration-white/40 underline-offset-2 hover:decoration-white"
                onclick={() => openExternal(photo.link)}
              >
                © {photo.photographer} · Planespotters.net
              </button>
            {:else}
              <div class="pointer-events-none absolute inset-0 opacity-[0.18]" style="background: repeating-radial-gradient(circle at 78% 62%, transparent 0 22px, var(--livery) 22px 23px)"></div>
              <GlyphIcon category={p.category} size={150} rotate={38} class="absolute -right-3 top-1/2 -translate-y-1/2 text-livery opacity-60" />
              <span class="label absolute bottom-2 left-3">{CATEGORY_LABEL[p.category]}</span>
            {/if}
            {#if p.badges.length}
              <div class="absolute left-2.5 top-2.5 flex flex-wrap gap-1.5">
                {#each p.badges as b (b)}
                  <span class="badge badge-{b} bg-panel/85">{BADGE_LABEL[b]}</span>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <div class="flex items-start gap-3 px-4 pb-1 pt-3" style="border-left: 4px solid var(--livery)">
          <div class="min-w-0 flex-1">
            <div class="label flex items-center gap-1.5">
              {#if pinned}<Pin size={11} /> Pinned{:else}Overhead now{/if}
            </div>
            <h2 class="truncate font-display font-bold leading-[1.05] tracking-wide" style="font-size: var(--text-display)">
              {p.callsign ?? p.hex.toUpperCase()}
            </h2>
            {#if p.airline}<div class="truncate font-display text-lg font-semibold text-ink">{p.airline}</div>{/if}
            {#if p.registration || p.type_name}
              <div class="truncate text-sm text-ink2">{[p.registration, p.type_name].filter(Boolean).join(' · ')}</div>
            {/if}
          </div>
          {#if pinned}
            <button class="btn btn-icon btn-ghost -mr-2 mt-0.5" aria-label="Unpin" onclick={() => app.pin(p.hex)}><X size={16} /></button>
          {/if}
        </div>
        {#if compact && p.badges.length}
          <div class="flex flex-wrap gap-1.5 px-4 pb-1">
            {#each p.badges as b (b)}<span class="badge badge-{b}">{BADGE_LABEL[b]}</span>{/each}
          </div>
        {/if}

        <dl class="grid grid-cols-2 gap-x-4 gap-y-2.5 px-4 pb-3 pt-3">
          {#each stats as s (s.k)}
            <div class="min-w-0 border-t border-line pt-1.5">
              <dt class="label">{s.k}</dt>
              <dd class="truncate text-lg leading-tight" class:text-danger={s.tone === 'danger'} class:text-accent={s.tone === 'up'} class:text-warn={s.tone === 'down'}>
                {s.v}{#if s.sub}<span class="ml-1.5 text-xs text-ink3">{s.sub}</span>{/if}
              </dd>
            </div>
          {/each}
        </dl>
        {#if ca(p)}
          <div class="mx-4 mb-3 border border-line2 bg-panel2 px-3 py-2">
            <div class="label">Closest approach</div>
            <div class="text-base text-select">{ca(p)}</div>
          </div>
        {/if}
      </div>
    {/key}
  {/if}
</section>

<style>
  section {
    /* livery colours are lifted toward the ink colour so dark liveries stay legible on dark themes */
    --livery: color-mix(in oklab, var(--livery-raw) 62%, var(--ink));
  }
  :global(.text-livery) {
    color: var(--livery);
  }
</style>
