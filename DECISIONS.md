# Decisions and deviations

Notes on choices made while building, and on places where reality differed from the brief.

## Provider access
- **airplanes.live returned 403** from the build sandbox. The failover chain handles this exactly as it would in the wild (switch at once on 403, 429, 5xx, timeout or malformed JSON; back off 2s doubling to 60s per provider; retry the preferred provider at most every 60s). adsb.fi and adsb.lol answered, and live captures from them are in `tests/fixtures/live_*`. Nothing about airplanes.live's response format was guessed: it shares the ADSBx v2 schema with the others, and its parser path is covered by those captures.
- **Planespotters** also refused requests from the sandbox, so the photo API was never exercised against the real service. The code follows the documented endpoint and degrades to a drawn silhouette on any failure. The brief asked for 24h caching; positive results are cached 24h, failures 6h, both on disk.
- Global request spacing is 1.1s, above the 1 request/s the terms ask for. The poll interval cannot be set below 2s.

## Data model
- `alt_baro_ft` is `Option` with a separate `on_ground` flag, because feeds send the string `"ground"` instead of a number.
- The query radius is the view radius plus 10 nm, plus padding for the privacy offset, so planes at the edge don't pop in late.
- Ground traffic is scored at 8% so taxiing aircraft never win "overhead now".
- Tracks go stale after 60s (drawn faded) and are removed after 90s.
- The privacy offset (shifting the stated home before querying) is handled in the core; the radius padding above accounts for it.

## Architecture
- The Rust workspace is split into `overhead-core` and a thin `src-tauri`, so all logic is testable with plain `cargo test` and no webview.
- `EngineCore` is a synchronous, deterministic state machine; `Engine` wraps it in a tokio loop. Time is injected, which is what makes the e2e tests and replay reliable.
- The UI contract is one `Snapshot` event per poll plus separate `enrichment` and `alert` events. Each `AircraftView` carries ENU position and velocity so the front end can dead-reckon smoothly at 60 fps between polls, blending corrections over 300 ms rather than snapping.
- The front end repeats the elevation formula (for hit-testing and the sky view). It has its own unit tests.

## Views
- The **sky view** is a polar plot with north up and east to the right, as you'd draw it on paper. A real sky chart held overhead mirrors east and west; this was left as a documented simplification.
- The map is MapLibre with a style generated from the active theme's CSS variables, so the basemap changes with the theme. Planes are drawn on a transparent canvas through `map.project` rather than as map layers, which keeps radar and map rendering identical.
- MapLibre 6 loads its worker as a separate module; it is imported with `?url` and registered with `setWorkerUrl`.

## Fixtures and mock data
- `tests/fixtures/live_*` are real captures. `synthetic_*` and `demo_replay.json` are generated and are labelled as such wherever they are used.
- Demo mode (no home set) and the browser dev backend (`src/lib/mock/engine.ts`) produce synthetic traffic. Demo mode is always labelled "DEMO" in the UI. Live mode never falls back to synthetic data.

## Verification limits
- The sandbox could not reach the full live APIs, and has no display, so the desktop shell was compiled and linted there but not launched. The front end was checked through headless Chrome screenshots using the browser mock backend. The installers are built by GitHub Actions.
- Builds are unsigned.
