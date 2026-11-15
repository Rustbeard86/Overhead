<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import type { Snippet } from 'svelte';
  let {
    label, onclick, pressed, children, hotkey, class: cls = '', disabled = false,
  }: { label: string; onclick?: () => void; pressed?: boolean; children: Snippet; hotkey?: string; class?: string; disabled?: boolean } = $props();
</script>

<Tooltip.Root delayDuration={350}>
  <Tooltip.Trigger>
    {#snippet child({ props })}
      <button {...props} class="btn btn-icon btn-ghost {cls}" aria-label={label} aria-pressed={pressed} {onclick} {disabled}>
        {@render children()}
      </button>
    {/snippet}
  </Tooltip.Trigger>
  <Tooltip.Portal>
    <Tooltip.Content class="tooltip" sideOffset={6}>
      {label}{#if hotkey}<span class="ml-2 text-ink3">{hotkey}</span>{/if}
    </Tooltip.Content>
  </Tooltip.Portal>
</Tooltip.Root>
