// Resolve the active theme's CSS variables into concrete colours for canvas code.

import type { ThemeName } from './types';

export interface Palette {
  bg: string;
  radarBg: string;
  radarCore: string;
  ring: string;
  ringStrong: string;
  sweep: string; // "r, g, b"
  plane: string;
  planeGhost: string;
  accent: string;
  select: string;
  ink: string;
  ink2: string;
  ink3: string;
  warn: string;
  danger: string;
  mil: string;
  heavy: string;
  heli: string;
  line: string;
  dark: boolean;
}

export function readPalette(): Palette {
  const s = getComputedStyle(document.documentElement);
  const g = (n: string) => s.getPropertyValue(n).trim();
  return {
    bg: g('--bg'),
    radarBg: g('--radar-bg'),
    radarCore: g('--radar-core'),
    ring: g('--ring'),
    ringStrong: g('--ring-strong'),
    sweep: g('--sweep'),
    plane: g('--plane'),
    planeGhost: g('--plane-ghost'),
    accent: g('--accent'),
    select: g('--select'),
    ink: g('--ink'),
    ink2: g('--ink-2'),
    ink3: g('--ink-3'),
    warn: g('--warn'),
    danger: g('--danger'),
    mil: g('--mil'),
    heavy: g('--heavy'),
    heli: g('--heli'),
    line: g('--line'),
    dark: document.documentElement.dataset.theme !== 'daylight',
  };
}

export function applyTheme(t: ThemeName) {
  document.documentElement.dataset.theme = t;
}

export function mapColors() {
  const s = getComputedStyle(document.documentElement);
  const g = (n: string) => s.getPropertyValue(n).trim();
  return {
    land: g('--map-land'),
    land2: g('--map-land-2'),
    water: g('--map-water'),
    road: g('--map-road'),
    roadMajor: g('--map-road-major'),
    label: g('--map-label'),
    runway: g('--map-runway'),
    line: g('--line-strong'),
    bg: g('--bg'),
  };
}
