<script lang="ts">
  import type { Snippet } from 'svelte';
  let { label, hint, children }: { label: string; hint?: string; children: Snippet } = $props();
</script>

<div class="grid items-center gap-x-4 gap-y-1 border-b border-line py-2.5 sm:grid-cols-[1fr_minmax(0,16rem)]">
  <div>
    <div class="font-display text-sm font-semibold uppercase tracking-[0.08em]">{label}</div>
    {#if hint}<div class="text-xs text-ink3">{hint}</div>{/if}
  </div>
  <div class="flex items-center justify-end gap-3">{@render children()}</div>
</div>
