# DashPay — TODO / backlog

Single source of truth for outstanding DashPay work. Sources: the
kotlin-platform/dashj comparison (`KOTLIN_PLATFORM_COMPARISON.md`), the spec
track, and the multi-agent reviews. Prioritized; check off as done.

---

## P0 — bugs (functional / data-loss; fix soon)

- [x] **`update_profile` is destructive — wipes sibling fields.** Fixed
  (`0ad99d0282`): read-modify-write — seed the property map from the existing
  doc's `properties()`, overlay only provided fields, build the returned profile
  from the merged state (the local cache was wiped too). `profile.rs`.
- [x] **Sent payments stuck on `Pending` forever.** Fixed (`245d9da0e3`): wired a
  sender-side confirm path — a confirmed `TransactionDetected` re-detection flips
  the `Sent` entry `Pending→Confirmed` in place. `payments.rs`, `core_bridge.rs`.
- [ ] **Contact-request fetch truncates at 100, no pagination/high-water.**
  Newest requests buried permanently under a flood; non-incremental re-fetch every
  sweep. → **`SYNC_CORRECTNESS_SPEC.md` stage 1** (REVIEWED — implement).
  `contact_request_queries.rs:65,117`.
- [ ] **Contact-profile sync entirely absent.** We sync our *own* profile but
  never fetch contacts'/senders' displayName/avatar (`all_identities()` excludes
  contacts). → folded into **`SYNC_CORRECTNESS_SPEC.md` stage 2** (REVIEWED —
  id-keyed `contact_profiles` cache, established + pending senders).
  `accessors.rs:54`.

## P1 — interop (cross-client correctness)

- [ ] **`accountReference` ASK28 byte-order interop-break.** We read
  `be(ASK[28..32])>>4` (iOS dash-shared-core conv., chosen in M3); dashj/Android
  reads `le(ASK[0..4])>>4` — proven-different values. **Decide canonical** (iOS vs
  dashj — they disagree; check the G15 on-chain census + the iOS stack), flip
  `account_secret_key_28` + `unmask_account_reference` symmetrically, add a **dashj
  known-answer test**. → `dip14.rs:216-258`.
- [ ] **Friendship path hardcodes `account'` = `0'`.** key-wallet drops the
  account index from the derivation path; dashj derives under the counterparty's
  real account → disjoint address spaces if a counterparty uses account ≠ 0.
  Upstream rust-dashcore/key-wallet change (`account_type.rs:486,509`) + pass the
  real account on registration (`contacts.rs:474`). *(cross-repo; latent)*
- [ ] **`encryptedAccountLabel` not padded / omitted when empty.** kotlin always
  pads to ≥16 chars w/ spaces and always emits (empty → 16 spaces); labels <16
  chars currently **error** in our code. Fix: pad ≥16, trim on decrypt, always
  emit. → `contact_request.rs:319-334`.

## P2 — parity gaps / hardening

- [ ] **No per-contact tx-history query** (`getContactTransactions` equiv) and **no
  tx→contact reverse lookup for *sent* txs** (`match_in_collection` searches only
  receival pools, not external/send). Data exists (`PaymentEntry.counterparty_id`);
  add the accessors. → `contacts.rs:357`, `dashpay_payment.rs`.
- [ ] **Key selection narrower than canonical** — no AUTHENTICATION fallback on
  send (kotlin has one), DECRYPTION-first vs ENCRYPTION-first. *Product decision*
  (we can't send to an identity with only an AUTH ECDSA key; kotlin can).
  → `select_recipient_key_index`, `contact_requests.rs:395`.
- [ ] **ECDH dashj known-answer test** — lock the one byte-level assumption (we
  relied on dashj's class comment, not bytes) with a fixed-vector cross-impl KAT.
- [ ] **Multi-account contacts** — we keep one request per direction; a contact on
  simultaneous multiple accounts can't be represented (`accepted_accounts` exists
  but is never populated). Widen if multi-account becomes a requirement.
  → `contact_requests.rs:323-393`.
- [ ] **rs-sdk-ffi: `DashSDKContactRequestResult` drops `entropy`** — a non-Rust
  embedder calling `dash_sdk_dashpay_create_contact_request` + a generic
  document-put can't recover the entropy consensus needs to validate the doc id.
  Extend the C result struct with `entropy`. *(deferred from PR #3841 review as an
  rs-sdk-ffi follow-up; the example app uses the platform-wallet path, not this.)*
  → `packages/rs-sdk-ffi/src/dashpay/contact_request.rs:181`.
- [ ] Minor: contactInfo fetch is also 100-truncated (same pagination fix); send
  address-reuse if SPV drops our own broadcast tx (consider `mark_address_used` at
  broadcast).

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
- [ ] **Spec 2 — Ignore (per-sender mute), synced via contactInfo** (subsumes the
  old BLOCK_SPEC + reject→on-chain). Cross-device ignore signal rides a DIP-15
  **`relationshipState`** field — a minor-version extension on the Spec-1 varint
  format (additive, so a v0 reader ignores it). On Ignore: write it on the sender's
  `contactInfo` so every device applies it on sync; on-sync suppress the ignored
  sender from the **main incoming list** (all their requests, rotations included).
  Reversible (un-ignore). **Blocked on the R1 privacy investigation**
  (non-established-sender leak). `BLOCK_SPEC.md` (4-lens reviewed §0 R1–R10) is the
  starting point — per-sender; rename block→ignore, drop the separate reject path,
  keep Q1 (un-ignore resyncs / rewind cursor).
  - [ ] **Ignored list (UI + state):** a dedicated "Ignored" screen lists the
    ignored senders with an **Un-ignore** action — ignored ≠ invisible, just hidden
    from the main pending list. Requires persisting enough to display each
    (identity id min; **name/avatar needs their profile → depends on the
    contact-profile-sync fix, P0 #4**) and a query over `ignored_senders`.
- [ ] **Refactor: collapse `reject` → per-sender `ignore`.** `rejected_contact_requests`
  (keyed `(sender, accountReference)`) → `ignored_senders` (keyed by sender);
  `is_request_rejected(sender,ref)` → `is_sender_ignored(sender)`; simplify the
  restore/wipe/persist plumbing built this session. Decide terminology: `ignore`
  (Android term) vs keep `reject`/`block` in code. Established-contact rotation
  (`apply_rotated_incoming_request`) is UNCHANGED.
- [ ] **R1 privacy investigation** — does a `contactInfo` about a *non-established*
  sender leak who you blocked (count + `$createdAt`↔contactRequest timing)? Per-
  sender (leaky) vs single owner-scoped self-encrypted list (bounded) vs
  established-only. Resolve before Spec 2/3.

## Contract track (DIP / governance — later)

These need a change to the registered `dashpay` data contract, so they're a
DIP/maintainer-coordination effort separate from the wallet work.

- [ ] **Real query-level DoS protection — filter blocked/rejected senders out
  *before* fetching.** Incremental fetch (P0 #3) bounds cost but still fetches each
  new request once; truly *not fetching* a known-bad sender needs a server-side
  filter the current index can't serve (recipient-keyed, no `sender NOT IN`, +
  Sybil). Park until there's a contract-level mechanism. *(was deferred explicitly
  as "needs contract change")*
- [ ] (struck — see guardrails) the countable `[toUserId, $ownerId]` GROUP-BY index
  is NOT pursued (public count proof leaks the inbound social graph, R6).

## Cross-cutting research

- [ ] **Diff the iOS stack** (`dashwallet-ios` / `dashsync-iOS` / `dash-shared-core`)
  — the comparison was Android-only; iOS uses the *other* `accountReference`
  convention, so it's load-bearing for the P1 #1 canonical decision.
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
- [ ] **Comment-cleanup pass** — when next touching the DashPay code, strip
  spec-gate / milestone / dev-time refs from source comments (`G5 stage 1`,
  `M3 task 13`, `P0`, `RED before fix`) per the timeless-comments convention.
  Opportunistic, not a mass rewrite.

## Done (this session)

- [x] PR #3841 review feedback (cancel-token, transient→permanent, rejected-
  tombstone restore, purpose_mismatch, disabled keys, V001→V002 then squash,
  reject `removed_incoming`, wipe PHASE 1, seed zeroize) — all 45 threads resolved.
- [x] Comprehensive kotlin-platform/dashj/dash-wallet comparison
  (`KOTLIN_PLATFORM_COMPARISON.md`).
