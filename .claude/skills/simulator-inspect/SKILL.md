---
name: simulator-inspect
description: Inspect SwiftExampleApp state directly from the booted iOS simulator — screenshot, read SwiftData, capture logs, look up app data. Use whenever the user reports a UI bug, asks "why is X stuck?", or you need to verify the app's actual persisted state vs. what the user sees. Read-only: this skill does NOT tap, swipe, or type — the user drives the UI, you verify it.
argument-hint: "[lock-slot | identity-index | wallet | full]"
---

# Simulator Inspect — SwiftExampleApp ground-truth verification

When a user reports a UI symptom in SwiftExampleApp ("the row is stuck on Waiting…", "my balance didn't update", "the registration spinner never finishes"), the fastest way to a real diagnosis is reading the app's **actual SwiftData state** rather than relying on screenshots alone. The app's UI is a `@Query`-driven projection of the SQLite database backing `default.store`; if a row says `statusRaw = 1`, the UI shows "Broadcast" — full stop, no caching weirdness.

This skill exposes the toolkit Claude can use without any extra installs (`idb`, Appium, etc. are NOT required). All commands are read-only or screenshot-only.

## When to use

- User reports an asset lock is stuck mid-flight
- User asks to verify what's in SwiftData for an identity / wallet / lock
- You're hypothesizing about UI vs. state divergence and need ground truth
- You want a screenshot of the current screen the user is on
- You need the full txid / outpoint / id of something the UI is truncating
- You're testing a fix and want to confirm the persister wrote what you expect

## When NOT to use

- You need to tap a button, swipe, or type text. **You can't from here.** Ask the user.
- You need to drive a full UAT cycle autonomously. Use a UI-automation tool like `idb` instead (requires install) or have the user execute the matrix.

## Quick reference

All commands resolve the booted device automatically. The app bundle id is `org.dashfoundation.SwiftExampleApp` (the SwiftExampleApp in `packages/swift-sdk/SwiftExampleApp`).

```bash
# 1. Screenshot the current sim screen
xcrun simctl io booted screenshot /tmp/sim.png

# 2. Find the SwiftData store path (data container changes per build)
DATA_DIR=$(xcrun simctl get_app_container booted org.dashfoundation.SwiftExampleApp data)
STORE="$DATA_DIR/Library/Application Support/default.store"

# 3. Read SwiftData
sqlite3 "$STORE" -header -column "SELECT ..."

# 4. Stream logs (warn: 'getpwuid_r' noise on stderr is harmless)
xcrun simctl spawn booted log show --last 60s --info \
    --predicate 'processImagePath CONTAINS "SwiftExampleApp"'
```

## Schema cheat sheet (read-only)

SwiftData persists everything under `default.store` with `Z`-prefixed Core Data column names. The full table list comes from `sqlite3 "$STORE" ".tables"`. The most common ones for this app:

| Table | Key columns | Purpose |
|---|---|---|
| `ZPERSISTENTASSETLOCK` | `ZSTATUSRAW`, `ZIDENTITYINDEXRAW`, `ZOUTPOINTHEX`, `ZPROOFBYTES`, `ZWALLETID` | Tracked asset locks for identity funding |
| `ZPERSISTENTIDENTITY` | `ZIDENTITYINDEX`, `ZIDENTITYID`, `ZNETWORKRAW`, `ZWALLET` | Registered platform identities |
| `ZPERSISTENTWALLET` | `ZWALLETID`, `ZLABEL`, `ZNETWORKRAW` | Local wallets |
| `ZPERSISTENTACCOUNT` | `ZACCOUNTTYPE`, `ZWALLET` | Per-wallet accounts (BIP44 / Platform Payment etc.) |
| `ZPERSISTENTTXO` | `ZWALLETID`, `ZTRANSACTION`, `ZSPENDINGTRANSACTION` | UTXOs, source of `TransactionListView` |
| `ZPERSISTENTTRANSACTION` | `ZTXID`, `ZCONTEXT`, `ZFIRSTSEEN`, `ZBLOCKHEIGHT` | Confirmed/mempool TXs |
| `ZPERSISTENTPUBLICKEY` | `ZKEYINDEX`, `ZIDENTITY` | Identity pubkeys |
| `ZPERSISTENTDOCUMENT` | `ZDOCUMENTID`, `ZDATACONTRACT` | Persisted documents |

`ZSTATUSRAW` values on asset lock: `0`=Built, `1`=Broadcast, `2`=InstantSendLocked, `3`=ChainLocked. (Mirror of Rust `AssetLockStatus`.)

`ZCONTEXT` on transaction: `0`=mempool, `1`=instantSend, `2`=inBlock, `3`=inChainLockedBlock.

## Common workflows

### Workflow A — Verify an asset lock the user says is "stuck"

```bash
DATA_DIR=$(xcrun simctl get_app_container booted org.dashfoundation.SwiftExampleApp data)
STORE="$DATA_DIR/Library/Application Support/default.store"

# Replace 10 with the slot the user mentioned (visible as "Identity Index" in the UI).
sqlite3 "$STORE" -header -column "
SELECT ZIDENTITYINDEXRAW AS slot,
       ZSTATUSRAW       AS status,
       ZAMOUNTDUFFS     AS duffs,
       length(ZPROOFBYTES) AS proof_len,
       length(ZTRANSACTIONBYTES) AS tx_len,
       ZOUTPOINTHEX
  FROM ZPERSISTENTASSETLOCK
 WHERE ZIDENTITYINDEXRAW = 10;"
```

Interpretation:

| `status` | `proof_len` | Meaning |
|---|---|---|
| 1 | NULL/empty | Broadcast, waiting for IS lock — normal if recent, suspicious if old (event-vs-poll gap) |
| 2 or 3 | non-null | Resumable. UI should show a Resume button. |
| 1+ | non-null | **Inconsistency** — write ordering bug. |
| 0 | anything | Built but never broadcast — tight crash window. |

The full `ZOUTPOINTHEX` value (which the UI truncates to a `780ea99…257d0:0` prefix) is the round-trip-able outpoint you can paste into a testnet explorer. Strip the `:VOUT` suffix to get the txid.

### Workflow B — Cross-check against testnet chain state

```bash
# Use the txid (first 64 hex chars of ZOUTPOINTHEX, before the ':') against insight
TXID=780ea9931eae9d4e6a0df2c0c2721c11bd645fc453fb2907b4a4894893a257d0
curl -s "https://insight.testnet.networks.dash.org/insight-api/tx/$TXID" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); \
                  print(f'block: {d.get(\"blockheight\")}, confirmations: {d.get(\"confirmations\")}, txlock: {d.get(\"txlock\")}')"
```

Or use `WebFetch` against the same URL with a prompt asking for confirmations / `txlock` / `chainlocked`.

Diagnostic table for `(SwiftData state, chain state)` → root cause:

| SwiftData says | On chain | Diagnosis |
|---|---|---|
| status 1, no proof | mined + chainlocked | **SPV catch-up gap** — signatures exist but our wallet hasn't backfilled them. Needs Rust-side refresh helper. |
| status 1, no proof | mined, no chainlock | Pure timing — waiting for masternodes. Not a bug if recent. |
| status 1, no proof | not found / not mined | Broadcast never confirmed. TX may have been dropped. |
| status 2/3, proof present, but UI shows "Waiting…" | anything | **UI reactivity bug** — `@Query` not picking up the row. |
| status 2/3, proof present | TX not chainlocked yet | Fine — IS-lock proof is sufficient for Platform submit. |

### Workflow C — Find which identity/document/contract a SwiftData row belongs to

The Z-prefixed schema is Core Data conventions; relationship columns are integer foreign keys to the related table's `Z_PK`. Join shape:

```bash
sqlite3 "$STORE" "
SELECT i.ZIDENTITYINDEX, hex(i.ZIDENTITYID), w.ZLABEL
  FROM ZPERSISTENTIDENTITY i
  LEFT JOIN ZPERSISTENTWALLET w ON i.ZWALLET = w.Z_PK
  WHERE i.ZIDENTITYINDEX = 10;"
```

### Workflow D — Get a screenshot for visual confirmation

```bash
xcrun simctl io booted screenshot /tmp/sim.png
# Then use the Read tool on /tmp/sim.png — Claude can view the image inline.
```

Pair with `xcrun simctl status_bar booted override --time "9:41"` first if you want the canonical clean-status-bar Apple marketing-shot look. Reset with `xcrun simctl status_bar booted clear`.

### Workflow E — Stream logs while the user does an action

```bash
# Foreground capture for ~30s (run in background, then read the file)
xcrun simctl spawn booted log show --last 30s --info \
    --predicate 'processImagePath CONTAINS "SwiftExampleApp"' > /tmp/applog.txt 2>&1
```

For long-running streams, use `log stream` instead and redirect to a file; let the user perform the action; then `cat` the file. Note that the example app uses plain `print` / `os_log` mostly without a dedicated subsystem identifier, so `processImagePath` is a more reliable filter than `subsystem`.

### Workflow F — Locate the booted UDID / app bundle

```bash
# UDID of the booted device (one of them, if multiple)
xcrun simctl list devices booted

# All installed apps with bundle ids
xcrun simctl listapps booted | grep -E "CFBundleIdentifier|CFBundleName"

# Just our app
xcrun simctl listapps booted | grep -B1 -A6 SwiftExample
```

### Workflow G — Reproduce a deep-link path

If the app exposes a custom URL scheme:
```bash
xcrun simctl openurl booted "dashplatform://identity/abc123"
```

(SwiftExampleApp doesn't currently register a URL scheme, but this is the path if one is added — useful for jumping directly to a specific screen during testing.)

## Pitfalls

- **Data container changes per install.** Always look it up via `get_app_container` — don't hardcode the UUID in any path.
- **`default.store-wal` and `-shm` files** are the SQLite write-ahead log and shared memory; don't move/delete them while the app is running, or you'll corrupt the journal. Reading the `.store` directly is fine (SQLite handles it).
- **Multiple booted simulators** — `booted` picks one. If the user has multiple, ask which device or pass the UDID explicitly.
- **`getpwuid_r did not find a match for uid 502`** on log commands is a harmless stderr warning; the logs still stream.
- **Status changes during read** are not atomic vs. the SwiftData write — if you read mid-flight you may see status=1 one query and status=2 the next. For diagnostics that's fine; if it bothers you, take two reads 1s apart.
- **Z_PK foreign keys are NOT stable across re-installs** — they're integer primary keys. Don't quote them in long-lived issue reports; quote `ZIDENTITYID` / `ZOUTPOINTHEX` / `ZWALLETID` blobs instead.

## What this skill does NOT do

- **Tap, swipe, scroll, type.** Use the user, or install `idb` (`brew install facebook/fb/idb-companion && pip install fb-idb`) and document the dependency.
- **Modify state.** Writing to the SwiftData store from outside the app while the app is running is unsafe. To force a state change, use the app's UI or restart the app under controlled conditions.
- **Mock the network / chain state.** For that you need testnet faucets, regtest, or fixture-based tests at the Rust layer.

## Worked example — the iter 5 stuck-resume diagnosis (2026-05-13)

User reported: identity slot #10 stuck on "Waiting for InstantSendLock…" forever. We used:

1. **Workflow A**: read `ZPERSISTENTASSETLOCK` for slot 10 → status=1, proof_len=NULL, tx_len=240 bytes, full outpoint `780ea9931eae9d4e6a0df2c0c2721c11bd645fc453fb2907b4a4894893a257d0:0`.
2. **Workflow B**: WebFetch to insight.testnet → block 1475917, 67 confirmations, `txlock: true`.
3. **Diagnosis from the table**: row 1 — SPV catch-up gap. IS-lock signature exists on chain but our wallet didn't backfill it on app load.
4. **Root-cause fix scoped**: add `IdentityWallet::refresh_asset_lock_proof_state(outpoint)` in `rs-platform-wallet` that polls SPV's local store, call it at wallet load + inside `wait_for_proof` on entry.

That whole diagnosis took two `sqlite3` queries + one `WebFetch`. No taps required, no screenshots beyond the initial "user shared screen", no guessing.
