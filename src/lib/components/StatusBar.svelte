<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import { app } from '../store.svelte';
  import { fmtAlt, fmtDist } from '../units';

  const s = $derived(app.snapshot);
  const u = $derived(app.settings.units);
  let clock = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (clock = new Date()), 1000);
    return () => clearInterval(t);
  });
  const label = $derived(
    !s ? 'Starting' : s.source === 'demo' ? 'Demo' : s.source === 'replay' ? 'Replay' : s.link === 'live' ? 'Live' : s.link === 'degraded' ? 'Degraded' : s.link === 'connecting' ? 'Connecting' : 'No data',
  );
  const tone = $derived(!s ? 'text-ink3' : s.link === 'live' ? 'text-accent' : s.link === 'degraded' || s.link === 'connecting' ? 'text-warn' : 'text-danger');
  const age = $derived(s?.data_age_s != null ? (s.data_age_s < 10 ? s.data_age_s.toFixed(1) : Math.round(s.data_age_s).toString()) + ' s' : '—');
  const counts = $derived({ all: s?.aircraft.length ?? 0, view: s?.aircraft.filter((a) => a.in_view).length ?? 0 });
</script>

<footer class="flex h-7 shrink-0 items-center gap-4 border-t border-line bg-panel px-3 text-xs text-ink2" aria-label="Status">
  <Tooltip.Root delayDuration={200}>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <button {...props} class="flex items-center gap-2 {tone}" aria-label="Data source status">
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
            {#if s?.link === 'live' || s?.source !== 'live'}
              <circle cx="5" cy="5" r="4" fill="currentColor" />
            {:else if s?.link === 'degraded' || s?.link === 'connecting'}
              <path d="M5 1 L9 9 L1 9 Z" fill="currentColor" />
            {:else}
              <rect x="1.5" y="1.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.6" />
            {/if}
          </svg>
          <span class="font-display text-[11px] font-bold uppercase tracking-[0.14em]">{label}</span>
          {#if s?.provider}<span class="text-ink2">{s.provider}</span>{/if}
          <span class="text-ink3">age {age}</span>
        </button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Portal>
      <Tooltip.Content class="tooltip !normal-case !tracking-normal" side="top" sideOffset={8}>
        {#if s && s.providers.length}
          <div class="flex flex-col gap-1 text-[11px]">
            {#each s.providers as p (p.id)}
              <div class="flex items-center justify-between gap-6">
                <span class={p.active ? 'text-accent' : ''}>{p.active ? '▸ ' : '  '}{p.name}</span>
                <span class={p.state === 'ok' ? 'text-accent' : p.state === 'backoff' ? 'text-warn' : 'text-ink3'}>
                  {p.state === 'backoff' ? `${p.last_error ?? 'error'} · retry ${Math.ceil(p.retry_in_s)}s` : p.state}
                </span>
              </div>
            {/each}
          </div>
        {:else}
          <span class="text-[11px]">{s?.source === 'live' ? 'Waiting for the first response' : 'Synthetic data, no network'}</span>
        {/if}
      </Tooltip.Content>
    </Tooltip.Portal>
  </Tooltip.Root>

  <span class="hidden sm:inline" aria-label="Aircraft counts">{counts.view} in range <span class="text-ink3">/ {counts.all} tracked</span></span>
  <span class="flex-1"></span>
  {#if s}
    <span class="hidden md:inline">today <b class="font-bold text-ink">{s.today.count}</b></span>
    {#if s.today.closest_km != null}<span class="hidden lg:inline">closest <b class="font-bold text-ink">{fmtDist(s.today.closest_km, u)}</b></span>{/if}
    {#if s.today.lowest_ft != null}<span class="hidden lg:inline">lowest <b class="font-bold text-ink">{fmtAlt(s.today.lowest_ft, u)}</b></span>{/if}
  {/if}
  <time class="num text-ink3">{clock.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })}</time>
</footer>
