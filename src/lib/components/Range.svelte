<script lang="ts">
  import { Slider } from 'bits-ui';
  let { value, min, max, step = 1, onchange, label }: { value: number; min: number; max: number; step?: number; onchange: (v: number) => void; label: string } = $props();
</script>

<Slider.Root type="single" {value} {min} {max} {step} onValueChange={onchange} class="slider-root" aria-label={label}>
  {#snippet children({ thumbItems })}
    <span class="slider-track"><Slider.Range class="slider-range" /></span>
    {#each thumbItems as t (t.index)}
      <Slider.Thumb index={t.index} class="slider-thumb" />
    {/each}
  {/snippet}
</Slider.Root>
