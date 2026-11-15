<script lang="ts">
  import { app } from '../store.svelte';
  import { altHundreds, fmtDist } from '../units';
  import GlyphIcon from './GlyphIcon.svelte';
  const p = $derived(app.cardPlane);
  const u = $derived(app.settings.units);
</script>

{#if p}
  <button class="panel flex items-center gap-3 px-3 py-2 text-left" onclick={() => (app.panelOpen = true)} aria-label="Open aircraft details">
    <GlyphIcon category={p.category} size={26} rotate={30} class="text-accent" />
    <span class="flex flex-col">
      <span class="font-display text-lg font-bold leading-none tracking-wide">{p.callsign ?? p.hex.toUpperCase()}</span>
      <span class="mt-1 text-xs text-ink2">
        {p.on_ground ? 'ground' : `FL${altHundreds(p.alt_ft, u)}`} · {fmtDist(p.dist_km, u)}{p.elev_deg != null ? ` · ${Math.round(p.elev_deg)}°` : ''}
      </span>
    </span>
  </button>
{/if}
