<script lang="ts">
  import { fade } from 'svelte/transition';
  import { WifiOff, Loader } from '@lucide/svelte';
  import { app } from '../store.svelte';
  const s = $derived(app.snapshot);
</script>

{#if s?.link === 'unavailable'}
  <div class="pointer-events-none absolute inset-0 grid place-items-center" transition:fade={{ duration: 300 }} role="status">
    <div class="flex items-center gap-3 border border-danger bg-panel/90 px-5 py-3 text-danger">
      <WifiOff size={20} />
      <div>
        <div class="font-display text-lg font-bold uppercase tracking-[0.18em]">Data unavailable</div>
        <div class="text-xs text-ink2">All providers failing. Retrying automatically.</div>
      </div>
    </div>
  </div>
{:else if s?.link === 'connecting'}
  <div class="pointer-events-none absolute inset-0 grid place-items-center" transition:fade={{ duration: 300 }} role="status">
    <div class="flex items-center gap-3 border border-line2 bg-panel/90 px-5 py-3 text-ink2">
      <Loader size={18} class="animate-spin" />
      <span class="font-display text-sm font-semibold uppercase tracking-[0.18em]">Acquiring traffic</span>
    </div>
  </div>
{:else if s && s.link === 'degraded'}
  <div class="pointer-events-none absolute left-1/2 top-3 -translate-x-1/2" transition:fade={{ duration: 200 }} role="status">
    <div class="border border-warn bg-panel/90 px-3 py-1 text-xs text-warn">Connection lost · extrapolating positions</div>
  </div>
{/if}
