---
name: emulator-control
description: Drive and inspect KotlinExampleApp on a booted Android emulator end-to-end — tap/swipe/type by Compose testTag, screenshot, dump the view tree, unlock for auth-bound signing, fund via faucet or self-send, run full identity/DPNS/DashPay flows against testnet. The Android counterpart of `simulator-control` (iOS). Use when driving KotlinExampleApp, reproducing a UI bug, or verifying an SDK change end-to-end on the emulator.
argument-hint: "[describe | screenshot | tap-tag <testTag> | tap-text <text> | type <text> | back | unlock | fund <addr>]"
---

# Emulator Control — drive + inspect KotlinExampleApp

The Android sibling of `simulator-control`. When testing KotlinExampleApp on an
Android emulator, you can do everything the user could: tap by Compose testTag,
type, swipe, screenshot, dump the view tree, unlock the device for auth-bound
signing, fund a wallet, and run identity/DPNS/DashPay flows against live
testnet. Use it alongside the human — confirm before spending funds or
broadcasting.

## Setup (once per session)

```bash
export ANDROID_HOME=/opt/homebrew/share/android-commandlinetools
export PATH=$ANDROID_HOME/platform-tools:$PATH
export JAVA_HOME=/opt/homebrew/opt/openjdk@17        # for any gradle work
adb devices                                          # find the running emulator
D=emulator-5554                                      # this session's serial
adb -s $D shell getprop ro.product.cpu.abi           # MUST be arm64-v8a on Apple Silicon
PKG=org.dashfoundation.example                       # KotlinExampleApp
adb -s $D shell monkey -p $PKG -c android.intent.category.LAUNCHER 1   # launch
```

AVDs on this Mac: `kotlin_sdk_ci` (arm64-v8a — use this) and `kotlin_sdk_ci_x86`
(x86_64 — will NOT run the arm64-only native lib). API 35.

## The killer feature: tap by Compose testTag

`MainActivity` sets `Modifier.semantics { testTagsAsResourceId = true }`, so every
Compose `testTag` shows up as `resource-id` in a `uiautomator dump`. That makes
the app fully scriptable by stable IDs — never hardcode pixel coords. The whole
driver is this helper (write it to the scratchpad once per session):

```bash
# ui.sh — dump the tree, find a node by resource-id (testTag) or text, tap center.
D=emulator-5554; X=/tmp/ui.xml
dump(){ adb -s $D shell uiautomator dump /sdcard/ui.xml >/dev/null 2>&1; adb -s $D pull /sdcard/ui.xml $X >/dev/null 2>&1; }
center_of(){ python3 - "$X" "$1" <<'PY'
import sys,re,xml.etree.ElementTree as ET
root=ET.parse(sys.argv[1]).getroot()
for n in root.iter('node'):
    if n.get('resource-id','')==sys.argv[2]:
        m=re.findall(r'\[(\d+),(\d+)\]\[(\d+),(\d+)\]',n.get('bounds',''))
        if m: x1,y1,x2,y2=map(int,m[0]); print((x1+x2)//2,(y1+y2)//2); break
PY
}
tap_id(){ dump; read x y < <(center_of "$1"); [ -n "$x" ] && adb -s $D shell input tap $x $y || echo "NOT FOUND: $1"; }
shot(){ adb -s $D exec-out screencap -p > "/tmp/${1:-shot}.png"; }
```

`center_of` also works on `text=`/`content-desc=` with a one-line tweak. To read
the whole screen, dump then print every node with a non-empty resource-id / text
/ content-desc. Screenshots (`shot`) are worth reading into context whenever the
tree is ambiguous.

## CRITICAL: unlock the device before any signing / Keystore decrypt

Both Keystore aliases are `setUnlockedDeviceRequired(true)` and `KEYS_ALIAS` is
also `setUserAuthenticationRequired(true)`. **A locked emulator makes every
Keystore decrypt fail** with `java.security.InvalidKeyException: Keystore
operation failed` — this breaks wallet-unlock, roundtrip, and DashPay
instrumented tests, and is NOT a code bug. CI enrolls PIN `1234`
(`.github/workflows/kotlin-sdk-build.yml`) and unlocks before the suite.

```bash
# Check: deviceLocked=1 / strongAuthRequired=0x1 means locked.
adb -s $D shell "dumpsys trust | grep -i deviceLocked"
# Unlock with PIN 1234:
adb -s $D shell input keyevent KEYCODE_WAKEUP
adb -s $D shell input swipe 540 1600 540 600      # reveal PIN pad
adb -s $D shell input text 1234
adb -s $D shell input keyevent KEYCODE_ENTER      # deviceLocked -> 0, strongAuthRequired -> 0x0
```

**Signing prompts a device-credential dialog** titled **"Authorize signing"**
(the app's `BiometricGate`, reached via `KeystoreSigner.retrieveKeyWithAuth`).
Its secure input does NOT reliably accept `input text` — use digit keyevents:

```bash
adb -s $D shell input tap 540 1400                       # focus the PIN field
adb -s $D shell input keyevent KEYCODE_1 KEYCODE_2 KEYCODE_3 KEYCODE_4
adb -s $D shell input keyevent KEYCODE_ENTER
```

keystore2 logcat confirms success: `on_device_unlocked(password.is_some()=true)`
→ `add_auth_token(authType=0x1)` → `BiometricService AuthSession Dismissed`.

## Funding a wallet on testnet

1. **In-app faucet (preferred):** Wallet → Receive (Core tab) → scroll to
   `receive.faucetButton` ("Get 1 tDASH — Testnet Faucet"). It solves the
   `faucet.thepasta.org` cap.js PoW internally.
2. **Faucet fallback:** on rate-limit/failure the app opens the web faucet in
   Chrome (brittle: first-run wall + manual PoW). Don't fight it — **self-fund
   instead**: from any already-funded wallet, Send (`walletDetail.sendButton` →
   `send.recipientField` + `send.amountField` → `send.submitButton`) a small
   amount to the new wallet's Core address. Arrives via InstantSend, usable for
   an asset lock before block confirmation.
3. **Watch L1 arrival** (independent of the app):
   `curl -s https://insight.testnet.networks.dash.org/insight-api/addr/<ADDR>`
   → `balanceSat` / `unconfirmedBalanceSat` (duffs).

## Key testTags by screen

- Nav: `rootTab.{sync,wallets,identities,dashpay,settings}`
- Wallet list: `wallets.add`, `wallets.walletRow.<walletIdHex>`
- Create wallet: `createWallet.{name,importToggle,submit}`; seed backup
  `seedBackup.{wroteItDownToggle,continueButton,quizWord.<word>,createWalletButton}`
- Wallet detail: `walletDetail.{sendButton,receiveButton,platformBalanceMenu}`
- Receive: `receive.{tab.<label>,address,copyButton,shareButton,faucetButton}`
- Send: `send.{recipientField,amountField,addRecipient,submitButton}`
- Identities: `identities.addMenu` → menu items "Create Identity" / "Load
  Identity" / "Search Wallets for Identities" / "State Transitions";
  `identities.row.<idHex>`
- Create identity: `createIdentity.{sourceWalletPicker,fundingSourcePicker,amount,identityIndex,submit}`;
  progress `registrationProgress.step.{PreparingKeys,Building,Broadcasting,Confirming,Registering}` + `registrationProgress.completed`
- Identity detail: `identityDetail.{idHex,topUpFromCore,topUp,transfer,transferToAddress,withdraw,registerName,selectMainName,dashpay,viewKeys,refresh}`
- DPNS: `registerName.{label,availability,contestStatus,viewContest,submit,success}`
- Keys: `keysList.row.<keyId>`, `keysList.addKey`, `keyDetail.{publicKey,reveal,disable}`

## Worked flow — full identity + DPNS e2e (store + sign paths)

```
Create wallet (createWallet.* → seed quiz → createWalletButton)
  → Receive → fund (faucet or self-send)  → wait for Core balance
  → Identities → identities.addMenu → Create Identity
       sourceWalletPicker = new wallet, fundingSourcePicker = Core balance,
       amount ≤ funded duffs (default 50000000 may exceed it), submit
  → progress runs PreparingKeys (stores identity keys under KEYS_ALIAS)
       … Confirming (asset lock) … Registering → "Authorize signing" (PIN)
       → registrationProgress.completed = "Identity created"
  → View Identity → identityDetail.registerName
       registerName.label = a UNIQUE, NON-contested name → submit → PIN
       → registerName.success = "Registered <name>.dash"
  → identityDetail.refresh → name appears under DPNS NAMES (on-chain confirm)
```

DPNS naming: a label is **Regular** (instant) vs **contested** (masternode
vote). Non-contested = longer than 19 chars OR contains digits — check
`registerName.contestStatus` before submitting. The success/`DPNS NAMES` string
shows the DPNS-normalized homoglyph form (`o→0`, `i/l→1`), e.g.
`kotlin-e2e-7023` → `k0t11n-e2e-7023`; the registered label is the original.

## Pitfalls

- **Soft keyboard covers submit buttons.** Tapping a "submit" coordinate while
  the keyboard is up hits a keyboard key instead (amount fields silently gain
  digits). Hide it (`KEYCODE_BACK` / `KEYCODE_ESCAPE`) and scroll the form up so
  the real button is on-screen, then re-dump for its live bounds before tapping.
- **Re-flowing lists** (the seed-verification chips) shift after each tap —
  re-dump before every tap in such lists, don't reuse cached coordinates.
- **Nav tabs remember their back-stack.** Tapping `rootTab.identities` may land
  deep on a Key Detail; press `KEYCODE_BACK` to pop to the list/detail root.
- **arm64 only.** The native lib is arm64-v8a; the `_x86` AVD can't load it.
- **Two sessions, distinct state.** When another session runs the iOS simulator,
  the Android emulator is already separate storage — but still use a fresh
  wallet and uniquely-named DPNS labels/identities to avoid confusion.

## Build for the emulator

```bash
cd packages/kotlin-sdk
JAVA_HOME=/opt/homebrew/opt/openjdk@17 ANDROID_HOME=/opt/homebrew/share/android-commandlinetools \
  ./gradlew :sdk:assembleDebug :app:assembleDebug        # in-place (repo is APFS; no sparse-image redirect needed)
./gradlew :sdk:connectedDebugAndroidTest                 # REQUIRES an unlocked device (see above)
```

Filter instrumented runs with
`-Pandroid.testInstrumentationRunnerArguments.class=<FQCN>[#method][,…]`;
opt into live-testnet tests with `-Ptestnet=true`.

## What this skill does NOT do

- Trigger `KeyPermanentlyInvalidatedException` (permanent Keystore invalidation)
  — that needs a real credential reset / biometric re-enrollment and is not
  reliably scriptable on an emulator; it stays a device-bound manual gate.
- Read app persistence directly the way `simulator-control` reads SwiftData —
  Kotlin state is Room (DataStore + SQLite in the app's data dir); inspect it
  via the UI or `adb shell run-as $PKG` if ground truth is needed.
