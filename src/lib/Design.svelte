<script lang="ts">
  import { toast } from 'svelte-sonner';
  import { app } from './store.svelte';
  import GlyphIcon from './components/GlyphIcon.svelte';
  import OverheadCard from './components/OverheadCard.svelte';
  import { CATEGORY_LABEL } from './glyphs';
  import type { Category, ThemeName } from './types';

  const themes: ThemeName[] = ['phosphor', 'amber', 'daylight'];
  const cats = Object.keys(CATEGORY_LABEL) as Category[];
  const swatches = ['bg', 'panel', 'panel-2', 'raised', 'line', 'line-strong', 'ink', 'ink-2', 'ink-3', 'accent', 'accent-dim', 'select', 'warn', 'danger', 'mil', 'heavy', 'heli'];
  const scale = [
    ['Display', 'var(--text-display)', 'font-display font-bold'],
    ['2XL', 'var(--text-2xl)', 'font-display font-bold'],
    ['XL', 'var(--text-xl)', 'font-display font-semibold'],
    ['LG', 'var(--text-lg)', 'font-data'],
    ['Base', 'var(--text-base)', 'font-data'],
    ['SM', 'var(--text-sm)', 'font-data'],
    ['XS', 'var(--text-xs)', 'font-data'],
  ] as const;
  const strips = [
    { cs: 'BAW117', ty: 'A320', alt: '098', spd: '389', d: '1.2', cat: 'narrowbody' as Category, state: 'featured' },
    { cs: 'UAE7', ty: 'A388', alt: '042', spd: '417', d: '8.4', cat: 'widebody' as Category, state: '' },
    { cs: 'RRR7216', ty: 'A400', alt: '120', spd: '330', d: '18', cat: 'military' as Category, state: 'pinned' },
    { cs: 'RYR42XE', ty: 'B738', alt: '110', spd: '340', d: '22', cat: 'narrowbody' as Category, state: 'emergency' },
    { cs: 'GOSEY', ty: 'C172', alt: '023', spd: '58', d: '9.1', cat: 'light' as Category, state: '' },
  ];
</script>

<div class="h-full overflow-y-auto bg-bg" style="user-select:text">
  <div class="mx-auto flex max-w-6xl flex-col gap-8 p-6">
    <header class="flex flex-wrap items-end justify-between gap-4 border-b border-line pb-4">
      <div>
        <div class="label">Overhead · living style guide</div>
        <h1 class="font-display text-4xl font-bold tracking-wide">Instrument, not dashboard</h1>
        <p class="mt-1 max-w-xl text-ink2">One token set, three themes. Everything below reads the same CSS variables the app uses.</p>
      </div>
      <div class="flex items-center gap-2">
        <div class="seg" role="radiogroup" aria-label="Theme">
          {#each themes as t (t)}
            <button role="radio" aria-checked={app.settings.theme === t} data-active={app.settings.theme === t} onclick={() => app.update((s) => (s.theme = t))}>{t}</button>
          {/each}
        </div>
        <a class="btn" href="#/">App</a>
      </div>
    </header>

    <section>
      <h2 class="label mb-3">Colour</h2>
      <div class="grid grid-cols-3 gap-px border border-line bg-line sm:grid-cols-6 lg:grid-cols-9">
        {#each swatches as s (s)}
          <div class="bg-panel p-2">
            <div class="h-12 border border-line2" style:background={`var(--${s})`}></div>
            <div class="mt-1.5 text-[11px] text-ink2">--{s}</div>
          </div>
        {/each}
      </div>
    </section>

    <section class="grid gap-8 lg:grid-cols-2">
      <div>
        <h2 class="label mb-3">Type</h2>
        <div class="flex flex-col gap-2 border border-line bg-panel p-4">
          {#each scale as [name, size, cls] (name)}
            <div class="flex items-baseline gap-4">
              <span class="w-16 shrink-0 text-[11px] text-ink3">{name}</span>
              <span class={cls} style:font-size={size} style:line-height="1.1">BAW117 · 35,000 ft · 450 kt</span>
            </div>
          {/each}
          <div class="mt-2 text-xs text-ink3">Chakra Petch for headings, callsigns and labels. B612 Mono (drawn for cockpit displays) for data, with tabular figures.</div>
        </div>
      </div>
      <div>
        <h2 class="label mb-3">Aircraft glyphs</h2>
        <div class="grid grid-cols-3 gap-px border border-line bg-line">
          {#each cats as c, i (c)}
            <div class="flex flex-col items-center gap-2 bg-panel p-4">
              <GlyphIcon category={c} size={56} rotate={i * 20} class="text-accent" />
              <span class="label">{CATEGORY_LABEL[c]}</span>
            </div>
          {/each}
        </div>
      </div>
    </section>

    <section class="grid gap-8 lg:grid-cols-2">
      <div class="flex flex-col gap-6">
        <div>
          <h2 class="label mb-3">Buttons &amp; fields</h2>
          <div class="flex flex-wrap items-center gap-3 border border-line bg-panel p-4">
            <button class="btn btn-primary">Primary</button>
            <button class="btn">Secondary</button>
            <button class="btn" aria-pressed="true">Active</button>
            <button class="btn btn-ghost">Ghost</button>
            <button class="btn" disabled style="opacity:.45">Disabled</button>
            <input class="field !w-40" value="51.5074" aria-label="Sample field" />
          </div>
        </div>
        <div>
          <h2 class="label mb-3">Badges</h2>
          <div class="flex flex-wrap gap-2 border border-line bg-panel p-4">
            <span class="badge badge-mil">Military</span><span class="badge badge-emergency">Emergency</span><span class="badge badge-heavy">Heavy</span>
            <span class="badge badge-helicopter">Helicopter</span><span class="badge badge-rare">Rare type</span><span class="badge badge-watch">Watch list</span>
          </div>
        </div>
        <div>
          <h2 class="label mb-3">Toasts</h2>
          <div class="flex flex-wrap gap-2 border border-line bg-panel p-4">
            <button class="btn" onclick={() => toast.info('BAW117: flyover', { description: '1.2 km away at 3,200 ft' })}>Flyover</button>
            <button class="btn" onclick={() => toast.warning('RRR7216: military aircraft', { description: 'A400' })}>Military</button>
            <button class="btn" onclick={() => toast.error('RYR42XE: emergency', { description: 'Squawk 7700 (general emergency)' })}>Emergency</button>
          </div>
        </div>
        <div>
          <h2 class="label mb-3">Flight strips</h2>
          <div class="border border-line">
            {#each strips as s (s.cs)}
              <div class="strip" data-featured={s.state === 'featured'} data-pinned={s.state === 'pinned'} data-emergency={s.state === 'emergency'}>
                <span class="flex items-center gap-1.5"><GlyphIcon category={s.cat} size={13} class={s.state === 'emergency' ? 'text-danger' : s.cat === 'military' ? 'text-mil' : 'text-accent'} /><span class="cs">{s.cs}</span></span>
                <span class="dim">{s.ty}</span><span>{s.alt}</span><span class="dim text-right">{s.spd}</span><span class="text-right">{s.d}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
      <div>
        <h2 class="label mb-3">Featured card</h2>
        <OverheadCard />
      </div>
    </section>
  </div>
</div>
