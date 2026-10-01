# App-wide Context Menu Policy

## Objective and accepted behavior
Keep the native WebView editing menu for editable text controls so system Copy/Paste work reliably. On non-editable app surfaces, suppress the browser/WebView context menu and show no replacement menu. User explicitly approved removing the Caduxo custom menu entirely.

## Scope
- In: one shared editable-control predicate for mouse and keyboard context-menu gestures; native pass-through for editable controls; suppression for non-editable surfaces; remove the unused custom menu implementation and its translations.
- Out: custom menu UI/actions, custom clipboard integration, native OS window/tray menus, product-specific actions.

## Task checklist
- [x] T1 — Initial custom menu prototype and native-menu suppression. This was superseded by the user-approved simplification below.
- [x] T2 — Independent reviews identified and helped remediate implementation defects; earlier `npm run check` / `npm run build` passed.
- [x] T3 — Keep native WebView editing menu on editable text controls. User approved.
- [x] T4 — Remove the unused Caduxo menu and all its state/actions/i18n; on non-editable surfaces prevent the native browser menu without replacement. User explicitly requested removal.
- [x] T5 — Bump version for the behavior fix as a SemVer patch: synchronized all product metadata to `0.2.1`. Evidence: all six version entries agree; `npm run check` passed (0 errors/warnings); `npm run build` passed in 2.40s (existing >500 kB advisory).

## Routing and checks
- T1 multi-file writer; T2 independent verifier; T3 one-file policy edit; T4 bounded worker for multi-file deletion, parent follow-up for suppression-only handler, independent verification.
- Strict TDD is not configured and no JS test runner exists. Verification: `npm run i18n:generate`, `npm run check`, `npm run build`, read-only source review.

## Verification evidence
- `npm run i18n:generate`: completed, typesafe-i18n reports files up to date.
- `npm run check`: passed, 0 errors and 0 warnings.
- `npm run build`: passed in 2.38s; Vite reports the existing informational >500 kB JS chunk advisory.
- Independent verifier confirmed: editable fields pass through to native context menu; non-editable surfaces call only `preventDefault()` and open no menu; Menu/Shift+F10 follows same rule; no custom component, state, actions, or i18n namespace remain.
- Manual Tauri runtime test across platforms has not been performed.

## Delivery
- Branch: `feat/app-context-menu`, based on `fe257af`.
- Work-unit commit: `171b8f3 fix(ui): preserve native context menus for fields`.
- No push or PR created.
- Final working tree was clean after the commit.

## Next step
PR to `main` will run the GitHub Actions Ubuntu and Windows typecheck/production-build matrix. Manual Tauri runtime testing was done by the user and confirmed working.