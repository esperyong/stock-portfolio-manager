# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A cross-platform **desktop** app (macOS/Windows/Linux) for personal stock-portfolio management across US, CN (A-share), and HK markets. Built with **Tauri 2.0**: a **Rust** backend (`src-tauri/`) and a **React 18 + TypeScript** frontend (`src/`), talking to a local **SQLite** database. The UI is in Chinese; user-facing strings and error messages are written in Chinese.

## Commands

```bash
npm install                    # install frontend deps (run once)
npm run tauri dev              # run the full app (Vite dev server + Rust backend) — use this to develop
npm run tauri build           # build platform installers into src-tauri/target/release/bundle/

# Frontend-only
npm run dev                   # Vite only — invoke() calls fail without the Rust backend; rarely what you want
npx tsc --noEmit              # type-check the frontend (no emit)
npm run build                 # tsc + vite build (what CI runs for the frontend)

# Rust backend (from src-tauri/)
cd src-tauri && cargo test                 # run all backend tests
cd src-tauri && cargo test <test_name>     # run a single test by name
cd src-tauri && cargo build                # compile-check the backend without running the app
```

Versioning: `package.json` and `src-tauri/Cargo.toml` share the version (currently 1.2.0) — keep them in sync. Pushing a `v*` git tag triggers `.github/workflows/build.yml`, which builds installers for all platforms as draft-release assets.

## Commit conventions

Do **not** add any Claude/AI attribution to commits — no `Co-Authored-By: Claude ...` trailer and no "Generated with Claude Code" line. Commit messages describe the change only.

## Architecture

### The IPC boundary is the center of gravity

Every backend capability is a Tauri **command**. The flow is always:

```
React component → Zustand store → invoke("command_name", args)   [frontend, src/]
                                        ↓  Tauri IPC
#[tauri::command] fn command_name(...)  →  service fn  →  db (SQLite)   [backend, src-tauri/]
```

- **Commands must be registered in two places to exist:** defined in `src-tauri/src/commands/<domain>.rs`, then listed in the `invoke_handler![...]` macro in `src-tauri/src/lib.rs`. Forgetting the second step is the most common "command not found" bug.
- **Argument casing:** commands use `#[tauri::command(rename_all = "camelCase")]`. The frontend passes **camelCase** keys (`{ accountId }`); the Rust signature uses **snake_case** (`account_id`). They must correspond.
- **Types are duplicated by hand** across the boundary: Rust structs in `src-tauri/src/models/` (serde-serialized) mirror TypeScript interfaces in `src/types/index.ts`. Change one, change the other. Serde uses the Rust field names as-is (snake_case), so TS interfaces use snake_case field names too.

### Backend layering (`src-tauri/src/`)

- `commands/` — thin IPC handlers. Validate input, lock the DB, call services, map errors to `Result<T, String>` (the error string is shown to the user, so write it in Chinese).
- `services/` — business logic (quotes, exchange rates, snapshots, performance, imports, AI). This is where non-trivial logic lives.
- `models/` — serde data structs shared over IPC.
- `db/mod.rs` — the `Database` struct and **all schema/migrations** (see below).
- `lib.rs` — app setup, Tauri state registration, command registration, and the startup background task.

### Database & migrations

- One SQLite file at the OS app-data dir (`com.portfolio.manager/portfolio.db`), opened once and shared as `Database { conn: Mutex<Connection> }` via Tauri managed state. Every command does `db.conn.lock()` — the whole DB is serialized behind one mutex, so keep locked sections short and never hold the lock across an `.await`.
- **There is no migration framework.** `Database::run_migrations()` in `db/mod.rs` runs top-to-bottom on every startup and must stay idempotent:
  - New tables → `CREATE TABLE IF NOT EXISTS ...`.
  - New columns → `let _ = conn.execute_batch("ALTER TABLE x ADD COLUMN ...");` — the `let _ =` deliberately swallows the "duplicate column" error on subsequent runs.
  - To evolve the schema, **append** to `run_migrations()`; don't rewrite existing statements (users' DBs already ran them).
- Money/P&L columns are stored per-market (`us_*`, `cn_*`, `hk_*`) so values can be aggregated in their native currency and converted at display time.

### Caching & the startup background refresh

Live data (quotes, FX rates) uses a two-tier cache: an in-memory cache (`QuoteCache`, `ExchangeRateCache`, Tauri state) backed by persisted DB tables (`cached_quotes`, `cached_exchange_rates`). On startup `lib.rs`:
1. Warm-loads the in-memory caches from the DB so the UI renders instantly with last-known prices.
2. Spawns a `tokio` task (after a short delay) that force-refreshes all holding quotes from upstream, persists them, and emits **events** to the frontend: `quotes-refreshed` (re-render) and `quote-warning` (banner text).

Frontend listens with `@tauri-apps/api/event`. The quote-warning banner has two delivery paths (a `quote-warning` event **and** a `take_quote_warning` polling fallback) because webview listener registration can race the backend emit — see the comments in `src/App.tsx`. `quoteStore.quoteWarning` is the single source of truth for that banner.

### Frontend state (`src/stores/`)

One Zustand store per domain (`holdingStore`, `transactionStore`, `quoteStore`, …). Stores are the only place that calls `invoke()`; components consume stores. A store method typically: set `loading`, `await invoke(...)`, update state, catch → set `error: String(err)`. Follow the existing store shape when adding one.

## Domain rules that aren't obvious from the code

- **Markets** are `US | CN | HK`; **currencies** `USD | CNY | HKD`. These are CHECK-constrained in SQLite and typed in `src/types/index.ts`.
- **Transaction types** are `BUY | SELL | OPEN | PAY`:
  - `OPEN` = initial position entry when a holding is first created — **no cash impact**, not a real trade.
  - `PAY` = dividend received.
  - Cash effect on the account: `BUY = -(amount+commission)`, `SELL = amount-commission`, `PAY = +amount`, `OPEN = 0` (see `transactions.rs`).
- **Average-cost recalculation** happens in `commands/transactions.rs` on every create/update/delete of a transaction. Whether a `SELL`/`PAY` adjusts the cost basis (net-cost method) is controlled per-market by the `cn_/us_/hk_adjust_sell_pay_cost` flags in `quote_provider_config`. Default: **CN adjusts** (A-share convention), **US/HK do not**. Editing transaction logic means preserving this reversal-and-replay behavior.
- **Fractional shares** are allowed only for `US` holdings; `CN`/`HK` must be whole shares (enforced in `holdings.rs`). Cash holdings use symbols prefixed `$CASH-` and are exempt.
- **Quote providers are pluggable per market.** `quote_provider_config` selects a provider (`xueqiu` / `yahoo` / `eastmoney` / `tencent`) for each of US/HK/CN. **Xueqiu requires a user-supplied cookie (`xq_a_token`) and `u` value**, entered in Settings — these are distinct values, do not copy one into the other. On upstream failure the service sets `LAST_QUOTE_WARNING`, surfaced to the UI.
- **Options** (`option_records` table) track sold puts/calls. `contract_status` is computed and stored **only at import time** (not recomputed on every load) — the recent perf optimization. Options contract matching accounts for stock splits (`stock_splits`, `option_share_lots` tables).
- **Imports** support several brokers via CSV (Firstrade, IB, Moomoo, 同花顺/THS) plus OCR from 同花顺 screenshots — see the `ImportFrom*` modals in `src/pages/Transactions/` and `src/pages/Holdings/`, and `commands/import_export.rs` + `commands/ocr.rs`.

## `tools/`

Standalone one-off Rust CLI utilities (separate Cargo projects) that operate **directly on the SQLite DB** — e.g. `backfill_open_transactions` (add missing OPEN records), `normalize_hk_symbols` (strip leading zeros from HK symbols). Run them against a DB copy; they are maintenance scripts, not part of the app build.
