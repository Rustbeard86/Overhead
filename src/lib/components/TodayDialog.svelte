<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { X, Download } from '@lucide/svelte';
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import { app } from '../store.svelte';
  import { readPalette } from '../theme';
  import { fmtAlt, fmtDist } from '../units';
  import type { TodayData } from '../types';
  import { toast } from 'svelte-sonner';

  let data = $state<TodayData | null>(null);
  let plotEl = $state<HTMLDivElement>();
  let plot: uPlot | undefined;
  const u = $derived(app.settings.units);

  $effect(() => {
    if (app.todayOpen) app.today().then((d) => (data = d)).catch(() => (data = null));
    else {
      plot?.destroy();
      plot = undefined;
    }
  });

  $effect(() => {
    if (!data || !plotEl) return;
    const d = data;
    const pal = readPalette();
    plot?.destroy();
    const w = plotEl.clientWidth || 600;
    const bars = uPlot.paths.bars!({ size: [0.7, 60], align: 1 });
    plot = new uPlot(
      {
        width: w,
        height: 150,
        cursor: { show: false },
        legend: { show: false },
        select: { show: false, left: 0, top: 0, width: 0, height: 0 },
        scales: { x: { time: false, range: [-0.5, 23.5] }, y: { range: (_u, _min, max) => [0, Math.max(4, max)] } },
        axes: [
          { stroke: pal.ink2, grid: { show: false }, ticks: { stroke: pal.line }, font: '10px "B612 Mono"', values: (_u, v) => v.map((x) => (x % 3 === 0 ? String(x).padStart(2, '0') : '')), splits: () => Array.from({ length: 24 }, (_, i) => i) },
          { stroke: pal.ink2, grid: { stroke: pal.line, width: 1 }, ticks: { show: false }, font: '10px "B612 Mono"', size: 28 },
        ],
        series: [{}, { stroke: pal.accent, fill: pal.accent + '99', paths: bars, width: 0 }],
      },
      [Array.from({ length: 24 }, (_, i) => i), d.hourly],
      plotEl,
    );
    return () => plot?.destroy();
  });

  const time = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });

  async function exportCsv() {
    try {
      const p = await app.exportCsv();
      if (p) toast.success('Exported', { description: p });
    } catch (e) {
      toast.error('Export failed', { description: String(e) });
    }
  }
</script>

<Dialog.Root bind:open={app.todayOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="dialog" style="height: min(40rem, calc(100vh - 1.5rem))">
      <div class="flex items-center justify-between border-b border-line px-5 py-3">
        <Dialog.Title class="font-display text-xl font-bold uppercase tracking-[0.12em]">Today</Dialog.Title>
        <div class="flex items-center gap-2">
          <button class="btn" onclick={exportCsv}><Download size={14} /> Export CSV</button>
          <Dialog.Close class="btn btn-icon btn-ghost" aria-label="Close"><X size={16} /></Dialog.Close>
        </div>
      </div>
      <Dialog.Description class="sr-only">Sightings logged today with first seen time, closest approach and minimum altitude.</Dialog.Description>
      <div class="min-h-0 flex-1 overflow-y-auto">
        {#if !data}
          <div class="grid h-40 place-items-center text-ink3">Loading…</div>
        {:else}
          <div class="grid grid-cols-3 border-b border-line">
            {#each [['Aircraft', String(data.stats.count)], ['Closest', data.stats.closest_km != null ? fmtDist(data.stats.closest_km, u) : '—'], ['Lowest', data.stats.lowest_ft != null ? fmtAlt(data.stats.lowest_ft, u) : '—']] as [k, v] (k)}
              <div class="border-r border-line px-5 py-3 last:border-r-0">
                <div class="label">{k}</div>
                <div class="font-display text-2xl font-bold">{v}</div>
              </div>
            {/each}
          </div>
          <div class="px-5 pt-3">
            <div class="label mb-1">Sightings by hour</div>
            <div bind:this={plotEl} class="w-full"></div>
          </div>
          <div class="px-5 pb-4 pt-2">
            {#if data.sightings.length === 0}
              <p class="py-6 text-center text-sm text-ink3">Nothing logged yet today. Aircraft inside the view radius are recorded automatically.</p>
            {:else}
              <table class="w-full text-sm">
                <thead>
                  <tr class="text-left">
                    {#each ['Callsign', 'Type', 'First seen', 'Closest', 'Lowest'] as h (h)}<th class="label border-b border-line py-1.5 font-normal">{h}</th>{/each}
                  </tr>
                </thead>
                <tbody>
                  {#each data.sightings as r (r.id ?? r.hex + r.first_seen_ms)}
                    <tr class="border-b border-line">
                      <td class="py-1.5 font-display font-bold">{r.callsign ?? r.hex.toUpperCase()}</td>
                      <td class="text-ink2">{r.type_code ?? '—'}</td>
                      <td class="num text-ink2">{time(r.first_seen_ms)}</td>
                      <td class="num">{fmtDist(r.min_dist_km, u)}</td>
                      <td class="num">{r.min_alt_ft != null ? fmtAlt(r.min_alt_ft, u) : '—'}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
