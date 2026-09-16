# Lot movement warning cleanup

## Goal

Clean up warnings introduced by the lot movement ledger UX changes before wrapping the feature.

## Tasks

- [x] Align frontend movement kind strings with backend movement kind enum.
- [x] Remove unused modal props and stale call-site bindings.
- [x] Run frontend build/checks and record remaining warnings.
- [x] Fix remaining `ScanSearchBox.svelte` invalid CSS warning.
- [x] Remove dead Rust lot-movement helper code that generated dev warnings.
