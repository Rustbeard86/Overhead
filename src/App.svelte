<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { Toaster } from 'svelte-sonner';
  import { Tooltip } from 'bits-ui';
  import { app } from './lib/store.svelte';
  import TopBar from './lib/components/TopBar.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import RadarCanvas from './lib/components/RadarCanvas.svelte';
  import SkyCanvas from './lib/components/SkyCanvas.svelte';
  import MapView from './lib/components/MapView.svelte';
  import OverheadCard from './lib/components/OverheadCard.svelte';
  import StripList from './lib/components/StripList.svelte';
  import FeaturedChip from './lib/components/FeaturedChip.svelte';
  import StageOverlays from './lib/components/StageOverlays.svelte';
  import SettingsDialog from './lib/components/SettingsDialog.svelte';
  import TodayDialog from './lib/components/TodayDialog.svelte';
  import Onboarding from './lib/components/Onboarding.svelte';
  import Design from './lib/Design.svelte';

  let route = $state(location.hash);
  const kiosk = $derived(app.settings.window.kiosk);
  let idleTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => {
    if (route !== '#/design') app.init();
    else app.init();
  });

  function poke() {
    app.chromeHidden = false;
    clearTimeout(idleTimer);
    if (kiosk) idleTimer = setTimeout(() => (app.chromeHidden = true), 4000);
  }
  $effect(() => {
    kiosk;
    poke();
    if (!kiosk) app.chromeHidden = false;
  });

  function key(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    if (t && /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (app.settingsOpen || app.todayOpen || app.onboardingOpen) return;
    poke();
    switch (e.key) {
      case '1': app.setView('radar'); break;
      case '2': app.setView('map'); break;
      case '3': app.setView('sky'); break;
      case 'f': case 'F': app.update((s) => (s.window.kiosk = !s.window.kiosk)); break;
      case 'j': case 'ArrowDown': if (!(t?.closest?.('[role=listbox]'))) { e.preventDefault(); app.cycle(1); } break;
      case 'k': case 'ArrowUp': if (!(t?.closest?.('[role=listbox]'))) { e.preventDefault(); app.cycle(-1); } break;
      case 'Escape':
        if (kiosk) app.update((s) => (s.window.kiosk = false));
        else if (app.pinnedHex) app.pin(app.pinnedHex);
        else app.panelOpen = false;
        break;
    }
  }
</script>

<svelte:window onkeydown={key} onpointermove={poke} onhashchange={() => (route = location.hash)} />

<Tooltip.Provider>
  {#if route === '#/design'}
    <Design />
  {:else}
    <div class="flex h-full flex-col" class:kiosk data-hide-chrome={kiosk && app.chromeHidden} style:cursor={kiosk && app.chromeHidden ? 'none' : ''}>
      <div class="chrome"><TopBar /></div>
      <div class="main relative min-h-0 flex-1">
        <section class="relative min-h-0 min-w-0 overflow-hidden" aria-label={`${app.view} view`}>
          {#key app.view}
            <div class="absolute inset-0" in:scale={{ start: 0.985, duration: 240, opacity: 0 }} out:fade={{ duration: 0 }}>
              {#if app.view === 'radar'}<RadarCanvas />{:else if app.view === 'map'}<MapView />{:else}<SkyCanvas />{/if}
            </div>
          {/key}
          <StageOverlays />
          <div class="chip absolute bottom-3 left-3"><FeaturedChip /></div>
        </section>
        <aside class="side flex min-h-0 flex-col gap-2 overflow-y-auto border-l border-line bg-bg p-2" data-open={app.panelOpen} aria-label="Aircraft panel">
          <OverheadCard />
          <StripList />
        </aside>
        {#if app.panelOpen}
          <button class="scrim" aria-label="Close panel" onclick={() => (app.panelOpen = false)} transition:fade={{ duration: 150 }}></button>
        {/if}
      </div>
      <div class="chrome"><StatusBar /></div>
    </div>
  {/if}
  <Toaster position="top-right" visibleToasts={4} closeButton={false} />
  <SettingsDialog />
  <TodayDialog />
  <Onboarding />
</Tooltip.Provider>

<style>
  .main {
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(19rem, 26vw, 25rem);
  }
  .side {
    scroll-behavior: smooth;
  }
  .scrim {
    display: none;
  }
  .chrome {
    transition: opacity 320ms var(--ease), transform 320ms var(--ease);
  }
  [data-hide-chrome='true'] .chrome {
    opacity: 0;
    pointer-events: none;
  }
  .chip {
    display: none;
  }
  @media (max-width: 899px) {
    .main {
      grid-template-columns: minmax(0, 1fr);
    }
    .side {
      position: absolute;
      z-index: 25;
      top: 0;
      right: 0;
      bottom: 0;
      width: min(94vw, 23rem);
      transform: translateX(102%);
      transition: transform 280ms var(--ease);
      box-shadow: -12px 0 30px rgba(0, 0, 0, 0.35);
    }
    .side[data-open='true'] {
      transform: none;
    }
    .scrim {
      display: block;
      position: absolute;
      inset: 0;
      z-index: 24;
      background: var(--scrim);
      border: 0;
    }
    .chip {
      display: block;
      z-index: 10;
    }
  }
  @media (max-height: 420px) {
    .chip {
      display: none;
    }
  }
</style>
