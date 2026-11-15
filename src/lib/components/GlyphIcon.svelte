<script lang="ts">
  import { GLYPH_PATHS } from '../glyphs';
  import type { Category } from '../types';
  let { category, size = 16, rotate = 0, class: cls = '' }: { category: Category; size?: number; rotate?: number; class?: string } = $props();
</script>

<svg class={cls} width={size} height={size} viewBox="-50 -50 100 100" aria-hidden="true" style:transform={`rotate(${rotate}deg)`}>
  <path d={GLYPH_PATHS[category]} fill="currentColor" />
</svg>
