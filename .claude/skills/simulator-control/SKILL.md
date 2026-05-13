---
name: simulator-control
description: Drive and inspect SwiftExampleApp on the booted iOS simulator end-to-end — tap, swipe, type, screenshot, read SwiftData, stream logs, dump the accessibility tree. Use when the user reports a UI bug, asks "why is X stuck?", wants a UAT run automated, or you need to verify the app's persisted state against what the UI shows. Covers both inspection (read-only via SwiftData + screenshots) AND control (UI automation via idb).
argument-hint: "[describe | screenshot | tap-label <label> | tap-coord <x> <y> | type <text> | back | inspect <slot>]"
---

# Simulator Control — drive + inspect SwiftExampleApp

When the user is testing SwiftExampleApp in the iOS simulator, you can do everything they could do: tap buttons by label, type text, swipe, dump the accessibility tree, screenshot the screen, read the SwiftData store directly for ground truth, cross-check chain state against testnet. Use this together with the human-in-the-loop, not against them — confirm with them before destructive actions.

## Required tooling

| Tool | Install | Why |
|---|---|---|
| `xcrun simctl` | Xcode CLT (already installed for iOS dev) | Screenshot, app container, openurl, logs |
| `sqlite3` | macOS default | Read SwiftData `default.store` |
| `idb` + `idb_companion` | `brew install facebook/fb/idb-companion` + `pipx install --python /opt/homebrew/bin/python3.12 fb-idb` (fb-idb 1.1.7 uses asyncio.get_event_loop() which Python 3.14 dropped — **must pin 3.12**) | Tap / swipe / type / accessibility tree |
| `curl` + `WebFetch` | builtin | Cross-check chain state via insight.testnet API |

Without `idb` the inspection workflows (screenshot + SwiftData + logs) still work; only control workflows are blocked.

## Quick command reference

```bash
# === Setup (once per session) ===
export PATH="$HOME/.local/bin:$PATH"
UDID=$(xcrun simctl list devices booted | awk -F'[()]' '/Booted/ {print $2}')
idb connect $UDID    # starts idb_companion alongside the running sim
BUNDLE=org.dashfoundation.SwiftExampleApp

# === INSPECT ===
xcrun simctl io booted screenshot /tmp/sim.png             # screenshot
idb ui describe-all --udid $UDID                            # accessibility tree (JSON)
idb ui describe-point --udid $UDID X Y                      # element under coord

# === CONTROL ===
idb ui tap     --udid $UDID X Y                             # tap at coord
idb ui swipe   --udid $UDID X1 Y1 X2 Y2 --duration 0.3      # swipe
idb ui text    --udid $UDID "hello"                         # type text into focused field
idb ui key     --udid $UDID 40                              # short press keycode (40 = return)
idb ui button  --udid $UDID HOME                            # hardware buttons: HOME, LOCK, SIRI, SIDE_BUTTON

# === APP ===
xcrun simctl launch    booted $BUNDLE
xcrun simctl terminate booted $BUNDLE
xcrun simctl openurl   booted "dashplatform://identity/abc123"

# === DATA ===
DATA=$(xcrun simctl get_app_container booted $BUNDLE data)
STORE="$DATA/Library/Application Support/default.store"
sqlite3 "$STORE" "SELECT ..."

# === LOGS ===
xcrun simctl spawn booted log show --last 60s --info \
    --predicate 'processImagePath CONTAINS "SwiftExampleApp"'
```

## The label-find-then-tap pattern (killer feature)

Don't hardcode pixel coords. Dump the accessibility tree, filter by `AXLabel` (exact or substring), tap the frame center. Works across iPhone models / orientations / SwiftUI layout tweaks.

```bash
tap_label() {
    local label="$1"
    local udid=$(xcrun simctl list devices booted | awk -F'[()]' '/Booted/ {print $2}')
    LABEL="$label" UDID="$udid" "$HOME/.local/bin/idb" ui describe-all --udid "$udid" 2>&1 \
        | python3 -c "
import json, os, subprocess, sys
items = json.loads(sys.stdin.read())
label = os.environ['LABEL']
# Exact match first, fall back to substring
match = next((it for it in items if it.get('AXLabel') == label and it.get('enabled')), None)
if not match:
    match = next((it for it in items if label in (it.get('AXLabel') or '') and it.get('enabled')), None)
if not match:
    print(f'no enabled element matching {label!r}', file=sys.stderr); sys.exit(1)
f = match['frame']
x, y = int(f['x'] + f['width']/2), int(f['y'] + f['height']/2)
subprocess.run([os.path.expanduser('~/.local/bin/idb'), 'ui', 'tap', '--udid', os.environ['UDID'], str(x), str(y)], check=True)
print(f'tapped {match.get(\"AXLabel\")!r} ({match.get(\"type\")}) at ({x},{y})')
"
}

tap_label "Resume"
```

Use `AXUniqueId` instead of `AXLabel` when the UI sets one (more stable across localization). The back-navigation button in this app, for example, has `AXUniqueId: "BackButton"`.

## SwiftData schema cheat sheet

The app's `default.store` is a Core Data SQLite database. Tables are prefixed `ZPERSISTENT*` and columns `Z*`. Get the full list via `sqlite3 "$STORE" ".tables"`. Most relevant:

| Table | Key columns | Purpose |
|---|---|---|
| `ZPERSISTENTASSETLOCK` | `ZSTATUSRAW`, `ZIDENTITYINDEXRAW`, `ZOUTPOINTHEX`, `ZPROOFBYTES`, `ZWALLETID`, `ZAMOUNTDUFFS` | Tracked asset locks |
| `ZPERSISTENTIDENTITY` | `ZIDENTITYINDEX`, `ZIDENTITYID`, `ZNETWORKRAW`, `ZWALLET` | Registered platform identities |
| `ZPERSISTENTWALLET` | `ZWALLETID`, `ZLABEL`, `ZNETWORKRAW` | Local wallets |
| `ZPERSISTENTACCOUNT` | `ZACCOUNTTYPE`, `ZWALLET` | Per-wallet accounts |
| `ZPERSISTENTTXO` | `ZWALLETID`, `ZTRANSACTION`, `ZSPENDINGTRANSACTION` | UTXOs, source of `TransactionListView` |
| `ZPERSISTENTTRANSACTION` | `ZTXID`, `ZCONTEXT`, `ZFIRSTSEEN`, `ZBLOCKHEIGHT` | TXs |
| `ZPERSISTENTDOCUMENT` | `ZDOCUMENTID`, `ZDATACONTRACT` | Documents |

Discriminants (mirror Rust enums):
- `ZSTATUSRAW` on asset lock: `0`=Built, `1`=Broadcast, `2`=InstantSendLocked, `3`=ChainLocked
- `ZCONTEXT` on transaction: `0`=mempool, `1`=instantSend, `2`=inBlock, `3`=inChainLockedBlock

`Z_PK` columns are integer foreign keys to the related table's primary key — stable for the install lifetime but NOT across re-installs. Quote `ZIDENTITYID` / `ZOUTPOINTHEX` / `ZWALLETID` blobs in any long-lived reference.

## Common workflows

### A — Verify a "stuck" asset lock (the SPV-catch-up-gap diagnostic)

```bash
sqlite3 "$STORE" -header -column "
SELECT ZIDENTITYINDEXRAW AS slot, ZSTATUSRAW AS status,
       ZAMOUNTDUFFS AS duffs, length(ZPROOFBYTES) AS proof_len,
       length(ZTRANSACTIONBYTES) AS tx_len, ZOUTPOINTHEX
  FROM ZPERSISTENTASSETLOCK
 WHERE ZIDENTITYINDEXRAW = 10;"
```

Then cross-check chain state — strip the `:vout` suffix to get the txid:
```bash
TXID=$(sqlite3 "$STORE" "SELECT substr(ZOUTPOINTHEX, 1, 64) FROM ZPERSISTENTASSETLOCK WHERE ZIDENTITYINDEXRAW = 10;")
curl -s "https://insight.testnet.networks.dash.org/insight-api/tx/$TXID" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'block={d.get(\"blockheight\")} conf={d.get(\"confirmations\")} txlock={d.get(\"txlock\")}')"
```

| SwiftData | On chain | Diagnosis |
|---|---|---|
| status 1, no proof | mined + txlock | **SPV catch-up gap** — signatures exist but the wallet hasn't backfilled them on app load |
| status 1, no proof | mined, no txlock | Pure timing — waiting for masternodes |
| status 1, no proof | not found / not mined | TX dropped or never confirmed |
| status 2/3, proof present | anything | UI should already be showing Resume |
| status 2/3, proof present, UI shows "Waiting…" | anything | **UI reactivity bug** — `@Query` not picking up update |

### B — Drive a full UAT scenario end-to-end (example: crash-recovery)

```bash
# 1. Snapshot SwiftData state
sqlite3 "$STORE" "SELECT ZIDENTITYINDEXRAW, ZSTATUSRAW FROM ZPERSISTENTASSETLOCK ORDER BY ZIDENTITYINDEXRAW;"

# 2. Force-quit + relaunch (simulates a crash)
xcrun simctl terminate booted $BUNDLE
xcrun simctl launch    booted $BUNDLE
sleep 3

# 3. Take the Identities tab → Resumable Registrations → Resume row
xcrun simctl io booted screenshot /tmp/after-launch.png
tap_label "Identities"
sleep 1
tap_label "Resume"      # taps the first Resume button in the visible accessibility tree

# 4. Verify the resume submit fires — read SwiftData a few times until ZSTATUSRAW goes 1->2->identity row appears
```

### C — Tap an arbitrary row by stable substring (e.g. an outpoint prefix)

The accessibility tree exposes truncated UI strings AND full underlying labels for `Text(verbatim:)` content. Use a substring match to find a row whose visible txid prefix is known:

```bash
LABEL="780ea9931" tap_label "$LABEL"
```

### D — Find element under a screen point (debug a layout)

```bash
idb ui describe-point --udid $UDID 200 400
```

Returns the element at that coordinate — useful when an interactive area isn't where the visible layout suggests (e.g. SwiftUI Form hit-test boundaries on iOS 26).

### E — Type into a focused TextField

```bash
# Tap the field first to focus it, then type.
tap_label "Amount"
idb ui text --udid $UDID "0.0025"
idb ui key  --udid $UDID 40   # return
```

iOS keycodes: `40`=return, `42`=backspace, `43`=tab, `44`=space, see Apple's `HIDKeyboardKey` table.

### F — Hardware buttons + system actions

```bash
idb ui button --udid $UDID HOME             # go to springboard
idb ui button --udid $UDID LOCK             # lock screen
idb ui button --udid $UDID SIDE_BUTTON      # side button
idb ui button --udid $UDID SIRI             # invoke Siri
```

### G — Screenshot-diff to verify a state change

```bash
xcrun simctl io booted screenshot /tmp/before.png
tap_label "Resume"
sleep 1
xcrun simctl io booted screenshot /tmp/after.png
# Compare with magick or by reading both images into Claude.
```

### H — Log capture during an action

```bash
xcrun simctl spawn booted log stream --info \
    --predicate 'processImagePath CONTAINS "SwiftExampleApp"' > /tmp/applog.txt 2>&1 &
LOG_PID=$!
# ... drive UI via idb / let the user act ...
sleep 5
kill $LOG_PID
grep -iE "error|panic|fatal|💥|⚠️" /tmp/applog.txt
```

### I — Poll-and-wait for a state transition

When you've kicked off an async operation and want to wait for the UI/SwiftData to confirm it:
```bash
for i in {1..30}; do
    status=$(sqlite3 "$STORE" "SELECT ZSTATUSRAW FROM ZPERSISTENTASSETLOCK WHERE ZIDENTITYINDEXRAW = 10;")
    echo "[$i] status=$status"
    [ "$status" -ge 2 ] && break
    sleep 2
done
```

## Setup checklist

Run before any session that needs UI control:

```bash
export PATH="$HOME/.local/bin:$PATH"
which idb || { echo "install: brew install facebook/fb/idb-companion && pipx install --python /opt/homebrew/bin/python3.12 fb-idb"; exit 1; }
UDID=$(xcrun simctl list devices booted | awk -F'[()]' '/Booted/ {print $2}')
[ -z "$UDID" ] && { echo "no booted sim — boot one in Xcode or via 'xcrun simctl boot <udid>'"; exit 1; }
idb connect $UDID 2>&1 | grep -q "udid:" || { echo "idb companion not reachable"; exit 1; }
echo "ready: UDID=$UDID"
```

If `idb connect` hangs, clear stale companion processes: `pkill -f idb_companion` then re-run.

## Pitfalls

- **Data container path changes per install.** Always use `xcrun simctl get_app_container` to locate the SwiftData store — never hardcode the UUID.
- **fb-idb 1.1.7 + Python 3.14 = broken.** Pin to Python 3.12 via `pipx install --python /opt/homebrew/bin/python3.12 fb-idb`. The error is `RuntimeError: There is no current event loop in thread 'MainThread'.`
- **`getpwuid_r did not find a match`** stderr noise from `xcrun simctl spawn booted log ...` is harmless; logs still stream.
- **Multiple booted simulators** — pass `--udid` explicitly. `xcrun simctl list devices booted` may pick a different one than the user expects.
- **`describe-all` returns disabled / non-interactive elements too** — filter on `enabled == true` and `role in {AXButton, AXTextField, AXLink}` for actions, or `type == "Cell"` for list rows.
- **`AXLabel` is not unique** — multiple "Confirmed" badges, multiple chevrons. When ambiguous, narrow by `frame.y` range or by walking the tree near a known parent. Prefer `AXUniqueId` when set.
- **Tap coordinates are in points, not pixels** — `describe-all`'s frames match `xcrun simctl io screenshot` coords directly (no scaling).
- **Sheet / modal presentations** can change the accessibility tree drastically — always `describe-all` again after a navigation, don't cache element coords across screens.
- **Status-bar override for clean screenshots**: `xcrun simctl status_bar booted override --time "9:41"` and `xcrun simctl status_bar booted clear` to reset.
- **Don't tap on a screen the user is mid-interaction with** unless you've confirmed it's safe — they may lose form state. Snapshot first, ask second on anything destructive.
- **Cross-process SwiftData writes are unsafe** while the app holds the SQLite connection — readonly queries only.

## What this skill does NOT do

- **Mock the network / chain state.** For that you need testnet faucets, regtest, or fixture-based tests at the Rust layer.
- **Simulate IS-lock / chain-lock signatures.** Those come from the masternode network. To test those code paths deterministically you need test fixtures injected at the Rust persister layer.
- **Cross-process writes** to SwiftData while the app is running.

## Worked example — iter 5 stuck-resume diagnosis (2026-05-13)

User reported: identity slot #10 stuck on "Waiting for InstantSendLock…" forever. End-to-end workflow used:

1. **Screenshot** → confirmed UI shows "Broadcast", proofBytes "—".
2. **SwiftData query** → got the **full** outpoint `780ea9931eae9d4e6a0df2c0c2721c11bd645fc453fb2907b4a4894893a257d0:0` (the UI truncates to `780ea99…257d0:0`, useless for chain lookups).
3. **WebFetch insight.testnet** with the full txid → block 1475917, 67 confirmations, **`txlock: true`** → diagnostic table row 1: **SPV catch-up gap**.
4. **`idb ui describe-all`** → found `BackButton` at frame `{{16, 62}, {44, 44}}` and the slot-row by its full-txid `AXLabel`.
5. **`idb ui tap 38 84`** → navigated back to the list, screenshot revealed all 9 asset locks: only slot #10 was stuck on Broadcast; slots #2-#9 are all InstantSendLocked. Confirms the bug is specific to outpoints that were already confirmed before this app session was first launched.
6. **Label-find-then-tap** with substring `"780ea9931"` → restored user's screen to slot #10 detail.

Total time: ~5 minutes of automated control + verification. No coordinate guessing, no screenshot squinting, ground truth from SwiftData + chain.

## Future enhancements

- Wrap `tap_label` in a checked-in script at `.claude/skills/simulator-control/scripts/tap-label.py`.
- Add `wait-for-label "<label>" --timeout 30` that polls `describe-all` until the label appears (or disappears) — useful for SPV-delivered status transitions during UAT.
- Macro `run-uat-scenario <name>` driving the full iter 5 UAT matrix once seed-data fixtures are in place.
