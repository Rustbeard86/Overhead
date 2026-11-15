export interface LatLon {
  lat: number;
  lon: number;
}

const valid = (lat: number, lon: number) => Number.isFinite(lat) && Number.isFinite(lon) && Math.abs(lat) <= 90 && Math.abs(lon) <= 180;

/**
 * Pull a coordinate pair out of pasted text. Understands:
 *  - "29.810659, -82.527504" (Google Maps right-click), also separated by space or semicolon
 *  - Google Maps URLs: ".../@29.81,-82.53,17z" and "...!3d29.81!4d-82.53"
 *  - "29.81 N, 82.53 W" and degree signs
 * Returns null for anything else, including a single number.
 */
export function parseCoords(text: string): LatLon | null {
  const s = text.trim();
  if (!s) return null;
  const pin = s.match(/!3d(-?\d+(?:\.\d+)?)!4d(-?\d+(?:\.\d+)?)/);
  if (pin) return pick(+pin[1], +pin[2]);
  const at = s.match(/@(-?\d+(?:\.\d+)?),(-?\d+(?:\.\d+)?)/);
  if (at) return pick(+at[1], +at[2]);
  const hemi = s.match(/^(-?\d+(?:\.\d+)?)\s*°?\s*([NS])[\s,;]+(-?\d+(?:\.\d+)?)\s*°?\s*([EW])$/i);
  if (hemi) {
    const lat = +hemi[1] * (hemi[2].toUpperCase() === 'S' ? -1 : 1);
    const lon = +hemi[3] * (hemi[4].toUpperCase() === 'W' ? -1 : 1);
    return pick(lat, lon);
  }
  const plain = s.match(/^\(?\s*(-?\d+(?:\.\d+)?)\s*°?\s*[,;\s]\s*(-?\d+(?:\.\d+)?)\s*°?\s*\)?$/);
  if (plain) return pick(+plain[1], +plain[2]);
  return null;
}

function pick(lat: number, lon: number): LatLon | null {
  return valid(lat, lon) ? { lat, lon } : null;
}
