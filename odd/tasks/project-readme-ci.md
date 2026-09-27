# Project README, licensing, and Linux/Windows CI

## Goal
Document Caduxo for contributors and users, add GitHub Actions coverage for Linux and Windows Tauri builds, establish GPLv3 contribution terms, and keep the bilingual README technically complete.

## Tasks
- [x] Create a root README with project overview, features, prerequisites, local development, checks, packaging, and platform notes.
- [x] Add a GitHub Actions workflow that installs platform dependencies and builds Tauri artifacts on Ubuntu and Windows.
- [x] Verify workflow syntax and local project checks.
- [x] Translate the README to Spanish and add the Caduxo banner image.
- [x] Add an equivalent English section so the README is bilingual.
- [x] Add GPLv3 license and contribution guidance.
- [x] Document that Caduxo is local software intended for one computer, not an online service.
- [x] Review and polish README structure and terminology.
- [x] Audit documented technologies and add donation/support guidance.

## Evidence
- `npm run check` passed: 0 errors and 0 warnings.
- `.github/workflows/build.yml` passed Pi YAML validation during editing.
- Fixed the linuxdeploy download URL after verification identified a malformed split URL.
- README has equivalent English and Spanish sections, language selector, banner, local/single-device scope, GPLv3 terms, no-warranty language, and Ko-fi support badge.
- Documented verified technologies, i18n, build tooling, Tauri integrations, themes, system tray, macOS non-priority, frontend test-runner gap, and corrected Linux data paths.
- `LICENSE` contains GPLv3 text with `Caduxo Contributors` attribution.
- `CONTRIBUTING.md` states submitted contributions are licensed under GPLv3.
