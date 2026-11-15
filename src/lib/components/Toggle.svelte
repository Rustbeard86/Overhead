<script lang="ts">
  import { Switch } from 'bits-ui';
  let { checked, onchange, label }: { checked: boolean; onchange: (v: boolean) => void; label: string } = $props();
</script>

<Switch.Root class="switch" {checked} onCheckedChange={onchange} aria-label={label}>
  <Switch.Thumb class="thumb" />
</Switch.Root>
