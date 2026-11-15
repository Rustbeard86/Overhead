// Aircraft silhouettes, drawn by hand for this project (top-down, nose up,
// 100x100 box centred on 0,0). Original artwork released under the project
// licence (MIT); no third-party silhouette sets are used.

import type { Category } from './types';

export const GLYPH_PATHS: Record<Category, string> = {
  narrowbody:
    'M0,-48 C1.8,-48 2.6,-42 2.6,-36 L2.6,-9 L36,6 C37.2,6.6 37.2,9.4 36,9.4 L2.6,1.4 L2.4,26 L13,35 C13.8,35.7 13.8,38 13,38.2 L0.8,35 L0,36 L-0.8,35 L-13,38.2 C-13.8,38 -13.8,35.7 -13,35 L-2.4,26 L-2.6,1.4 L-36,9.4 C-37.2,9.4 -37.2,6.6 -36,6 L-2.6,-9 L-2.6,-36 C-2.6,-42 -1.8,-48 0,-48 Z M-14,1 h4 v9 h-4 Z M10,1 h4 v9 h-4 Z',
  widebody:
    'M0,-50 C2.4,-50 3.6,-43 3.6,-36 L3.6,-10 L44,9 C45.4,9.7 45.4,13 44,13.2 L3.6,3.2 L3.4,28 L15,38 C15.8,38.8 15.8,41.4 15,41.6 L1,37.6 L0,39 L-1,37.6 L-15,41.6 C-15.8,41.4 -15.8,38.8 -15,38 L-3.4,28 L-3.6,3.2 L-44,13.2 C-45.4,13 -45.4,9.7 -44,9 L-3.6,-10 L-3.6,-36 C-3.6,-43 -2.4,-50 0,-50 Z M-27,3 h4.4 v11 h-4.4 Z M-15,-2 h4.4 v11 h-4.4 Z M10.6,-2 h4.4 v11 h-4.4 Z M22.6,3 h4.4 v11 h-4.4 Z',
  light:
    'M0,-40 C1.6,-40 2.2,-36 2.2,-30 L2.2,-6 L40,-4 L40,3.5 L2.2,4.5 L2,26 L14,28 L14,33 L0.6,32 L0,34 L-0.6,32 L-14,33 L-14,28 L-2,26 L-2.2,4.5 L-40,3.5 L-40,-4 L-2.2,-6 L-2.2,-30 C-2.2,-36 -1.6,-40 0,-40 Z M-6,-43 h12 v1.8 h-12 Z',
  helicopter:
    'M0,-18 C7,-18 9,-8 9,0 C9,9 5,14 2,16 L1.6,40 L5,44 L5,47 L-5,47 L-5,44 L-1.6,40 L-2,16 C-5,14 -9,9 -9,0 C-9,-8 -7,-18 0,-18 Z',
  military:
    'M0,-46 L3.4,-26 L8,-12 L38,14 L38,22 L9,17 L7,26 L16,36 L16,41 L0,38 L-16,41 L-16,36 L-7,26 L-9,17 L-38,22 L-38,14 L-8,-12 L-3.4,-26 Z',
  unknown: 'M0,-24 L14,18 L0,10 L-14,18 Z',
};

/** Relative on-screen size (px at scale 1) per category, so a widebody reads bigger than a Cessna. */
export const GLYPH_SIZE: Record<Category, number> = {
  narrowbody: 13.5,
  widebody: 17.5,
  light: 10.5,
  helicopter: 12,
  military: 13.5,
  unknown: 10,
};

export const CATEGORY_LABEL: Record<Category, string> = {
  narrowbody: 'Narrowbody',
  widebody: 'Widebody',
  light: 'Light / GA',
  helicopter: 'Helicopter',
  military: 'Military',
  unknown: 'Unknown',
};

const cache = new Map<Category, Path2D>();
export function glyphPath(c: Category): Path2D {
  let p = cache.get(c);
  if (!p) {
    p = new Path2D(GLYPH_PATHS[c]);
    cache.set(c, p);
  }
  return p;
}
