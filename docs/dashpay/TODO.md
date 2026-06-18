# DashPay — TODO / backlog

Single source of truth for outstanding DashPay work. Sources: the
kotlin-platform/dashj comparison (`KOTLIN_PLATFORM_COMPARISON.md`), the spec
track, and the multi-agent reviews. Prioritized; check off as done.

> **STATUS (2026-06-18): the implementable backlog is complete.** Every P0/P1/P2
> bug, the full sync-correctness spec (Spec 0/1/2 + reject→ignore refactor), the
> R1 privacy resolution, the per-contact accessor, and the comment-cleanup pass
> are done, tested, and pushed on `feat/dashpay-m1-sync-correctness`. The five
> remaining `[ ]` items are **blocked on resources outside this codebase**, not
> oversights:
> - **Devnet integration tests** + **on-device UAT** — need a funded devnet
>   identity / harness (deferred to the end by agreement).
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

- [ ] **On-device UAT of the PR #3841 fixes** (shipped + pushed, but not yet
  device-verified): rejected-tombstone restore (reject a contact → relaunch →
  stays gone, both SQLite + SwiftData backends), wallet-wipe leaves no DashPay
  plaintext, sent-payment Pending→Confirmed. Needs a devnet identity rebuild (sim
  store was reset). *NB: the reject→ignore refactor (Spec 2) will replace the
  tombstone path, so verify before or alongside that work.*
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
