<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { app } from '../store.svelte';
  import HomePicker from './HomePicker.svelte';

  let lat = $state<number | null>(null);
  let lon = $state<number | null>(null);
  const valid = $derived(lat != null && lon != null && Math.abs(lat) <= 90 && Math.abs(lon) <= 180 && !Number.isNaN(lat) && !Number.isNaN(lon));
  const num = (v: string) => (v.trim() === '' ? null : Number(v));
  async function save() {
    if (!valid) return;
    await app.setHome(lat!, lon!);
    app.onboardingOpen = false;
  }
</script>

<Dialog.Root bind:open={app.onboardingOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="dialog" interactOutsideBehavior="ignore">
      <div class="border-b border-line px-5 py-4">
        <div class="label">First run</div>
        <Dialog.Title class="font-display text-2xl font-bold tracking-wide">Where is home?</Dialog.Title>
        <Dialog.Description class="mt-1 text-sm text-ink2">
          Overhead shows aircraft around this point. It stays on your computer. Until you set it, the app runs on synthetic demo traffic.
        </Dialog.Description>
      </div>
      <div class="flex flex-col gap-3 overflow-y-auto p-5">
        <HomePicker bind:lat bind:lon />
        <div class="grid grid-cols-2 gap-3">
          <label class="flex flex-col gap-1"><span class="label">Latitude</span>
            <input class="field" inputmode="decimal" placeholder="51.5074" value={lat ?? ''} oninput={(e) => (lat = num(e.currentTarget.value))} /></label>
          <label class="flex flex-col gap-1"><span class="label">Longitude</span>
            <input class="field" inputmode="decimal" placeholder="-0.1278" value={lon ?? ''} oninput={(e) => (lon = num(e.currentTarget.value))} /></label>
        </div>
      </div>
      <div class="flex items-center justify-between gap-3 border-t border-line px-5 py-3">
        <button class="btn btn-ghost" onclick={() => app.finishOnboarding()}>Use demo for now</button>
        <div class="flex items-center gap-3">
          {#if !valid}<span class="text-xs text-ink2">Click the map or enter coordinates</span>{/if}
          <button class="btn btn-primary" disabled={!valid} onclick={save}>Set home</button>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
