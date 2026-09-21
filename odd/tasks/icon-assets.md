# Icon assets

## Goal
Prepare the provided icon assets from `/home/xeworg/Descargas/iconos/` for use in Caduxo.

## Tasks

- [x] Inspect `1.png`, `2.png`, and `3.png` and decide asset roles: app icon/system tray/navbar mark from `1.png`, README image from `3.png`, and whether `2.png` has any useful role.
- [x] Generate project-ready icon outputs in the appropriate Caduxo asset locations, preserving existing Tauri icon names/sizes where required.
- [x] Wire the navigation brand mark to the prepared app icon if the existing UI has a suitable brand/title area.
- [x] Run focused validation for asset presence and frontend/Tauri build compatibility where practical.

## Asset role decisions

| Source | Decision | Rationale |
| --- | --- | --- |
| `/home/xeworg/Descargas/iconos/1.png` (1254×1254, RGBA, no text) | **Adopted** as primary mark. | Square aspect, transparent corners, no baked-in typography → most versatile surface (app icon, system tray, navbar mark, future favicons). |
| `/home/xeworg/Descargas/iconos/2.png` (1254×1254, RGBA, with "Caduxo" wordmark + tagline "Controla hoy, evita pérdidas mañana") | **Not adopted.** | Bakes in a Spanish tagline that competes with the cleaner `3.png` for any horizontal/wordmark use and would force a single marketing voice on every surface. Square format also makes it a worse icon than `1.png`. Not used. |
| `/home/xeworg/Descargas/iconos/3.png` (2010×782, RGBA, horizontal lockup with wordmark, no tagline) | **Adopted** as README/horizontal brand asset. | Wide aspect, neutral wordmark without tagline → best fit for README banner, About dialog, splash screen, store listings. |

## Outputs

### Tauri app/tray icons (regenerated from `1.png`)
Tool: `npx --no-install @tauri-apps/cli icon /home/xeworg/Descargas/iconos/1.png`
Output dir: `src-tauri/icons/` (Tauri CLI default, next to `tauri.conf.json`)

Regenerated:
- Desktop: `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`, `icon.png` (512×512), `icon.ico` (multi-res 16/24/32/48/64/256), `icon.icns`
- Windows Store: `Square30x30Logo.png`, `Square44x44Logo.png`, `Square71x71Logo.png`, `Square89x89Logo.png`, `Square107x107Logo.png`, `Square142x142Logo.png`, `Square150x150Logo.png`, `Square284x284Logo.png`, `Square310x310Logo.png`, `StoreLogo.png`
- iOS: full `AppIcon-*@1x/@2x/@3x.png` set under `src-tauri/icons/ios/`
- Android: full `mipmap-*dpi/ic_launcher*.png` set + adaptive-icon XML under `src-tauri/icons/android/`

`tauri.conf.json` `bundle.icon` array already lists the five required entries (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`) — no config change needed.

### Frontend brand mark (derived from `1.png`)
Tool: `convert /home/xeworg/Descargas/iconos/1.png -resize 64x64 src/assets/caduxo-mark.png` and `-resize 128x128` for the 2× variant.
- `src/assets/caduxo-mark.png` — 64×64 RGBA, 1× navbar mark
- `src/assets/caduxo-mark@2x.png` — 128×128 RGBA, 2× navbar mark

### Navbar wiring
File: `src/App.svelte`
- Added `import caduxoMark from "./assets/caduxo-mark.png";` at the top of the script.
- Inserted a `<img src={caduxoMark} ... width="24" height="24" aria-hidden="true">` inside the existing `app-brand` button, immediately before the "Caduxo" wordmark.
- Updated `.app-brand` CSS to `display: inline-flex; align-items: center; gap: 0.5rem;` so the mark and wordmark stay vertically aligned.
- Added `.app-brand-mark { height: 1.5rem; width: 1.5rem; flex: none; }` for sizing.
- Image is decorative (`alt=""`, `aria-hidden="true"`); accessible name remains `$LL.nav.dashboard()` from the button's `aria-label`.

### README-ready banner (derived from `3.png`)
Tool: `convert /home/xeworg/Descargas/iconos/3.png docs/assets/caduxo-readme-banner.png`
- `docs/assets/caduxo-readme-banner.png` — 2010×782 RGBA (original aspect preserved)
- No README created (none exists in the repo yet — task instructed not to create one).

## Validation

- `npx --no-install @tauri-apps/cli icon /home/xeworg/Descargas/iconos/1.png` → completed with no errors; produced 50+ asset files across desktop/iOS/Android. Confirmed with `identify`:
  - `icon.png` → PNG 512×512
  - `32x32.png` → PNG 32×32
  - `128x128.png` → PNG 128×128
  - `128x128@2x.png` → PNG 256×256
  - `icon.ico` → multi-resolution (32, 16, 24, 48, 64, 256)
  - `icon.icns` → generated, but ImageMagick has no ICNS decode delegate (cosmetic only — Tauri's bundler reads it natively on macOS).
- `identify src/assets/caduxo-mark.png src/assets/caduxo-mark@2x.png docs/assets/caduxo-readme-banner.png` → confirms 64×64, 128×128, 2010×782 PNG/RGBA.
- `npm run check` (svelte-check) → `svelte-check found 0 errors and 0 warnings`. The new `<img>` import and the modified `.app-brand`/`.app-brand-mark` rules type-check clean.

## Notes / follow-ups

- No new runtime dependency added. The Tauri icon CLI is already a dev dependency (`@tauri-apps/cli@2.11.4`) and ImageMagick is preinstalled in the environment.
- `2.png` was inspected and consciously rejected; no asset derived from it.
- If a README is later authored, reference the banner via `../docs/assets/caduxo-readme-banner.png` (relative from repo root: `docs/assets/caduxo-readme-banner.png`).
- Per the delegation contract, this change was **not committed**. Branch `feat/scanner-quick-operations` working tree now contains the regenerated Tauri icons, the new `src/assets/` and `docs/assets/` directories, the `App.svelte` navbar wiring, and this updated task file.
