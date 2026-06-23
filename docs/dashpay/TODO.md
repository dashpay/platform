# DashPay — TODO / backlog

Single source of truth for outstanding DashPay work. Sources: the
kotlin-platform/dashj comparison (`KOTLIN_PLATFORM_COMPARISON.md`), the spec
track, and the multi-agent reviews. Prioritized; check off as done.

> **STATUS (2026-06-18): the implementable backlog is complete.** Every P0/P1/P2
> bug, the full sync-correctness spec (Spec 0/1/2 + reject→ignore refactor), the
> R1 privacy resolution, the per-contact accessor, and the comment-cleanup pass
> are done, tested, and pushed on `feat/dashpay-m1-sync-correctness`. The
> remaining `[ ]` items are **blocked on resources outside this codebase**, not
> oversights — **except one new code bug found during UAT (imported-identity
> signing — see Verification & hygiene)**:
> - **On-device UAT — DONE 2026-06-20 (testnet):** ignore-restore ✅ and wallet-wipe
>   ✅ PASS; sent-payment Pending→Confirmed 🐛 FAILS. Two code bugs surfaced (both in
>   Verification & hygiene): imported-identity signing, and sent-payment stuck on
>   Pending. **Devnet integration tests** still need a funded harness.
> - **Encrypted profile ignored-list field** + **query-level DoS filter** — need a
>   registered `dashpay` data-contract change (DIP / governance), not wallet code.
> - The struck `[toUserId, $ownerId]` GROUP-BY index is a deliberate **don't-do**
>   (privacy guardrail R6), kept unchecked as a do-not-reintroduce marker.

---

## P0 — bugs (functional / data-loss; fix soon)

- [x] **`update_profile` is destructive — wipes sibling fields.** Fixed
  (`0ad99d0282`): read-modify-write — seed the property map from the existing
  doc's `properties()`, overlay only provided fields, build the returned profile
  from the merged state (the local cache was wiped too). `profile.rs`.
- [x] **Sent payments stuck on `Pending` forever.** Fixed (`245d9da0e3`): wired a
  sender-side confirm path — a confirmed `TransactionDetected` re-detection flips
  the `Sent` entry `Pending→Confirmed` in place. `payments.rs`, `core_bridge.rs`.
- [x] **Contact-request fetch truncation: DONE** (Spec 0 stage 1, `3f2051e8b3`) —
  paginated retrieve-all + incremental high-water cursor; no burying.
- [x] **Contact-profile sync: DONE** (Spec 0 stage 2 + UI + durable persistence,
  `1f53897b63`/`b1936a7312`/`87d6cc733d`) — id-keyed `contact_profiles` cache for
  established + pending senders, displayed in the UI, survives restart.

## P1 — interop (cross-client correctness)

- [x] **`accountReference` ASK28 byte-order — RESOLVED: keep ours** (`c47314a90c`).
  An iOS-stack diff found iOS dash-shared-core and our Rust are *algebraically
  identical*, and that DIP-15 makes `accountReference` a one-time-pad obfuscation
  **recipients MUST ignore** — so the 4-way convention split (ours/iOS, Android
  `le[0..4]>>4`, dash-evo-tool/DIP-literal `be[0..4]>>4`) has **no on-chain interop
  failure** and no canonical value. Keep ours (matches iOS, the dominant wallet);
  documented the split + added an ASK28 KAT.
- [~] **Friendship path hardcodes `account'` = `0'`** — **upstream fix submitted:
  [rust-dashcore#813](https://github.com/dashpay/rust-dashcore/pull/813).**
  key-wallet's `AccountType::derivation_path()` discarded the `index` field and pushed
  a fixed `0'`; #813 honors `*index` (red→green test, backward-compatible for acct 0).
  **Framing corrected:** this is NOT a counterparty-interop break — the recipient pays
  from the *shared xpub* and ignores `accountReference` (per DIP-15 + our code), so a
  multi-account counterparty doesn't affect us. The real limitation is that *we* can't
  run multiple DashPay accounts → it's the **same item as multi-account (P2)**. Remaining
  after #813 merges: bump the key-wallet rev + thread the real account through our callers
  (`register_contact_account(.., account)` etc., currently hardcoded `0`).
- [x] **`encryptedAccountLabel` padded to ≥16 chars: DONE** (`2419159bb3`). Pad with
  trailing spaces on encrypt (kotlin `padEnd(16)`) so the ciphertext clears the
  48-byte contract floor; trim on decrypt; always emit. Tests pin it.

## P2 — parity gaps / hardening

- [x] **Per-contact tx-history accessor — DONE.** `payments_for_contact(contact)`
  filters `dashpay_payments` by `counterparty_id` (covers BOTH sent and received,
  since `send_payment` records the counterparty at send time — the "sent reverse
  lookup" the old SPV `match_in_collection` lacked is already on `PaymentEntry`).
- [x] ~~Key selection AUTHENTICATION fallback~~ **RESOLVED: deliberately NOT added.**
  Reusing a signing (AUTH) key for ECDH is poor key separation, and no live client
  population needs it (research/06 §G15: every observed recipient has a DECRYPTION
  or ENCRYPTION key). Documented at `select_recipient_key_index`. (We accept not
  sending to an identity that has *only* an AUTH key; kotlin's fallback is a
  security smell, not a parity gap worth matching.)
- [x] **ECDH known-answer test: DONE** (`4ae8504a2b`). KAT recomputes the shared key
  by hand (`SHA256((y&1|2)‖x)`) for fixed keys + pins symmetry — locks the byte
  convention.
- [~] **Multi-account contacts** — **DEFERRED (conditional, not a requirement).** The
  DIP-15 codec now carries `accepted_accounts` (Spec 1), but nothing populates it;
  widen only if simultaneous multi-account contacts become a real requirement. Shares
  the upstream derivation-path fix [rust-dashcore#813](https://github.com/dashpay/rust-dashcore/pull/813)
  (P1 above) — that PR is the enabling prerequisite for any non-zero DashPay account.
- [x] **rs-sdk-ffi `DashSDKContactRequestResult` entropy: DONE** (`514b32ebd1`).
  Added an inline `entropy: [u8;32]` field for generic embedders.
- [x] **contactInfo fetch pagination: DONE** (`e757d9a528`). `send address-reuse` —
  **DEFERRED (minor):** only bites if SPV drops our own broadcast; `mark_address_used`
  at broadcast is a small hardening with no observed incidence — revisit if it occurs.
- [ ] **Wire QR-based auto-accept (DIP-15).** `auto_accept.rs` fully implements the
  DIP-15 auto-accept proof (generate + verify, `m/9'/coin'/16'/timestamp'`, the
  70-byte proof format) but nothing wires it to a flow — the send path threads
  `auto_accept_proof: Option<Vec<u8>>` and production always passes `None`; the
  source even carries `// TODO: Where and how we use these helpers?`. The feature:
  a QR creator embeds a proof so the scanner's client auto-sends + auto-accepts the
  contact request without manual approval. To do: define the QR payload + scan flow,
  generate the proof on QR create, verify + auto-accept on scan. Confirm parity vs
  Android (dashj / kotlin-platform) — our comparison doc doesn't cover it yet. NOTE
  for the signer-seed-elimination work: `generate/verify_auto_accept_proof` is a
  *sign* path (derive key → ECDSA-sign), so it converts to the `Signer` model
  cleanly when wired; keep the helpers (do NOT delete as dead code).

## Spec / design track (in order — sync is FIRST)

- [~] **Spec 0 — `SYNC_CORRECTNESS_SPEC.md`** (**REVIEWED**; resolutions folded
  in §9). **Rust core of both stages implemented, reviewed (2-lens: all 8
  invariants upheld, no prod unwraps), fixed, committed** — stage 1 pagination +
  high-water cursor (`3f2051e8b3`), stage 2 id-keyed contact-profile cache
  (`1f53897b63`), cadence 60→15s (`a06fdd00a0`), review fixes (`ef35ca55cb`).
  Cursor + `contact_profiles` are **in-memory** (survive a session; reset on cold
  restart = one safe full re-fetch).
  - [x] **Contact-keyed FFI accessor + UI bind** (`b1936a7312`):
    `platform_wallet_get_contact_profile(owner, contact)` + `getContactProfile`
    Swift wrapper; the five `cachedProfile`/profile reads (ContactsView,
    ContactDetailView, ContactRequestsView, AddContactView, SendDashPayPaymentSheet)
    now read the contact cache (own-profile fallback for self-contacts). Verified by
    a clean `build_ios.sh --target sim` + app build. Stage 2 displays end-to-end.
  - [x] **Durable persistence — Rust changeset layer** (`06053bf589`):
    `contact_profiles` on IdentityEntry + from_managed + merge + apply (LWW per
    contact id); `sync_contact_profiles` emits one changeset/owner on change.
    Round-trip test pins survive-snapshot→apply + full-replace overwrite. So
    contact profiles already round-trip cross-device / replay.
  - [x] **Durable persistence — host-FFI layer** (`87d6cc733d`): `contact_profiles`
    now round-trips to SwiftData — `ContactProfileRowFFI`/`ContactProfileRestoreEntryFFI`
    arrays on `IdentityEntryFFI`/`IdentityRestoreEntryFFI` (+`restore_contact_profiles`),
    `PersistentDashpayContactProfile` model + handler store/restore. Memory-safety
    audited (no double-free/leak/UAF) + contact-id length guard on restore. Verified:
    110 FFI tests + `build_ios.sh` BUILD SUCCEEDED. The high-water cursor stays
    in-memory by design (reset → one safe full re-fetch).
  - [ ] **Devnet integration tests** (need a paginated mock/real harness): >100
    no-bury, partial-page-no-advance, equal-`$createdAt` boundary, In-query proof
    binding (Q-c stage-1 testnet check).
  > **MODEL DECISION (2026-06-17): collapse reject + block + ignore into ONE
  > concept — `ignore` (per-sender mute, = block, reversible). DROP per-request
  > reject.** Rationale: reject's only justification (don't suppress a legit
  > rotation) is thin — if you ignored the person you ignored them; un-ignore
  > covers "changed my mind"; and it matches Android (Accept/Ignore, no reject).
  > Keep **established-contact rotation** (re-keying a friendship) separate and
  > untouched — that's not suppression.

- [x] **Spec 1 — contactInfo `privateData` CBOR → DIP-15 varint: DONE.** Rewrote
  `crypto/contact_info.rs` to the DIP-15 Dash-message format (`version`
  major<<16|minor u32 LE, varstr `aliasName`/`note`, `displayHidden` u8,
  `acceptedAccounts` varInt-count+u32[]); tolerant decode (unknown **major** ⇒
  discard, unknown **minor**/trailing ⇒ ignore), padded to the 48-byte ciphertext
  floor. Verified against canonical `dip-0015.md` with a **byte-vector** test +
  round-trip / forward-compat / major-reject / truncation (8 tests). Struct gains
  `accepted_accounts`; dropped the now-dead `ciborium` dep.
- [x] **Spec 2 — Ignore (per-sender mute) — LOCAL-ONLY: DONE** (`62b7ad1875`).
  Refactored the per-request reject machinery → per-sender `ignored_senders`
  (`ignore_sender`/`is_sender_ignored`/`unignore_sender`); sync suppresses ALL of
  an ignored sender's requests incl. rotations; established-contact rotation
  untouched; `removed_incoming` still emitted. FFI `restore_dashpay_ignored` +
  `platform_wallet_(un)ignore_contact_sender`. No on-chain artifact (R1). Reviewed
  (correctness + FFI memory-safety audit); fixed a TOCTOU where a sync sweep could
  clobber the un-ignore cursor rewind (`advance_if_unchanged`, unit-pinned). 273 +
  110 tests + iOS build green. Cross-device deferred to the encrypted-profile
  contract item below.
  - [x] **Ignored list (UI + state)** — `IgnoredContactsView` (`@Query`-driven,
    name/avatar via `getContactProfile`, Un-ignore); `PersistentDashpayIgnoredSender`.
- [x] **Refactor: collapse `reject` → per-sender `ignore`: DONE** (folded into
  Spec 2 above — `rejected_contact_requests`→`ignored_senders`, renamed across
  Rust/FFI/SwiftData/Swift; `apply_rotated_incoming_request` UNCHANGED).
- [x] **R1 privacy investigation — RESOLVED (2026-06-18): non-established ignore is
  LEAKY → go local-only.** A `contactInfo` about a non-established sender leaks who
  you ignored: its public `$createdAt`/`$updatedAt` (enumerable via the
  `ownerIdAndUpdatedAt` index) correlates with the inbound `contactRequest`'s
  `$createdAt` (public `userIdCreatedAt` index) → re-identifies the encrypted target,
  plus a count leak. DIP-15's ≥2-established-contacts gate doesn't cover a *fresh*
  non-established sender (no ambiguity — exactly the "trivial linking" it warns of).
  **Decision: ignore is local-only; cross-device sync goes through a future encrypted
  field on the `profile` doc (Contract track), whose update timing is conflated with
  normal profile edits.**

- [~] **Seed elimination — Q2 = remove the `attach_wallet_seed` workaround.**
  Spec `SIGNER_SEED_ELIMINATION_SPEC.md` (design, REVIEWED v3, + Q2 status banner);
  live tracker `SEED_ELIMINATION_HANDOFF.md`. **Done + verified** (platform-wallet
  292/292; glue builds host + iOS-sim): the whole seedless contact-request flow —
  send/accept/drain/always-enqueue sweep, `ContactCryptoProvider` + signer host
  primitives (ECDH/accountReference/contactInfo seal-open/wrong-seed), deferred-crypto
  queue + SQLite, C3 (resident ECDH path deleted). Discovery is resolved via the
  KEPT carry-scalar (not a rewrite).
  **(4-lens plan review folded in — see the spec's Q2 banner for the MUST-FIX detail.)**
  - [x] **#7 contactInfo seedless — DONE** (`0da8ca5ddb`, `413229f048`, `15ca790aad`).
    `contact_info_seal`/`open` on the provider (real-auth-path parity test); publish does
    find-existing via `open` + encrypt via `seal` (shared fetch split into key-free scan +
    resident wrapper); FFI gains `core_signer_handle`; sweep enqueues `ContactInfoDecrypt`
    (seedless) / decrypts resident (resident-key types); drain op implemented (re-fetch +
    open + apply + confused-deputy guard). `derive_contact_info_keys` KEPT for resident-key
    types (per the discovery decision). Full re-fetch+decrypt is testnet-validated.
  - [x] **Discovery/loading — confirmed NO library change needed.** External-signable
    wallets already route through `discover_from_master` (via `resolve_master_from_resolver`);
    the `ResidentWallet` variants stay for resident-key wallet TYPES. carry-scalar kept.
  - [x] **De-dup — DONE** (`db18688545`): dead `dash_sdk_dashpay_*` rs-sdk-ffi surface
    deleted (−743); the 4 inline provider constructions collapsed to one
    `resolver_contact_crypto_provider` helper.
  - [ ] **§4.9 deletion of `attach_wallet_seed`** (the remaining core). CASCADE (measured by
    removing the `make_wallet`/`make_wallet_with` attach calls): **5 tests fail** —
    `register_contact_account_persists_account_registration`,
    `register_contact_account_with_precomputed_xpub_builds_account`,
    `register_external_with_precomputed_shared_key_builds_account`,
    `reconcile_records_received_payments_from_receival_utxos`,
    `reconcile_does_not_clobber_existing_entry_for_same_txid`. They call
    `register_contact_account(None)` (resident receiving-xpub derive) or derive a resident
    xpub in their setup. Fix: derive the xpub via a `Wallet`-from-seed (or `SeedCryptoProvider`)
    and pass `Some`; then determine whether `register_contact_account`'s `None` branch is
    production-dead now (sweep always-enqueues, send/accept/drain pass `Some`) → if so,
    C3-it (non-`Option`). Then delete `attach_wallet_seed` (`manager/attach_seed.rs`) + the
    `platform_wallet_manager_attach_wallet_seed_from_mnemonic` FFI + its 4 tests
    (`manager.rs`) + the `make_wallet` attach calls + the `KeychainSigner.sign(...)->Data?`
    nil-swallow (Swift). MUST-FIX: wire `verify_binds_to_xpub` (§4.8, zero callers) in the
    SAME change. SHOULD-FIX: port `WipingXprv` to `sign_with_mnemonic_resolver.rs` (§4.2).
  - [ ] **Swift** — drain-on-unlock replacing `unlockWalletFromKeychain` re-attach; pass the
    `core_signer_handle` the contactInfo/send/accept FFI now require.
  - [ ] **Testnet on-device acceptance** — happy path (import funded seed → discover →
    send/accept + pay + publish; background-discover inbound contact then unlock → payable)
    PLUS the two cases the all-positive script misses: (a) wrong/mis-mapped seed rejected
    LOUD; (b) cross-device contactInfo deferral (publish on A → seedless B sync → unlock B →
    appears). `git grep attach_wallet_seed` empty.

- [ ] **§6b — restore the deferred-crypto queue into `PlatformWalletInfo` on load.**
  Reader `all_pending_contact_crypto` exists (`cfg(test)`-gated); blocked upstream by
  `persister.rs` `LOAD_UNIMPLEMENTED: ClientStartState::wallets` (no per-wallet
  rehydration yet — nothing to attach the queue to). Wire once that lands. (Not Q2.)
- [ ] **§4.2 — error-/unwind-path scalar wipe hardening.** `resolve_derived_xprv` is
  already fixed (the `WipingXprv` RAII guard); the sibling
  `rs-platform-wallet-ffi/src/sign_with_mnemonic_resolver.rs:203-223` still hand-places
  `non_secure_erase` on the Ok-path only (a `?`/panic leaks both scalars). Folded into
  **Q2 step 4 as a SHOULD-FIX** (security review) — port `WipingXprv` to the sibling.
- [ ] **§4.8 caveat — present-but-zero-keys import** isn't covered by the xpub
  self-check. The carry-scalar fix means imports now materialize keys (so it's
  currently moot), but if a zero-keys import recurs, `verify_binds_to_xpub` won't
  catch it. Track as a residual. (Not Q2.)

## Contract track (DIP / governance — later)

These need a change to the registered `dashpay` data contract, so they're a
DIP/maintainer-coordination effort separate from the wallet work.

- [ ] **Add an encrypted ignored-contacts field to the `profile` document
  (cross-device ignore sync, privacy-bounded).** Per R1, syncing ignores via a
  per-sender `contactInfo` leaks who you ignored (timing-correlation). An encrypted
  list field on the **profile** — a single doc that already updates for many reasons
  (display name, avatar), so an update doesn't *specifically* signal an ignore —
  carries the ignored set cross-device with a bounded leak and no per-sender
  existence/count leak. Needs a registered `dashpay` contract change (DIP /
  governance). Until then, ignore stays local-only (Spec 2).
- [ ] **Real query-level DoS protection — filter blocked/rejected senders out
  *before* fetching.** Incremental fetch (P0 #3) bounds cost but still fetches each
  new request once; truly *not fetching* a known-bad sender needs a server-side
  filter the current index can't serve (recipient-keyed, no `sender NOT IN`, +
  Sybil). Park until there's a contract-level mechanism. *(was deferred explicitly
  as "needs contract change")*
- [ ] (struck — see guardrails) the countable `[toUserId, $ownerId]` GROUP-BY index
  is NOT pursued (public count proof leaks the inbound social graph, R6).

## Cross-cutting research

- [x] **Diff the iOS stack — DONE (2026-06-18).** Read dash-shared-core /
  dashsync-iOS / kotlin-platform / dash-evo-tool / DIP-15. Result: iOS and our
  Rust `accountReference` are *algebraically identical*; FOUR conventions exist but
  the field is a recipient-ignored one-time pad, so there's no interop break. Fed
  the P1 #1 resolution (keep ours). No canonical value exists to chase.
- [x] ~~Design question: is our reject/block complexity warranted?~~ **RESOLVED
  (2026-06-17)** → collapse to a single minimal per-sender `ignore` (= block,
  reversible); drop the per-request reject tombstone machinery (see the Model
  Decision callout in the spec track). Android does nothing here, so we're
  inventing it — keep it deliberately minimal. Remaining build work lives in
  Spec 2 + the collapse-reject refactor.
- [x] ~~Reconcile research/01 vs /07 on the contactInfo format~~ **DONE
  (2026-06-18)** → the contract validates `privateData` by **length only** (the
  schema's "array in cbor" text is advisory documentation, not an enforced
  constraint), so the encrypted plaintext format is a free convention — and **we use
  DIP-15 varint** (the authoritative protocol spec; cross-client interop). `research/07
  §C`'s "the schema description wins" over-weighted an advisory note as binding. See
  the DIP-15 decision in `CONTACTINFO_FORMAT_SPEC.md` + Spec 1 above.

## Guardrails (don't do)

- ✗ Don't write a byte-exact cross-client test on `avatarFingerprint` — it's a
  perceptual hash; pixel pipelines differ (greyscale average vs luma, resize
  filter). Use Hamming distance if testing interop at all.
- ✗ Don't re-introduce the countable `[toUserId, $ownerId]` GROUP-BY index
  (struck — its public count proof leaks the inbound social graph; R6).

## Verification & hygiene

- [x] **On-device UAT of the PR #3841 fixes — DONE 2026-06-20 (testnet).** Ran the
  three checks end-to-end on the iPhone 17 Pro sim against testnet, with SwiftData as
  ground truth. Two passed, one surfaced a bug (below).
  - **✅ ignore-restore — PASSES.** Bob → contact request → Alice; Alice taps Ignore →
    `ZPERSISTENTDASHPAYIGNOREDSENDER` row (sender=Bob, owner=Alice). After **relaunch +
    a DashPay sync** the ignored row persists and Alice's Requests shows "No pending
    requests" — Bob never reappears; the contact-request count drops (Spec 2
    `removed_incoming`). Verified on the **SwiftData backend** (the iOS persister; the
    SQLite/`rs-platform-wallet-storage` backend is the Rust persister, covered by its
    unit tests, not exercised on-device).
  - **✅ wallet-wipe — PASSES.** "Delete Wallet" cleared **every** table to 0 — wallet,
    identities, public keys, and all 5 DashPay tables (profiles/requests/payments/
    contact-profiles/ignored) + accounts/TXOs/transactions; zero residual plaintext
    (no "Alice"/"Bob" display names). Seed-zeroize confirmed: relaunch shows **no
    recovery prompt** = the mnemonic was removed from the keychain. (NB: the dev
    "Settings → Manage Local Data → Clear All Data" is a platform-cache clear only — it
    does NOT touch wallet/identity/DashPay data; the real wipe is "Delete Wallet".)
  - **🐛 sent-payment Pending→Confirmed — FAILS (see bug below).**
  - Harness notes for re-runs: had to force `defaults write -g AppleKeyboards
    "en_US@hw=US;sw=QWERTY"` (the sim inherits the host's Russian HW-keyboard layout,
    Cyrillic-izing idb `text`). Imported identities can't sign (separate bug below), so
    the run used two **in-app-created** identities A=`uatalice0619.dash` /
    B=`uatbob0619.dash`. Asset-lock amount field in the create flow APPENDS (no
    select-all) — clear with N×backspace then type. Min lock 0.0025 → ~36M credits is
    too little for a DPNS name (needs ~81.7M); fund ≥0.01 DASH.

- [x] **🐛 BUG (found in UAT 2026-06-20): sent DashPay payment stuck on Pending — FIXED
  (code + red→green test; pending on-device re-verify).** **Root cause = event-routing
  gap (candidate (a)), NOT the txid split (candidate (b)).** The bridge ran the DashPay
  payment hooks only on `WalletEvent::TransactionDetected`, which fires *only* for the
  first **mempool** sighting (context `Mempool`/`InstantSend` → `is_confirmed()` is
  false → the confirm hook early-returns). A wallet sees its *own* broadcast in the
  mempool first, so the tx reaches a confirmed context only when a block mines it —
  delivered as `WalletEvent::BlockProcessed` (the tx in `updated` = "previously-known
  records that just confirmed"). The bridge consumed `BlockProcessed` for the core
  changeset (hence `ZPERSISTENTTRANSACTION` advanced to inBlock) but never ran the
  payment hooks on those records, so the `Sent` entry stayed `Pending` forever. The
  txid representation split (candidate (b)) is real in SwiftData but **irrelevant to the
  Rust match**: both `send_payment` (insert) and the confirm path (lookup) key by
  `dashcore::Txid::to_string()` (display hex), and the SwiftData payment-row txid is
  restored as the same display hex — Rust never compares the payment-table txid against
  the transaction-table (internal-byte) txid. **Fix** (`core_bridge.rs`): route the
  records carried by `BlockProcessed` (`inserted` ∪ `updated`; `matured` excluded) to
  the same `record_incoming_dashpay_payments` + `confirm_sent_dashpay_payment` hooks via
  a new `dashpay_payment_records` / `run_dashpay_payment_hooks` seam. **Tests** (red→green):
  `block_processed_confirms_sent_payment` (payments.rs, end-to-end through the real
  dispatch) + `dashpay_payment_records_covers_block_processed_inserted_and_updated`
  (core_bridge.rs, routing) — both ✖ before the `BlockProcessed` arm, ✔ after. Full
  `platform-wallet` lib suite 281/281 green; clippy clean.
  **On-device verified (2026-06-20, testnet, iPhone 17 Pro sim):** re-imported the wallet
  (rediscovered the on-chain identities + reconstructed the Alice↔Bob established contact
  from chain — no identity signing needed, since a DashPay payment is a Core tx signed by
  the wallet's BIP-44 keys, so bug #1 does not block it), sent 0.001 DASH Alice→Bob.
  `platform_wallet/run.log` shows the confirm hook firing
  (`Confirming sent DashPay payment owner=D8VCf8Lo… txid=3938e6b7…`) when block 1499379 was
  processed via `BlockProcessed`, and `ZPERSISTENTDASHPAYPAYMENT.ZSTATUSRAW` reached `1`
  (Confirmed). **The fix works.**
  - **Secondary gap surfaced (follow-up, not the primary bug):** the confirm hook flips the
    *in-memory* map and emits a changeset, but the Swift side never persists DashPay payments
    from the changeset/store path — `dashpay_payments_overlay` is carried in the Rust
    changeset yet has **no FFI persister callback**. Payments reach SwiftData only via the
    pull-based `refreshDashPayPayments` (FFI getter → upsert), triggered solely by
    `ContactDetailView` (`.task` / `onSent` / manual Refresh) — NOT by the recurring DashPay
    sync. So the confirmed status shows after re-opening the contact / tapping Refresh, but
    does **not** auto-update while the user watches the screen. Fix options: (A) call
    `refreshDashPayPayments` for eligible identities inside the recurring DashPay sync
    (Swift-only, pull-based, matches the existing pattern); (B) surface
    `dashpay_payments_overlay` via a new FFI persister callback + Swift handler (push-based,
    immediate). **FIXED via option A + verified on-device (2026-06-20):** added
    `ContactDetailView.onChange(of: walletManager.dashPaySyncIsSyncing)` falling-edge →
    `refreshPayments()` (mirrors the sibling `ContactsView`/`ContactRequestsView` idiom;
    the 1s poller in `PlatformWalletManager` mirrors the FFI `isDashPaySyncing()` for the
    recurring Rust loop too, so the falling edge fires on every recurring pass). Verified:
    sent a 2nd payment, stayed on the contact screen with **no manual Refresh** — the
    confirm hook fired (`Confirming sent DashPay payment … txid=5fa9e036…`) and the row
    auto-flipped to **Confirmed** in SwiftData + the UI (`Payments (2)`, both "Sent …
    Confirmed"). No automated test (UI-only SwiftUI modifier; on-device verified per the
    CLAUDE.md UI exception). Ignore-restore + wallet-wipe are untouched by these changes
    (isolated to `core_bridge.rs` payment routing + `ContactDetailView` payment refresh).
  - **Multi-agent review (2026-06-20, 5 lenses: correctness / adversarial / Swift-iOS /
    testing / maintainability):** no blocking issues; all rated ship-able. **Applied:**
    exhaustive `match` in `dashpay_payment_records` (a new record-bearing `WalletEvent`
    variant now fails to compile rather than being silently dropped — same bug class);
    cheap non-allocating `carries_payment_records`; `refreshPayments()` re-entrancy guard;
    timeless test doc-comment; +idempotency and +`matured`-exclusion assertions in the
    integration test (281/281 lib green, clippy clean, sim build green). **Follow-ups —
    all three approved + DONE (2026-06-20):**
    - [x] **(robustness) Sent-payment reconcile.** `IdentityWallet::reconcile_sent_payments`
      (new local-only `dashpay_sync()` step): for each `Pending` `Sent` entry, consult the
      persisted core tx record (`get_core_tx_record`) and flip it when the tx is mined or
      IS-locked. Recovers a sent payment whose live confirm event was missed (lagged
      broadcast, or relaunch after the tx confirmed) — the sent-side equivalent of the
      incoming UTXO self-heal. Test `reconcile_sent_payments_confirms_from_persisted_record`
      (mined→Confirmed, mempool→Pending, idempotent).
    - [x] **(product) InstantSend = Confirmed.** The confirm gate now accepts `InstantSend`
      context, and `WalletEvent::TransactionInstantLocked` (txid-only, no record) routes to a
      new `confirm_sent_dashpay_payment_by_txid` path, so a sent payment shows `Confirmed` on
      the IS lock (seconds after broadcast) rather than waiting for the block. Tests:
      `instant_send_lock_confirms_sent_payment`, `instant_send_context_record_confirms_sent_payment`,
      `instant_locked_drives_payment_hooks_without_a_record`.
    - [x] **(perf) SwiftData write-amplification fixed.** `persistDashpayPayments` now only
      mutates a row (and its `lastUpdated`) when a field actually changed, so the recurring
      sync-edge refresh no longer re-fires the `@Query` on a quiescent channel.
    - [ ] **(test) Incoming-payment recording via `BlockProcessed`** (block-first sighting)
      still pinned only at the routing layer, not end-to-end — minor; left as a TODO.
    - 285/285 platform-wallet lib tests green, clippy clean, full `build_ios.sh` (xcframework
      + app) green.

- [x] **🐛 BUG (found in UAT 2026-06-19) — RESOLVED 2026-06-21: an IMPORTED identity
  could not sign any state transition.** Fixed by the carry-derived-scalar change
  (`IMPORTED_IDENTITY_KEY_MATERIALIZATION_SPEC.md`, commit `c567981c46`): discovery
  carries the already-verified 32-byte scalar through changeset→FFI→Swift so the client
  stores it directly instead of re-deriving from a not-yet-persisted mnemonic. On-device:
  clean import → 23/23 keys signable, a discovered identity signed a DashPay profile.
  Regression tests green (`breadcrumb_decisions_*`, `add_keys_*`,
  `identity_key_entry_debug_redacts_private_key`). Original diagnosis (historical) below.
  Every signed op — register DPNS name, set DashPay profile, and by
  extension contact requests + payments — failed with
  `SDK error: Protocol error: Generic Error: No PersistentPublicKey row matches the
  supplied public-key bytes` (`KeychainSigner.swift:137`). Diagnosis at the time:
  - The 5 keys ARE persisted correctly — `ZPERSISTENTPUBLICKEY.ZPUBLICKEYDATA` exactly
    matches all 5 of the identity's on-chain public keys (verified by SwiftData query).
  - `KeychainSigner`'s lookup (`KeychainSigner.swift:308-310`) matches purely by
    `row.publicKeyData == publicKey` — it is NOT identity-scoped, so the cosmetic
    `PersistentPublicKey.identityId` base58-String-vs-`PersistentIdentity` raw-Data
    difference is a red herring.
  - Therefore the Rust SDK is handing the signer a public key that is **not among the
    5 derived/persisted keys** — a key-selection / derivation-path issue specific to the
    import-an-existing-identity path (create-in-app presumably signs fine).
  - **To pin the exact mismatch:** instrument `KeychainSigner.sign(identityPublicKey:data:)`
    to log the supplied public-key hex, rebuild, reproduce, diff against the persisted 5.
  - Blocks the imported-identity UAT path; does not block create-in-app. Likely lives in
    the identity-import/key-derivation in `platform-wallet` (Swift SDK only persists/loads).
- [x] **Comment-cleanup pass — DONE (2026-06-18).** Stripped spec-gate / milestone
  / dev-time refs (`G1a`..`G15`, `M3 task 13`, `(P2)`, stage labels) from source
  comments + log strings across 18 DashPay files in `rs-platform-wallet` /
  `rs-platform-wallet-ffi`; gate IDs replaced with their plain-English meaning
  where a bare deletion would dangle (e.g. `G4`→"host-side signing hook",
  `G1c`→"transient/permanent failure policy"). Comment/string-only — verified
  zero executable lines changed, builds green, zero residual tokens.

## Done (this session)

- [x] PR #3841 review feedback (cancel-token, transient→permanent, rejected-
  tombstone restore, purpose_mismatch, disabled keys, V001→V002 then squash,
  reject `removed_incoming`, wipe PHASE 1, seed zeroize) — all 45 threads resolved.
- [x] Comprehensive kotlin-platform/dashj/dash-wallet comparison
  (`KOTLIN_PLATFORM_COMPARISON.md`).
