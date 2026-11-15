// Dark vector map style built from the active theme. Tiles: OpenFreeMap
// (https://openfreemap.org, free, no key; OpenMapTiles schema, data (c)
// OpenStreetMap contributors). We only supply our own colours, so the map
// restyles live with the theme.

import type { StyleSpecification, LayerSpecification } from 'maplibre-gl';
import { mapColors } from './theme';
import { setWorkerUrl } from 'maplibre-gl';
import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?url';

// Must run before any Map is created (first-run dialog included).
setWorkerUrl(workerUrl);

export function buildStyle(): StyleSpecification {
  const c = mapColors();
  const layers: LayerSpecification[] = [
    { id: 'bg', type: 'background', paint: { 'background-color': c.land } },
    { id: 'landcover', type: 'fill', source: 'ofm', 'source-layer': 'landcover', paint: { 'fill-color': c.land2, 'fill-opacity': 0.7 } },
    { id: 'landuse', type: 'fill', source: 'ofm', 'source-layer': 'landuse', filter: ['in', 'class', 'residential', 'industrial', 'commercial', 'retail', 'cemetery', 'pitch', 'park'], paint: { 'fill-color': c.land2, 'fill-opacity': 0.55 } },
    { id: 'park', type: 'fill', source: 'ofm', 'source-layer': 'park', paint: { 'fill-color': c.land2, 'fill-opacity': 0.6 } },
    { id: 'water', type: 'fill', source: 'ofm', 'source-layer': 'water', paint: { 'fill-color': c.water } },
    { id: 'waterway', type: 'line', source: 'ofm', 'source-layer': 'waterway', paint: { 'line-color': c.water, 'line-width': 1.2 } },
    { id: 'building', type: 'fill', source: 'ofm', 'source-layer': 'building', minzoom: 13, paint: { 'fill-color': c.land2, 'fill-opacity': 0.8 } },
    { id: 'aeroway-area', type: 'fill', source: 'ofm', 'source-layer': 'aeroway', filter: ['==', '$type', 'Polygon'], paint: { 'fill-color': c.line, 'fill-opacity': 0.35 } },
    {
      id: 'aeroway-runway', type: 'line', source: 'ofm', 'source-layer': 'aeroway', filter: ['all', ['==', '$type', 'LineString'], ['==', 'class', 'runway']],
      paint: { 'line-color': c.runway, 'line-opacity': 0.8, 'line-width': ['interpolate', ['exponential', 1.5], ['zoom'], 9, 1, 14, 8, 17, 28] },
    },
    {
      id: 'aeroway-taxiway', type: 'line', source: 'ofm', 'source-layer': 'aeroway', minzoom: 12, filter: ['all', ['==', '$type', 'LineString'], ['==', 'class', 'taxiway']],
      paint: { 'line-color': c.roadMajor, 'line-width': ['interpolate', ['linear'], ['zoom'], 12, 0.6, 16, 3] },
    },
    {
      id: 'road-minor', type: 'line', source: 'ofm', 'source-layer': 'transportation', minzoom: 11, filter: ['in', 'class', 'minor', 'service', 'track', 'path'],
      paint: { 'line-color': c.road, 'line-width': ['interpolate', ['linear'], ['zoom'], 11, 0.4, 16, 2.5] },
    },
    {
      id: 'road-major', type: 'line', source: 'ofm', 'source-layer': 'transportation', filter: ['in', 'class', 'primary', 'secondary', 'tertiary', 'trunk', 'motorway'],
      paint: { 'line-color': c.roadMajor, 'line-opacity': 0.85, 'line-width': ['interpolate', ['linear'], ['zoom'], 6, 0.5, 12, 1.4, 16, 5] },
    },
    { id: 'rail', type: 'line', source: 'ofm', 'source-layer': 'transportation', minzoom: 10, filter: ['==', 'class', 'rail'], paint: { 'line-color': c.road, 'line-dasharray': [3, 3], 'line-width': 0.8 } },
    { id: 'boundary', type: 'line', source: 'ofm', 'source-layer': 'boundary', filter: ['<=', 'admin_level', 4], paint: { 'line-color': c.line, 'line-dasharray': [4, 3], 'line-width': 0.8 } },
    {
      id: 'place', type: 'symbol', source: 'ofm', 'source-layer': 'place', filter: ['in', 'class', 'city', 'town', 'village', 'suburb'],
      layout: { 'text-field': ['coalesce', ['get', 'name:latin'], ['get', 'name']], 'text-font': ['Noto Sans Regular'], 'text-size': ['interpolate', ['linear'], ['zoom'], 6, 10, 12, 13], 'text-transform': 'uppercase', 'text-letter-spacing': 0.12, 'text-max-width': 8 },
      paint: { 'text-color': c.label, 'text-halo-color': c.bg, 'text-halo-width': 1.4 },
    },
    {
      id: 'water-name', type: 'symbol', source: 'ofm', 'source-layer': 'water_name',
      layout: { 'text-field': ['coalesce', ['get', 'name:latin'], ['get', 'name']], 'text-font': ['Noto Sans Italic'], 'text-size': 11 },
      paint: { 'text-color': c.label, 'text-opacity': 0.7, 'text-halo-color': c.bg, 'text-halo-width': 1 },
    },
  ];
  return {
    version: 8,
    glyphs: 'https://tiles.openfreemap.org/fonts/{fontstack}/{range}.pbf',
    sources: {
      ofm: {
        type: 'vector',
        url: 'https://tiles.openfreemap.org/planet',
        attribution: '<a href="https://openfreemap.org" target="_blank" rel="noopener">OpenFreeMap</a> © <a href="https://www.openmaptiles.org/" target="_blank" rel="noopener">OpenMapTiles</a> data © <a href="https://www.openstreetmap.org/copyright" target="_blank" rel="noopener">OpenStreetMap</a> contributors',
      },
    },
    layers,
  };
}
