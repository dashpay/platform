# Kotlin DashPay Migration — Known Leftovers & Follow-ups

Durable record of what is and isn't done as of the consolidated DashPay migration
PR (K1 + K2 + K3 + the follow-up fixes below). Kept so nothing is silently lost
when the four stacked PRs collapse into one.

## Resolved in this PR (follow-up fixes)

- **Seed hygiene on the platform-address signing path** — the mnemonic now crosses
  JNI as a scrubbable UTF-8 `byte[]` (`signWithMnemonicAndPathInto`) that the caller
  zeroes after use, never an un-scrubbable `java.lang.String`. Rust reads it into a
  pre-sized buffer so the `CString` conversion cannot orphan an unscrubbed plaintext
  copy. Pins: `SignerMnemonicScrubTest` (JVM, red→green).
- **Payment dispose-mid-send double-send guard** — the send flow was extracted to
  `performDashPaySend` + a `PaymentSender` seam so the `withContext(NonCancellable)`
  guard (broadcast + durability bookkeeping stay atomic against a mid-send teardown)
  has a deterministic regression test: `PerformDashPaySendDoubleSendGuardTest` (JVM,
  red→green).
- **`DataContractRef` GC backstop** — now registers a `NativeCleaner` like every
  other owned handle, so a leaked (never-closed) ref no longer leaks the native
  contract handle.

## Deferred to dedicated follow-up PRs

- **Durable contact-crypto queue persistence (item D).** The DashPay deferred
  contact-crypto queue (`PlatformWalletChangeSet.pending_contact_crypto_added/
  _cleared`) is not durable across process death on FFI hosts. Only the *write* half
  is cleanly addable; the *restore* half is blocked upstream — the in-repo SQLite
  persister's reader is `#[cfg(test)]`-only "because production load restore is
  blocked upstream (`LOAD_UNIMPLEMENTED: ClientStartState::wallets`)", and
  `rs-platform-wallet/src/wallet/apply.rs` drops the queue fields on the
  changeset-replay path. Adding the FFI slot + JNI trampoline + a `DashDatabase`
  v3→v4 Room migration + Swift model would persist rows nothing reads back — a large,
  irreversible surface (incl. a schema migration) for zero durability until the
  upstream start-state restore lands. The recurring signerless sweep already
  re-enqueues the work after restart, so the exposure is *delayed*, not *lost*
  contact-crypto work between sweeps. Do it as its own PR once the start-state
  restore exists. Full data-path map + add-a-persisted-field recipe recorded during
  the follow-ups research.

- **`signWithMnemonicAndPathInto` instrumented sign smoke.** The JVM test pins the
  seed-scrub invariant but NOT the JNI symbol binding: a Rust↔Kotlin symbol-name
  mismatch is a runtime `UnsatisfiedLinkError`, not a compile error, and the JVM
  helper test never loads the native symbol. `cargo check` + an emulator/on-device
  platform-address signature are what pin the actual native call.

- **PARITY: 8 partial / 7 deferred Swift views.** Each partial/deferred row in
  `packages/kotlin-sdk/PARITY.md` names the exact missing FFI export it needs; those
  land as the corresponding exports are added.

## Environment-bound (cannot be code-fixed here)

- **End-to-end send→accept→pay testnet UAT** — device/testnet-bound; not runnable in
  CI. Must be exercised on a real testnet wallet before relying on the full DashPay
  flow.
- **Live-network paths** (`searchDpnsNames`, `dashPaySyncNow`, the DashPay write
  paths) are exercised only under the `-Ptestnet=true` instrumented tier.

## Behavioral notes to carry (from the base Kotlin SDK PR)

- `rs-sdk-ffi` / `rs-sdk-trusted-context-provider` use rustls + webpki roots instead
  of the platform TLS stack (OpenSSL doesn't exist on Android). This also changes the
  iOS trust roots — no API change, but worth an iOS-side look.

## Review findings — all resolved upstream

Every P0/P1/P2 from the base-PR reviews (invalid Cargo `--features` arg, JNI
local-frame leaks, negative-amount/index/selector validation across credits/tokens/
funding/identity, `sendDashPayPayment` guard) was verified **already fixed** at the
base PR HEAD before this consolidation — none was outstanding. The only carried
review finding was the contact-crypto durability suggestion, addressed as item D
above.
