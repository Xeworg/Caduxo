# Apply Progress — caduxo-expiry-tracker / Slice 1

## Slice: Foundation and persistence skeleton

### Status: COMPLETE ✅

---

## Completed tasks

### 1. Project foundation

| Task | Status |
|------|--------|
| Initialize Tauri v2 project with Svelte + TypeScript | ✅ |
| Configure Rust workspace and app metadata | ✅ |
| Establish modular layered backend structure | ✅ |
| Add SQLite support with `sqlx` | ✅ |
| Add database migration runner | ✅ |
| Add app data directory resolution for Windows/Linux | ✅ |
| Add basic error handling shape shared by Tauri commands | ✅ |
| Add structured Rust logging with safe app-local log output | ✅ |
| Add baseline Rust test harness for backend/domain/persistence code | ✅ |
| Add baseline frontend test harness when non-trivial UI logic begins | ⏭ Deferred (out of scope for foundation slice) |

---

## Implementation notes

### Tauri v2 scaffold

- `tauri init` with `ci` flag, `frontendDist: ../dist`, `devUrl: http://localhost:1420`
- Window: 1100×720, resizable, devtools enabled
- Identifier: `com.caduxo.app`
- Notification plugin registered (`tauri-plugin-notification`)
- Package name in `Cargo.toml`: `caduxo` (not `app`)

### Module tree

```
src-tauri/src/
├── main.rs               — entry point, calls caduxo_lib::run()
├── lib.rs                — async setup: app data, logging, DB, state
├── error.rs              — AppError, DomainError, InfrastructureError, CommandError
├── logging.rs            — tracing subscriber, app-local log file (logs/caduxo.log)
├── state.rs              — AppState (DB pool), ping_db()
├── commands/
│   ├── mod.rs
│   └── health.rs         — health_check command
├── db/
│   ├── mod.rs            — re-exports DbPool, open_pool, run_migrations
│   ├── pool.rs           — SQLitePoolOptions, SqliteConnectOptions
│   ├── migrations.rs     — inline migration definition, build_migrator()
│   └── repositories/      — stub module (populated in later slices)
├── domain/
│   ├── mod.rs
│   ├── alerts.rs         — alert window, expiry, days-until pure functions
│   ├── expiry_status.rs — Urgency enum, classify_urgency, notification check
│   ├── lot_resolution.rs — remaining qty, valid qty, fully-resolved
│   └── validation.rs    — validate_sku, _description, _barcode, _name
├── dto/
│   └── mod.rs            — stub (populated in later slices)
└── services/
    ├── mod.rs
    └── health.rs         — HealthService::check
```

### Dependencies added

- `sqlx` with `runtime-tokio-rustls`, `sqlite`, `migrate`, `chrono` features
- `tokio` with `full` feature
- `uuid` with `v4`, `serde` features
- `chrono` with `serde` feature
- `thiserror`, `anyhow` for error handling
- `tracing`, `tracing-subscriber` with `env-filter`
- `dirs` for app directory resolution
- `tempfile` in dev-dependencies

### Logging design

- App-local: `$APPDATA/Caduxo/logs/caduxo.log` (Linux/macOS: `~/.local/share/Caduxo/logs/`)
- Console: enabled in debug builds only
- Structured fields: version, data_dir, dev_mode, operation counts, durations
- **Safe by default**: no product names, SKUs, barcodes, imported rows, or notes in logs

### Migration harness

- Inline migration definitions via `MIGRATIONS` constant
- `build_migrator()` constructs `sqlx::migrate::Migrator` without compile-time `migrate!` macro
- V1: creates `_caduxo_skeleton` table (placeholder smoke-test)
- Real schema migrations (stores, products, lots, etc.) added in Slice 2

### Domain tests

- 19 tests covering pure domain logic (alerts, expiry status, lot resolution, validation)
- DB pool smoke test using in-memory SQLite

---

## Verification evidence

```bash
# Rust compile
cd src-tauri && cargo check 2>&1 | tail -5
# → "Finished `dev` profile ... target(s) in 0.95s" ✅

# Rust tests
cd src-tauri && cargo test 2>&1 | tail -10
# → "19 passed; 0 failed; 0 ignored" ✅

# TypeScript check
npx tsc --noEmit 2>&1
# → (no output = clean) ✅

# Frontend build
npm run build 2>&1
# → "✓ built in 271ms" ✅

# Clippy (warnings only, no errors)
cd src-tauri && cargo clippy 2>&1 | grep "^error"
# → (no output = no errors) ✅
```

### Dead-code warnings (expected)

All 18 warnings are `dead_code` for domain/service functions not yet wired to commands.
These are intentional — the layered structure is in place for future slices to consume them.

---

## Out-of-scope decisions noted for later slices

1. **Frontend test harness**: deferred until non-trivial UI logic begins (per task spec)
2. **Real schema tables**: placeholder V1 migration only; full schema in Slice 2
3. **Product/store/lot/dashboard/notification/CSV/report/backup features**: future slices
4. **Visual polish**: basic app shell only; full UI in later slices

---

## Risks

| Risk | Mitigation |
|------|-----------|
| `dirs` crate path resolution differs on Windows | Use `app.path().app_data_dir()` from Tauri as primary; `dirs` as fallback |
| Migration inline approach diverges from `migrate!` macro | Design is compatible; can switch to `migrate!` by creating `migrations/` dir |
| Clippy warnings for unused domain functions | Intentional — functions are ready for later slices to consume |

---

## Next recommended action

**Slice 2 — Database schema and migrations**: Add all schema tables, indexes, and constraints per `design.md` §Database schema. Wire `is_first_run`, store CRUD, and the first Tauri command implementations.

Priority cleanup for Slice 2: update `src-tauri/src/logging.rs` to use `&Path` instead of `&PathBuf` in logging path function signatures before making clippy a strict gate.
