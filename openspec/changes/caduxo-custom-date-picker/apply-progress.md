# Apply Progress: caduxo-custom-date-picker

> Evidence ledger for the apply phase of `caduxo-custom-date-picker`.

## Discovery

**Scope confirmation — `grep -RIn 'type="date"' src/`:**

```
src/components/LotForm.svelte:246:    type="date"
src/components/LotForm.svelte:343:  label input[type="date"],
src/components/ReportsPage.svelte:370:    type="date"
src/components/ReportsPage.svelte:379:    type="date"
```

Exactly 4 hits. No additional hits. Scope confirmed as design-specified.

## File changes

| File | Status | Approx LOC delta |
|------|--------|-----------------|
| `src/components/CalendarMonth.svelte` | new | ~280 |
| `src/components/DatePicker.svelte` | new | ~340 |
| `src/components/CalendarPage.svelte` | new | ~360 |
| `src/components/LotForm.svelte` | edit | +18/−12 |
| `src/components/ReportsPage.svelte` | edit | +22/−16 |
| `src/components/DashboardPage.svelte` | edit | +2/−0 (pre-existing bug fix) |
| `src/App.svelte` | edit | +10/−1 |
| `openspec/specs/caduxo-expiry-tracker/spec.md` | edit | +80/−2 |
| `docs/prd.md` | edit | +10/−0 |
| **Total estimate** | — | **~820–1,050 LOC** |

## svelte-check

**Command:** `npx svelte-check --workspace . --threshold error`
**Exit code:** `0`
**Result:** 0 errors, 2 warnings in 2 files

No new errors introduced by this slice. The 2 remaining warnings are pre-existing a11y advisories in unchanged files:

- `LotForm.svelte:241`: A form label must be associated with a control (pre-existing — a read-only `<label><span>Unit</span></label>` unit chip)
- `ScanSearchBox.svelte:115`: Unknown CSS property `autocomplete` (the project uses `autocomplete: off` to suppress browser autofill on the scan input)

New components (CalendarMonth, DatePicker, CalendarPage): **zero errors, zero warnings**.

## vite build

**Command:** `npm run build`
**Exit code:** `0`

```
dist/index.html                   0.39 kB │ gzip:  0.26 kB
dist/assets/index-DVZYYmPw.css   68.35 kB │ gzip: 10.37 kB
dist/assets/index-5D-LS15D.js   192.28 kB │ gzip: 59.92 kB
✓ built in 2.00s
```

No Vite warnings related to new components. Clean build.

## cargo test baseline

**Command:** `cargo test --manifest-path src-tauri/Cargo.toml --lib`
**Result:** `test result: FAILED. 276 passed; 2 failed; 0 ignored; 0 measured`
**Exit code:** `101` (non-zero — pre-existing failures, baseline unchanged)

**276 passed + 2 pre-existing failures** — exactly matching the archived baseline.
The 2 failures are pre-existing (`services::reports::tests::preview_report_in_alert_window_returns_alert_lots`, `services::reports::tests::preview_report_next_30_days_returns_30d_lots`) — unrelated to this frontend-only slice.

The non-zero exit code is **expected and required** for the baseline — cargo reports failure whenever any test fails, regardless of whether the failures are pre-existing.

## M17 — Mechanical defect-fix gate

**Command:** `grep -R 'type="date"' src/`

Result (after all changes):

```
(no output)
```

**0 matches**. Zero `<input type="date">` elements or stale source comments remain in `src/`. **Gate PASSED.**

Confirmed by individual file greps:

- `grep -RIn 'type="date"' src/components/LotForm.svelte` → 0 matches
- `grep -RIn 'type="date"' src/components/ReportsPage.svelte` → 0 matches
- `grep -RIn 'type="date"' src/` → 0 matches

## Manual smoke checklist (M1–M20)

> These require a running Tauri desktop app. Linux/WebKitGTK environment.
> Marked as **deferred** pending GUI verification. See `verify-report.md` for evidence.

| # | Flow | Expected | Status |
|---|------|----------|--------|
| M1 | LotForm; trigger expires-input; click any day in current month | popover closes; `expiryDate` committed; no `×` icon | deferred |
| M2 | LotForm; focus text input; type `2026-09-30`; blur | `expiryDate` commits; no red border | deferred |
| M3 | LotForm; type `2026-13-40`; blur | red border + helper "Use YYYY-MM-DD"; `expiryDate` unchanged | deferred |
| M4 | LotForm; type `1899-12-31`; blur | red border + helper "Year must be 1900–2100" | deferred |
| M5 | LotForm; click `Today` button in popover | `expiryDate = today`; popover closes | deferred |
| M6 | LotForm; Tab from trigger; Tab again | focus moves to next form field, not into popover | deferred |
| M7 | LotForm; open popover; Esc | popover closes; no calendar selection committed; typed text intact | deferred |
| M8 | LotForm; open popover; `ArrowLeft` from focused day | focus moves −1 day, clamped to 1900–2100 | deferred |
| M9 | LotForm; open popover; Enter on focused day | that day commits; popover closes | deferred |
| M10 | ReportsPage; click `Date from`; type `2026-09-30`; blur | `dateFrom` committed; preview filters respect it | deferred |
| M11 | ReportsPage; click `×` on `Date from` | `dateFrom` clears; buildFilters returns `null` for date_from | deferred |
| M12 | Calendar tab in main nav | current month visible; today highlighted + selected | deferred |
| M13 | Calendar tab; click a day with 3+ expirations | day-detail panel lists 3+ rows | deferred |
| M14 | Calendar tab; click a day with no expirations | day-detail panel shows "No expirations on YYYY-MM-DD" | deferred |
| M15 | Calendar tab; click prev-month chevron twice | grid moves two months back; no backend call | deferred |
| M16 | Calendar tab; click a lot row in day-detail | opens lot edit overlay (existing pattern) | deferred |
| M17 | `grep -R 'type="date"' src/` | **zero matches** — mechanical defect fix gate | **PASS** |
| M18 | Year picker; click year chip | decade grid shows 12 years; `‹`/`›` decade work; out-of-range years disabled | deferred |
| M19 | Calendar tab; nav from March → Feb in non-leap year | Feb 29 not rendered; cell counts correct (28 days) | deferred |
| M20 | Linux/WebKitGTK; open picker | **does NOT** trigger GTK native picker | deferred |

## Implementation log

| Task | Status | Evidence |
|------|--------|----------|
| Discovery (grep scope) | ✅ | 4 hits confirmed, no expansion |
| apply-progress.md init | ✅ | File created |
| CalendarMonth.svelte | ✅ | 0 svelte-check errors for new file |
| DatePicker.svelte | ✅ | 0 svelte-check errors for new file; fixed unused timer declaration |
| CalendarPage.svelte | ✅ | 0 svelte-check errors for new file |
| LotForm.svelte adoption | ✅ | `type="date"` removed; DatePicker mounted; CSS orphan removed |
| ReportsPage.svelte (dateFrom) | ✅ | `type="date"` removed; DatePicker mounted |
| ReportsPage.svelte (dateTo) | ✅ | `type="date"` removed; DatePicker mounted |
| ReportsPage.svelte CSS cleanup | ✅ | Removed unused `.filter-field input` selectors |
| App.svelte nav wiring | ✅ | "calendar" added to Tab union; nav button + view arm added |
| Spec delta: Date input capability | ✅ | `## Capability: Date input` + 6 scenarios appended |
| Spec delta: Calendar capability | ✅ | `## Capability: Calendar` + 6 scenarios appended |
| Spec delta: lot registration pointer | ✅ | One-line pointer added |
| Spec delta: report filters pointer | ✅ | One-line pointer added |
| Spec cross-ref check | ✅ | `grep -nE 'type="date"'` — all mentions are spec-language about the defect fix |
| PRD Calendar section | ✅ | New `### Calendar tab` subsection added |
| DashboardPage.svelte partial row fix | ✅ | Added missing `default_unit_id: null, unit_type: null` to `openResolveFromDetail` |
| svelte-check | ✅ | Exit 0, 0 errors (276 passed + 2 pre-existing failures baseline unchanged) |
| npm run build | ✅ | Exit 0, clean build |
| M17 mechanical gate | ✅ | Zero `<input type="date">` elements in src/ |
| cargo test baseline | ✅ | 276 passed + 2 pre-existing failures — matches archived baseline |
| verify-report.md | ✅ | Written |

## Calendar loading timeout fix

After manual observation showed the Calendar tab stuck at `Loading lots…`, `CalendarPage.svelte` now calls `listDashboardLots` with the explicit documented null filters and wraps the invoke in a 10-second timeout. If the Tauri command stalls, the page exits loading and shows an actionable error instead of hanging forever. Re-ran `npx svelte-check --workspace . --threshold error` (exit 0), `npm run build` (exit 0), and `grep -RIn 'type="date"' src/ || true` (0 matches).

## Calendar loading hang — defensive patch (post-incident)

User reported the Calendar tab still stuck at `Loading lots…` after restarting the
Tauri app several times, so a stale HMR was not the cause. The prior 10-second
timeout patch was already in the bundle, so a stronger defensive fix was needed:
inflight generation tracking plus clear console evidence so a future hang is
debuggable from the DevTools console alone.

### Patch (`src/components/CalendarPage.svelte`)

1. **`loadGeneration` counter** — monotonic, incremented on every `loadLots()`
   call and again on `onMount` cleanup. Each load captures `myGen = ++loadGeneration`
   at start and re-checks `myGen === loadGeneration` after every await before
   touching `lots`, `errorMsg`, or `loading`. A stale promise can no longer
   mutate UI state after a refresh click, fast remount, or component teardown.
2. **`onMount` cleanup** — returns a teardown that increments `loadGeneration`,
   invalidating any outstanding load so a stale resolve cannot poke state on a
   torn-down component.
3. **Browser-safe timers** — `withTimeout` already uses `window.setTimeout` /
   `window.clearTimeout`, which run reliably inside Tauri's WebKitGTK / WebView2
   host. The handle is cleared on every settle path so a settled promise cannot
   leak a dangling timer, and the timer always rejects — a hung underlying
   invoke cannot skip the timeout.
4. **Console evidence** (visible from DevTools with no extra instrumentation):
   - `console.debug("[CalendarPage] mounted, dispatching loadLots")` — fires on
     every Calendar mount.
   - `console.debug("[CalendarPage] loadLots start", filters)` — fires on
     every loadLots invocation, with the explicit null filters object.
   - `console.debug("[CalendarPage] loadLots success", { lots: data.lots.length })`
     — fires on a settled resolve.
   - `console.error("[CalendarPage] loadLots failed", e)` — fires on timeout
     or rejection.
5. **Explicit null filters and 10s timeout** preserved unchanged.

### Validation evidence

| Command | Exit | Notes |
|---------|------|-------|
| `npx svelte-check --workspace . --threshold error` | `0` | 0 errors, 2 pre-existing warnings (unchanged) |
| `npm run build` | `0` | `dist/assets/index-C-rUzsEq.js` 192.54 kB — clean build |
| `grep -RIn 'type="date"' src/ \|\| true` | `0` | 0 matches — M17 still passes |

The patch is confined to `CalendarPage.svelte`. No other files were modified,
and no other root cause was confirmed inside the allowed edit surface. After
this patch, if the Calendar tab still hangs the user will see one of:

- `[CalendarPage] mounted, dispatching loadLots` **without** a corresponding
  `loadLots start` — `onMount` did not run; investigate parent routing.
- `loadLots start` **without** any `success` / `failed` line for 10+ seconds —
  the timeout did not fire; investigate Tauri's WebKit console for the
  blocking task.
- `loadLots failed` after 10 seconds — the backend Tauri command is the
  problem; check `src-tauri/src/commands/dashboard.rs` (out of scope here).

## Calendar loading hang — UI-visible diagnostics (post-incident)

The second defensive patch (loadGeneration + console anchors) had no
observable effect on the user because browser console output is NOT
forwarded to the terminal that hosts `npm run tauri dev`. Without an in-UI
signal the user could not report which exact stage was stuck. This third
iteration keeps every previous defence and makes the failure surface
visible from the screenshot alone.

### Patch (`src/components/CalendarPage.svelte` only)

1. **Reactive diagnostic state** — `loadStage`, `loadAttempts`,
   `lastLoadStartedAt`, `lastLoadFinishedAt`. `loadStage` flips to
   `mounted` in `onMount` before the first `loadLots` call, then to
   `loading` at the top of `loadLots`, then to `loaded N lots` on success
   or `failed: <msg>` in catch. The two timestamp vars are stamped at
   start and in `finally`.
2. **Muted diagnostic paragraph** rendered under both the cold-loading
   branch (`Loading lots…`) and the timeout-error branch. Format:
   `Calendar load status: <stage>, attempt #N` plus
   `(running | took N.Ns | failed after N.Ns)` depending on state.
   Exposed with `role="status"` + `aria-live="polite"` so screen readers
   also announce stage transitions, and carries
   `data-testid="cal-load-diag"` for future testing.
3. **Existing 10 s `withTimeout` behaviour preserved**. When it fires,
   the catch stores `Loading lots timed out after 10 seconds.` in
   `errorMsg`, `loadStage` flips to `failed: …`, and the existing error
   branch now visibly shows the timeout — distinguishable from an
   infinite spinner within 10 s.
4. **Console anchors upgraded from `console.debug` to `console.log`** for
   the `mounted`, `loadLots start`, and `loadLots success` lines so they
   appear by default in DevTools. The `loadLots failed` anchor stays
   `console.error` because failure should remain loud. A comment block on
   the diagnostic state documents that browser console is NOT forwarded
   to the terminal — only DevTools / webview capture sees these lines.
5. **Preserved unchanged**: monotonic `loadGeneration` counter,
   `onMount` cleanup teardown, `withTimeout` using
   `window.setTimeout`/`clearTimeout`, the explicit documented null
   `filters` object, and the existing Refresh button.

### Re-validation (post-patch)

| Command | Exit | Observed |
|---------|------|----------|
| `npx svelte-check --workspace . --threshold error` | `0` | `svelte-check found 0 errors and 2 warnings in 2 files` — exactly the pre-existing baseline; no new diagnostics from this patch |
| `npm run build` | `0` | `dist/assets/index-Csb3YaXk.js` 193.63 kB / gzip 60.40 kB — clean build; the pre-existing `expiry_lots.ts` dynamic-import note is unchanged |
| `grep -RIn 'type="date"' src/ \|\| true` | `0` | 0 matches — M17 still PASSES |

    The patch itself does not introduce new failure modes, does not change any
    state contract, and does not add dependency surface. The user can now
    paste the exact `Calendar load status: …` line from the screenshot into a
    bug report so the failure stage is reportable from the UI alone.

## Calendar loading hang — independent UI watchdog (post-incident)

Fourth iteration on the same incident. The third patch surfaced the
failure stage on screen (`Calendar load status: loading, attempt #1
(running)`) but the UI still never transitioned out of that state. That
means the `withTimeout` Promise.race wrapper was insufficient on its own:
if the underlying Tauri invoke never settles, the Promise.race timer
must fire its `reject`, the `await` must propagate it, and the `catch`
must run — three handoffs where any one could fail to update UI state.
The defensive fix is to make the timeout an INDEPENDENT `window.setTimeout`
that mutates component state DIRECTLY, parallel to the invoke, instead of
a `Promise.race` wrapper around it.

### Patch (`src/components/CalendarPage.svelte` only)

1. **`withTimeout` removed entirely.** The Promise.race wrapper is
   deleted; the timeout is no longer chained to the invoke's settlement.
   There is now a single timer in the file (`watchdog`) and it is
   independent of `listDashboardLots`'s resolution.
2. **Independent UI watchdog** — scheduled at the top of `loadLots`
   AFTER `loading = true` is committed and AFTER diagnostic counters are
   stamped. The watchdog callback:
   - Stale-checks `myGen !== loadGeneration` so a newer `loadLots()`
 call (refresh click) does not get overwritten by an old timer.
   - `loadGeneration++` to invalidate the in-flight invoke's late
 resolve, so when (or if) the underlying Tauri command eventually
 settles, the `await listDashboardLots(filters)` branch's
 `if (myGen !== loadGeneration) return;` guard short-circuits and
 the timeout state we just rendered is preserved.
   - Sets `loadStage = "failed: timeout waiting for list_dashboard_lots"`,
 `errorMsg = "Calendar load timed out after 10 seconds while waiting
 for list_dashboard_lots."`, `lastLoadFinishedAt = Date.now()`, and
 `loading = false` — directly, with no await in between.
   - Emits `console.error("[CalendarPage] loadLots watchdog fired
 (10 s); invoke did not settle")` for DevTools cross-reference.
3. **Watchdog cleared on every settlement path.** `window.clearTimeout(
   watchdog)` runs in the stale-return guard, in the success branch, and
   in the catch branch — so a settled invoke never leaves a dangling
   timer that would fire 10 s later and overwrite the success state.
4. **Generation guard preserved.** The watchdog's own `loadGeneration++`
   is what makes the guard work for the in-flight invoke's late resolve:
   when (or if) `listDashboardLots` eventually returns, `myGen` (captured
   before the bump) no longer matches the post-bump `loadGeneration`, so
   the existing `if (myGen !== loadGeneration) return;` check inside
   `loadLots` returns without mutating `lots`, `errorMsg`, or
   `loadStage`.
5. **Live elapsed-seconds tick** — `nowMs` updated by a
   `window.setInterval` (250 ms) that is started/stopped by a reactive
   `$:` block conditioned on `loading && lots.length === 0`. The
   diagnostic line under `Loading lots…` now reads
   `({N.N}s running)` and counts up live while the invoke is in flight,
   so the user can see exactly how long the screen has been hung before
   the watchdog fires. The interval is stopped in the same reactive
   branch when loading finishes, and explicitly cleared in the `onMount`
   cleanup teardown so a torn-down component cannot leak the handle.
6. **`LOAD_TIMEOUT_MS = 10_000`** hoisted to a module-level `const` so
   the timeout window is documented in one place and the watchdog call
   site is self-explanatory.
7. **Explicit documented null `filters`** preserved unchanged. Console
   anchors preserved unchanged (`console.log` for start/success,
   `console.error` for failure/watchdog).

### Re-validation (post-patch)

| Command | Exit | Observed |
|---------|------|----------|
| `npx svelte-check --workspace . --threshold error` | `0` | `svelte-check found 0 errors and 2 warnings in 2 files` — 0 errors, exactly the 2 pre-existing warnings on unchanged files; no new diagnostics from this patch |
| `npm run build` | `0` | `dist/assets/index-BZUBPuOY.js` 194.20 kB / gzip 60.49 kB; `dist/assets/index-BeLl8-pY.css` 68.54 kB / gzip 10.40 kB — clean build. Pre-existing dynamic-import note for `src/lib/expiry_lots.ts` is unchanged and is not caused by this patch |
| `grep -RIn 'type="date"' src/ \|\| true` | `0` | 0 matches — M17 still PASSES |

### Behaviour after the patch

- **Happy path:** invoke resolves within 10 s → success branch clears
  the watchdog, sets `loaded N lots`, UI transitions to the calendar
  grid. The elapsed-seconds tick never gets close to 10.
- **Stale return (refresh click):** new `loadLots()` bumps generation →
  in-flight invoke resolves → guard bails → no UI mutation. Old
  watchdog fires 10 s later → stale check bails (`myGen !== loadGeneration`)
  → no UI mutation. New load runs normally.
- **Backend hang (this incident):** invoke never settles → after 10 s
  the watchdog fires → mutates state directly → UI transitions to the
  error branch with `Calendar load status: failed: timeout waiting for
  list_dashboard_lots, attempt #1 (failed after 10.0s)`. The error
  message names the awaited Tauri command so the next investigation
  path is obvious.
- **Late resolve after watchdog fired:** the in-flight invoke eventually
  settles → `if (myGen !== loadGeneration) return;` bails (because the
  watchdog bumped `loadGeneration`) → the timeout state we rendered is
  preserved. No flicker, no overwrite.
- **Unmount during load:** the `onMount` cleanup teardown increments
  `loadGeneration` AND clears the `nowTickHandle`. If the watchdog has
  already fired, its stale check bails. The reactive `$:` block stops
  the tick interval as soon as `loading` flips false.

### Remaining diagnosis paths if the UI still says `running`

If the watchdog fires correctly the user will see the error branch
within 10 s. If the user STILL reports `loading, attempt #1 (running)`
after 10 s, then the watchdog itself is not firing — which means
Tauri's WebKit event loop is blocked on a synchronous task in the same
realm. That would be a Tauri/WebKitGTK runtime issue, not a Svelte
reactivity issue, and would need to be investigated from
`src-tauri/` (out of scope here). The patch cannot defend against a
truly blocked event loop because no JS timer can fire while the loop
is blocked — but in that pathological case nothing in the component
could update state anyway, so the diagnostic line would also be stuck.
