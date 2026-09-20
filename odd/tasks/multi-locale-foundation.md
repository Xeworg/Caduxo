# Multi-locale foundation

## Goal
Remove the most brittle two-locale assumptions in the frontend i18n foundation so adding a third locale starts from generated locale types and data-driven UI instead of hardcoded `en`/`es` branches.

## Scope
- Derive app locale typing from generated typesafe-i18n `Locales`.
- Replace manual locale whitelist checks with generated `isLocale`.
- Make OS/WebView detection data-driven through a supported locale table.
- Make the Configuration language selector and detected-language hint data-driven.
- Align settings DTO locale types with the shared supported locale type.

## Non-goals
- Do not add a third locale yet.
- Do not rewrite all pluralization or formatter debt in this slice.
- Do not touch backend user-message localization in this slice.

## Tasks
- [x] Task 1: Refactor locale typing/detection/Configuration selector to be data-driven.
- [x] Task 2: Run focused frontend checks and fix regressions within the same scope.
- [x] Task 3: Commit the verified slice.

## Evidence
- Writer validation: `npm run i18n:generate`, `npx svelte-check --tsconfig ./tsconfig.json`, `npm run build` passed.
- Independent verifier: `gentle-ai-verify` PASS; `npm run i18n:generate` idempotent and `npx svelte-check --tsconfig ./tsconfig.json --threshold error` reported 0 errors / 0 warnings.
- Native assessment: unavailable, so separate verifier was run as required.
- Commit: `c0ef58a feat: derive locale configuration from i18n data`.
