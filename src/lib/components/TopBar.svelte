<script lang="ts">
  import { Tabs } from 'bits-ui';
  import { Radar, Map as MapIcon, Orbit, Settings, ListOrdered, Maximize, PanelRight, X, Minus } from '@lucide/svelte';
  import { app } from '../store.svelte';
  import IconButton from './IconButton.svelte';
  import type { ViewName } from '../types';

  const views: { id: ViewName; name: string; key: string }[] = [
    { id: 'radar', name: 'Radar', key: '1' },
    { id: 'map', name: 'Map', key: '2' },
    { id: 'sky', name: 'Sky', key: '3' },
  ];
  const frameless = $derived(app.settings.window.frameless);
</script>

<header
  class="relative z-30 flex h-11 shrink-0 items-center gap-2 border-b border-line bg-panel px-2.5 transition-[opacity,transform] duration-300"
  data-tauri-drag-region
>
  <div class="flex items-center gap-2 pr-1" data-tauri-drag-region>
    <svg width="22" height="22" viewBox="-50 -50 100 100" aria-hidden="true">
      <circle r="42" fill="none" stroke="var(--accent)" stroke-opacity=".55" stroke-width="5" />
      <circle r="20" fill="none" stroke="var(--accent)" stroke-opacity=".4" stroke-width="5" />
      <path d="M0 0 L36 -22" stroke="var(--accent)" stroke-width="6" stroke-linecap="round" />
      <circle r="6" fill="var(--accent)" />
    </svg>
    <span class="font-display text-[0.95rem] font-bold uppercase tracking-[0.3em] max-[560px]:hidden" data-tauri-drag-region>Overhead</span>
  </div>

  <Tabs.Root value={app.view} onValueChange={(v) => app.setView(v as ViewName)} class="ml-1">
    <Tabs.List class="flex items-center border border-line2" aria-label="View">
      {#each views as v (v.id)}
        <Tabs.Trigger value={v.id} class="viewtab" aria-keyshortcuts={v.key}>
          {#if v.id === 'radar'}<Radar size={14} />{:else if v.id === 'map'}<MapIcon size={14} />{:else}<Orbit size={14} />{/if}
          <span class="max-[620px]:hidden">{v.name}</span>
          <kbd class="max-[760px]:hidden">{v.key}</kbd>
        </Tabs.Trigger>
      {/each}
    </Tabs.List>
  </Tabs.Root>

  <span class="flex-1" data-tauri-drag-region></span>

  {#if app.snapshot?.home_is_demo}
    <span class="max-[640px]:hidden"><button class="btn !h-7 whitespace-nowrap !border-warn !text-warn" onclick={() => (app.settingsOpen = true)}>Demo · set home</button></span>
  {/if}
  <IconButton label="Today" onclick={() => (app.todayOpen = true)}><ListOrdered size={16} /></IconButton>
  <IconButton label="Settings" onclick={() => (app.settingsOpen = true)}><Settings size={16} /></IconButton>
  <IconButton label="Kiosk mode" hotkey="F" pressed={app.settings.window.kiosk} onclick={() => app.update((s) => (s.window.kiosk = !s.window.kiosk))}><Maximize size={16} /></IconButton>
  <IconButton label="Aircraft panel" class="panel-toggle" pressed={app.panelOpen} onclick={() => (app.panelOpen = !app.panelOpen)}><PanelRight size={16} /></IconButton>
  {#if frameless}
    <span class="mx-1 h-5 w-px bg-line2"></span>
    <IconButton label="Minimize" onclick={() => app.win()?.minimize?.()}><Minus size={16} /></IconButton>
    <IconButton label="Close" onclick={() => app.win()?.exitApp?.()}><X size={16} /></IconButton>
  {/if}
</header>

<style>
  :global(.viewtab) {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    height: 1.9rem;
    padding: 0 0.7rem;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--text-sm);
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink-2);
    background: transparent;
    border: 0;
    border-right: 1px solid var(--line-strong);
    cursor: pointer;
    transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
  }
  :global(.viewtab:last-child) {
    border-right: 0;
  }
  :global(.viewtab:hover) {
    color: var(--ink);
    background: var(--raised);
  }
  :global(.viewtab[data-state='active']) {
    background: var(--accent);
    color: var(--accent-ink);
  }
  :global(.viewtab kbd) {
    font-family: var(--font-data);
    font-size: 0.62rem;
    opacity: 0.6;
  }
  @media (min-width: 900px) {
    :global(.panel-toggle) {
      display: none;
    }
  }
</style>
