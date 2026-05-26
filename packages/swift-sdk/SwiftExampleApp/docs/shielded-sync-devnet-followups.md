# Shielded sync devnet test — open follow-ups

Captures the punch list from the live test session against paloma
(`http://44.238.203.84:8080` quorum-list, DAPI on the 13 masternodes
at `68.67.122.{85,86,87,88,192,193,195,196,197,198,199,206,207}:1443`).

Companion to [`shielded-sync-timing-spec.md`](shielded-sync-timing-spec.md)
which covers what already shipped.

## Status legend

- **done** — landed in this branch, see commits
  `32beb346c3` (timing UI + bind) and `e4b69dbebc` (devnet wiring).
- **open** — not yet implemented.

## Issues discovered & fixes applied

| # | Issue | Status |
|---|-------|--------|
| 1 | Devnet wasn't wired into the iOS app — `SDK.init` only honored DAPI override on regtest; trusted context provider panicked on Devnet without quorum URL | **done** — new `DashSDKConfig.quorum_url`, `platformQuorumURL` UserDefaults key, devnet endpoint inputs in OptionsView |
| 2 | Stale cached `PlatformWalletManager` ignored fresh SDK on network re-activation → "no available addresses" forever | **done** — `WalletManagerStore.activate` compares SDK handle, rebuilds on mismatch |
| 3 | `SDKLogger.log` invisible without Xcode debugger attached | **done** — also routed through NSLog so `simctl spawn booted log stream` captures it |
| 4 | Only one DAPI node per SDK by default — entire shielded sync funnels through one gateway | **open** — see P1.1 |
| 5 | `Notes Synced` UI value stays at 0 throughout cold sync, jumps at end | **open** — see P1.2 |
| 6 | `lastSyncDuration` overwritten by short steady-state passes; cold-sync number lost | **open** — see P0.1 |
| 7 | Wallet A bound on paloma shows balance = 0 despite `Notes Synced ≈ 1M` | **open / investigating** — see P0.2 |
| 8 | DAPI nodes entered by hand; `/masternodes` returns all 13 with HTTP port → could auto-derive | **open** — see P1.1 |

## Proposed plan

### P0 — make the headline measurement reliable

#### P0.1 Preserve cold-sync duration

Track three values in `ShieldedService`:
- `lastSyncDuration` (most recent pass — already exists)
- `longestSyncDuration` (max ever this session — survives steady-state passes)
- Reset on `bind` / `reset` / `clearLocalState`

UI: stack "Last sync duration: 3 s" + "Longest pass: 1247 s" in the
ZK Shielded Sync Status section. The longest one is the cold-sync
headline number we want to keep across subsequent re-passes.

#### P0.2 Investigate the missing 400,000 balance

We have evidence paloma IS the snapshot (1M notes synced), but wallet A's
4 owned notes didn't surface as balance. Confirmed equal: ZIP-32
derivation between chain-side bake (`shielded_test_wallets.rs:60-65`,
`coin_type=1`) and wallet-side (`OrchardKeySet::from_seed`,
`coin_type=1` on Devnet via `keys.rs:68-71`). Open hypotheses:

- **H1 — Paloma is a stale image.** Deployed from a commit predating the
  shielded snapshot machinery, or built without `SDK_TEST_DATA=true`.
  1M notes exist but they're all filler — wallet A's owned notes were
  never seeded.
- **H2 — iOS persistence bug.** Decryption succeeds at the Rust layer
  but `PersistentShieldedNote` rows aren't written via the persister
  callback on the raw-seed bind path.
- **H3 — UI display bug.** Rows exist but
  `ShieldedNetworkSummaryRows.totalUnspentCredits`
  (`CoreContentView.swift:1325-1329`) filters them out incorrectly.

Diagnosis sequence:
1. Run the existing `cargo test -p platform-wallet --test shielded_sync`
   (in-process Regtest) — proves the decryption + persistence chain
   works. If green, eliminates a class of bugs and points at paloma
   or iOS-specific issues.
2. Fork the test to connect to paloma over the real network (uses the
   13-node DAPI list + the `44.238.203.84:8080` quorum URL). If green
   → paloma has the snapshot, iOS persistence/display is the bug. If
   red → paloma is the variable.
3. Storage Explorer in the iOS app — count `PersistentShieldedNote`
   rows under the bound wallet id. Zero rows = persistence; non-zero
   = display.

### P1 — performance + observability

#### P1.1 Auto-populate DAPI list from `/masternodes`

On SDK build, if Quorum URL is set: hit `{quorumURL}/masternodes`,
build `https://{ip}:1443,...` comma-separated, override
`dapi_addresses`. Drops the "you need 13 IPs to fan out" UX. If the
user has typed an explicit DAPI URL, manual entry wins.

#### P1.2 Per-chunk progress in shielded sync

Surface a progress event from
`rs-platform-wallet/src/wallet/shielded/sync.rs` every
`CHUNK_SIZE = 2048` notes processed. Wire through the FFI event vtable
into `ShieldedService` as a `@Published progress: (processed: UInt64, total: UInt64?)`.
Two side effects:
- Watermark advances per chunk → "Notes Synced" updates live during a
  cold sync, not just at pass end.
- Enables a real `ProgressView(value:total:)` instead of an
  indeterminate spinner.

Largest change in this list — touches Rust sync loop + FFI event vtable
+ Swift bridge.

#### P1.3 "N nodes connected" indicator

In the Shielded Sync Status section, render the count of live DAPI
addresses (`SDK` could expose `address_list.live_count`). Surfaces the
fan-out so the user knows whether they're funneling through one node
or distributed.

### P2 — nice to have

#### P2.1 Sync history

Append every completed pass's duration to a small ring buffer (last 5),
render as a small list under the elapsed row.

#### P2.2 Auto-test wallet B

We hardcoded A. Add a second button "Bind Test Wallet B" using
`SEED_B = [0x74; 32]` so we can measure both side by side.

## Recommended order of attack

1. **P0.2** first — figure out why balance is 0; if there's a real
   decryption bug, all timing measurements are suspect (you might be
   timing a broken path).
2. **P0.1** — cheap UI win, blocks repeat-measurement value loss.
3. **P1.1** — biggest perf gain for the test, low effort (Swift-only).
4. **P1.2** — biggest user-facing improvement but largest change; do
   once P0/P1.1 are done.
5. **P1.3 / P2.*** — nice-to-haves.

## Cleanup that ships with this branch

All raw-seed test code is tagged `TODO(shielded-snapshot-devnet-test)`.
Sites to delete when SwiftExampleApp adopts a real test-wallet import
flow (tracked: dashpay/platform#3714):

- `rs-platform-wallet-ffi/src/shielded_sync.rs`: the
  `platform_wallet_manager_bind_shielded_with_raw_seed` entry
- `swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManagerShieldedSync.swift`:
  the `bindShieldedRawSeed` wrapper
- `SwiftExampleApp/Core/Services/ShieldedService.swift`:
  `bindWithRawSeed(...)`
- `SwiftExampleApp/Core/Views/CoreContentView.swift`: the
  "Bind Test Wallet A (Shielded)" button
