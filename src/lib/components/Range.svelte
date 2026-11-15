<script lang="ts">
  import { Slider } from 'bits-ui';
  let {
    value,
    min,
    max,
    step = 1,
    onchange,
    label,
    unit = '',
    fmt = (v: number) => String(v),
  }: {
    value: number;
    min: number;
    max: number;
    step?: number;
    onchange: (v: number) => void;
    label: string;
    /** Shown after the live value, and in the screen-reader text. */
    unit?: string;
    /** Formats numbers for the readout and the scale labels. */
    fmt?: (v: number) => string;
  } = $props();

  const readout = $derived(`${fmt(value)}${unit ? ' ' + unit : ''}`);
  const clamped = $derived(Math.min(max, Math.max(min, value)));
</script>

<div class="range">
  <div class="range-readout" aria-hidden="true">{readout}</div>
  <Slider.Root
    type="single"
    value={clamped}
    {min}
    {max}
    {step}
    onValueChange={onchange}
    class="slider-root"
    aria-label={label}
  >
    {#snippet children({ thumbItems, tickItems })}
      {@const n = Math.max(1, tickItems.length - 1)}
      {@const minorEvery = Math.max(1, Math.ceil(n / 30))}
      {@const labelAt = new Set([0, 1, 2, 3, 4].map((k) => Math.round((k * n) / 4)))}
      <span class="slider-track"><Slider.Range class="slider-range" /></span>
      {#each tickItems as t (t.index)}
        {#if labelAt.has(t.index)}
          <Slider.Tick index={t.index} class="slider-tick slider-tick-major" />
          <Slider.TickLabel index={t.index} position="bottom" class="slider-tick-label">{fmt(t.value)}</Slider.TickLabel>
        {:else if t.index % minorEvery === 0}
          <Slider.Tick index={t.index} class="slider-tick" />
        {/if}
      {/each}
      {#each thumbItems as th (th.index)}
        <Slider.Thumb index={th.index} class="slider-thumb" aria-valuetext={readout} />
      {/each}
    {/snippet}
  </Slider.Root>
</div>
