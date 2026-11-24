# Dash Kotlin SDK (Android)

Android SDK and example app for Dash Platform — the Kotlin counterpart of
[`packages/swift-sdk`](../swift-sdk). It combines:

- **Core SDK** (SPV wallet): key derivation, accounts, UTXOs, SPV sync
- **Platform SDK**: identities, DPNS names, data contracts, documents,
  tokens, DashPay
- **KotlinExampleApp**: a Jetpack Compose app demonstrating the full surface,
  a one-for-one port of SwiftExampleApp

## Layout

```
kotlin-sdk/
├── sdk/                 # Android library (org.dashfoundation.dashsdk)
├── KotlinExampleApp/    # Compose example app (org.dashfoundation.example)
├── build_android.sh     # builds libdash_sdk_jni.so via cargo-ndk
└── gradle…              # Gradle 8.14 / AGP 8.9 / Kotlin 2.1
```

The native layer is [`packages/rs-unified-sdk-jni`](../rs-unified-sdk-jni), a Rust JNI shim
over `rs-sdk-ffi` and `platform-wallet-ffi`. There is no C
glue and no generated headers on Android — the shim calls the FFI crates as
ordinary Rust dependencies and exposes JNI symbols directly.

## Prerequisites

- JDK 17+
- Android SDK with platform 35 + NDK r28+ (`sdkmanager "ndk;28.1.13356709"`)
- Rust (workspace toolchain) + `cargo install cargo-ndk`
- Rust targets: `rustup target add aarch64-linux-android x86_64-linux-android`

## Build

```bash
# 1. Native library → sdk/src/main/jniLibs/{arm64-v8a,x86_64}/
./build_android.sh --verify

# 2. SDK + example app
./gradlew :sdk:assembleDebug :app:assembleDebug

# 3. Tests
./gradlew :sdk:testDebugUnitTest                 # JVM/Robolectric
./gradlew :sdk:connectedDebugAndroidTest         # instrumented (emulator)
```

Supported ABIs: `arm64-v8a` (devices) and `x86_64` (emulator). minSdk 29,
targetSdk 35. The `shielded` (Orchard) feature is enabled by default; build
with `./build_android.sh --no-shielded` for a slim transparent-only library.

## Architecture

See [CLAUDE.md](CLAUDE.md) for the layering rules (persist / load / bridge —
all orchestration lives in the Rust `platform-wallet` crate) and
[../swift-sdk/CLAUDE.md](../swift-sdk/CLAUDE.md) for the reference doctrine.

## Shielded tips

A shielded tip pays a DashPay username at the shielded address its profile
publishes. Rust decides everything; a screen shows what it returns.

1. Parse the typed amount with `manager.parseShieldedTipAmount(text)`, which
   returns credits. Show the parsed amount in the confirmation, not the typed
   text.
2. Call `manager.reviewShieldedTip(walletId, username)`. Show the recipient and
   the change: `New` when no recipient was confirmed for the name from this
   wallet before, `Unchanged`, or `Changed(previous)` when the name now pays
   another identity or address. A recipient is confirmed when a send to it is
   admitted, so none of these says a tip was paid.
   `unresolved` means an earlier payment to the recipient may still execute, so
   a send would be refused.
3. Send with `manager.sendShieldedTip(walletId, source, username,
   expectedRecipient, amount, memo)`, passing the review's `recipient` back
   unchanged. `source` is `ShieldedTipSource.Identity(id)` to pay from an
   identity's own shielded account, or `ShieldedTipSource.Account(n)` for an
   account by number. Rust resolves the name again and refuses a recipient that
   changed, so the user reviews again.

When the tip starts from a saved contact, pass the contact's identity id as
`contactIdentityId` to the review and the send. Rust refuses with
`ShieldedTipRecipientMismatch` when the contact's name now belongs to another
identity, with nothing sent; the user picks the recipient again. Leave it out
when the user typed the name, and do not compare the ids on the host.

`DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed` means the tip may have
left the wallet and may still execute: never send it again; a shielded sync
resolves it. Read `TransactionBroadcastUnconfirmed` and
`DashSdkError.StateTransitionResultUnconfirmed` the same way.

`ShieldedTipPending` means this call sent nothing, because another tip is being
sent or an earlier payment to the recipient is unresolved. It says nothing
about that earlier payment, which may be this same tip submitted twice and may
pay the recipient. A shielded sync resolves it as executed or as unable to
execute, and the refusal ends either way, so do not send the tip again because
the refusal ended or a new review says `unresolved == false`. Send it again
only once the earlier payment is known to have failed: its shielded activity
entry reads failed, not confirmed. The send returns no id for that entry, and
entries with the same account, recipient, amount and memo can have different
outcomes, so an entry that merely looks like the payment is not that evidence.
When the exact attempt cannot be identified, treat its outcome as unknown and
do not send again.

Any error other than these means the call sent nothing.

To show what an identity can tip from, call
`manager.shieldedIdentityBalance(walletId, identityId)` and show its state
instead of summing notes: `Unverified` until `prepareShieldedIdentityAddress`
proves the identity's account in this session, `Unavailable(account)` while the
account's ledger is not established (not known, which is not zero), and
`Available(account, credits)`. The read is local and waits at most 100 ms; a
busy `WalletOperation` means keep the last state and read again later.

## Identity-funded shield recovery

An unresolved identity-funded shield blocks another debit from the same identity
with `DashSdkError.PlatformWallet.ShieldedIdentityDebitPending`. Call
`manager.shieldedIdentityDebitRecoveryRecords(walletId)` to inspect active and
archived records. Each record is scoped by that wallet id plus `accountIndex`
and `activityId`; unreadable identity, nonce and amount fields are null. The
statuses are `RETRYING`, `PARKED`, and `UNKNOWN` (explicitly abandoned).

After explaining the risk and obtaining the user's acknowledgement, call
`manager.abandonShieldedIdentityDebit(walletId, accountIndex, activityId, acknowledgePossibleExecution)`
for the selected record. The final argument has no default and must be true.
This stops future automatic retry and permits another debit after durable
archival succeeds. It **does not cancel** an already relayed or in-flight
payment, even if the original nonce is still usable. A new payment could be an
additional debit. The complete signed record remains archived for later scan
confirmation and audit. Its activity outcome is Unknown (raw status 3), never
Failed merely because retry was abandoned.

`ShieldedRecoveryCorrupted` identifies malformed or invalid durable data;
`ShieldedRecoveryKeysRequired` identifies an account/key compatibility problem,
which damaged ciphertext can also cause. Retain these records for diagnosis.
