# DashPay simultaneous multi-account contacts — implementation spec

> **Problem.** DIP-15 lets a contact expose **multiple DashPay accounts** at once —
> each a separate `contactRequest` with a distinct `accountReference` (DIP-15 §8.4,
> §8.9, §10.8). Our contact-state layer collapses everything to **one channel per
> counterparty** (`BTreeMap<Identifier, …>`, a single-channel `EstablishedContact`),
> and the rotation machinery actively *supersedes* a contact's prior request rather
> than letting accounts coexist. So we can neither represent nor pay across a
> contact's multiple accounts, we always send our own account `0`, and we drop
> `contactInfo.acceptedAccounts` on ingest.
>
> **Status.** REVIEWED (4 lenses, 2026-06-24) — **KEEP DEFERRED** (see *Review
> outcome* below: a foundational blocker B-1 + reopened DoS + abuse surface, and no
> requirement). This feature was **deliberately
> deferred** by the team as *"conditional, not a requirement"* (backlog
> dashpay/platform#4020 multi-account item; `DIP_CONFORMANCE_GAPS.md` §2). There is **no current product
> requirement** forcing simultaneous multi-account. This spec exists so the work is
> *scoped and reviewed* and can be implemented when a requirement appears — and so
> the decision to keep deferring is an informed one. **Do not implement before this
> spec is reviewed and a requirement exists.**
>
> **Source.** Scope map from the 2026-06-24 blast-radius audit (all file:line below
> verified against `feat/dashpay-m1-sync-correctness`, pinned rust-dashcore `b4779fc`).

---

## Review outcome (2026-06-24, 4 lenses) — **KEEP DEFERRED**

A four-lens review (DIP-15 domain-fit, state-machine feasibility, scope/go-no-go,
security/abuse), each grounded against the code, converged: **the spec is an
accurate scope map, but the feature must NOT be built as designed, and there is no
product requirement driving it. Keep it deferred.** The reviews also corrected
several claims in §0–§4 below (annotated inline as ⚠**REV**). If a requirement ever
appears, **the first deliverable is a focused "channel identity under an opaque
`accountReference`" design note (resolving B-1) — not code.**

### Blocking findings (must be resolved in a revision before any code)

- **B-1 — Channel identity is unsolvable from the wire (the foundational blocker).**
  The design keys channels by the raw `accountReference`, but DIP-15's
  `accountReference = (version<<28) | (ASK28 ^ account)` is a sender-private one-time
  pad: the version nibble is cleartext, but `ASK28 = HMAC(sender_secret, compact_xpub)`
  is uninvertible by the recipient, **and a rotation ships a new xpub**, so the
  low-28-bit value is *uncorrelated* across a rotation. Result: "rotation of added
  channel B" is **information-theoretically indistinguishable** from "a brand-new
  account." So channels cannot be keyed by `accountReference` and still collapse
  rotations. Channel identity must be **out-of-band** (user-assigned at accept time;
  every later rotation re-prompts "which channel does this replace?"). This is
  permanent UX, not a TODO — and it gates B-2/B-3 below. (`account_reference.rs:41-66`.)
- **B-2 — Keying the collapse by `accountReference` re-opens the PR #3841 sweep
  thrash.** Immutable on-chain docs never disappear; a rotated sender leaves both
  old+new docs returning every sweep. `newest_received_per_sender` collapses
  per-sender *because rotation mutates the reference*; keying the collapse by the
  reference produces two survivors that flip-flop the stored channel forever — the
  exact regression #3841 fixed. A fixpoint exists only with a rotation-stable key →
  loops back to B-1. (`contact_requests.rs:811-829,1087-1117,3103-3105`.)
- **B-3 — The "local channel index" corrupts the receiving derivation path.**
  `DashpayReceivingFunds.index` is a **hardened path component**
  (`account_type.rs:489`), not just a map key — it selects the BIP32 path the
  counterparty derives against. A fabricated local index desyncs our *advertised*
  receiving addresses from our *watched* ones → incoming payments to that channel
  become invisible. The receiving index must be our **real** DashPay account number
  (the one masked into the published `accountReference`); only the *external* account
  may use a namespace. The send-side real-account thread (§2.3) is the only correct
  mechanism. (`key-wallet account_type.rs:472-526`.)
- **B-4 — `BTreeMap<u32, ContactChannel>` silently overwrites on collision.** Keying
  by a non-unique 28-bit value means two channels masking equal silently shadow each
  other (fund misdirection). The on-chain unique index `($ownerId, toUserId,
  accountReference)` (`dashpay.schema.json:148-163`) bounds this **per-sender** (a
  sender can't broadcast two colliding docs), but the spec must *state and rely on*
  that invariant and **reject-on-collision** (insert returning `Some` = loud error),
  never overwrite.
- **B-5 — No per-sender flood cap; the re-key converts a flood into pending-queue
  exhaustion.** Today `incoming_contact_requests` is one slot per sender + collapse →
  a flood is structurally absorbed. The re-key to `(counterparty, accountReference)`
  makes each new reference a pending triage prompt (a permanent doc returning every
  sweep). Needs a `MAX_PENDING_ADDITIONAL_ACCOUNTS_PER_SENDER` (mirror
  `MAX_AUTO_ACCEPT_QUEUED_PER_OWNER`, but per-(owner,sender)); over-cap → **silently
  drop, not enqueue**; wire the gate to the existing `ignored_senders` block.
- **B-6 — "Add account" is a phishing / confused-deputy surface.** A *malicious
  established contact* can send an add-request whose xpub points at an
  attacker-controlled address space (the crypto binds the channel to the contact's
  identity, not to the contact being honest). "Add account" must carry the same
  trust gravity as accepting a brand-new contact (surface the derived first address;
  no one-tap inline accept). The spec frames the gate as anti-flood only and omits
  payment redirection.

### Corrections to the body (factual)
- ⚠**REV §2.2 / Open Q2:** the version nibble is readable but does **not** correlate a
  rotation to a specific channel (B-1). Don't claim "consult the version nibble"
  resolves rotation-vs-new — it doesn't.
- ⚠**REV §2.1:** strike the "local channel index" for the receiving account (B-3).
- ⚠**REV §2.2:** `acceptedAccounts` per DIP-15 §10.4 stores **only non-version-0**
  references — never write channel-0 into it.
- ⚠**REV §4.4:** same-sender collisions are *blocked on-chain* by the unique index;
  state this invariant (it's the saving grace) and reject-on-collision (B-4).
- ⚠**REV §4.3:** migration is essentially free — there is **no in-repo SQLite schema**,
  `DashMigrationPlan.stages == []` (dev stores recreate from scratch), and contacts
  rebuild from chain (metadata rides `contactInfo`). The spec over-worries; the real
  plan is "let the store rebuild," with a "wipe local → re-sync reconstructs" test.
- ⚠**REV §3:** T2 (`accepted_accounts` round-trip) is **not** independently valuable —
  it writes a field nothing reads (inert). Fold it into T1; do **not** ship standalone.
  T1 itself must split into ≥4 PRs (struct re-key / collapse-inversion / user-gate /
  account-index thread), each with its own #3841-style fixpoint test.
- **Conformant lower-cost fallback (R1):** DIP-15 §8.4 allows *"either disregard all
  future contact requests ... or preferably ask the user."* Silently disregarding
  additional requests (≈ today's collapse) is **also conformant** and avoids the
  entire B-2/B-3/B-5/B-6 surface — the cheapest path if multi-account is ever wanted
  only nominally.

### Verdict & recommendation
**KEEP DEFERRED.** Upstream (#813) is unblocked, but the feature has a foundational
information-theoretic blocker (B-1), re-opens a fixed DoS (B-2), and adds real abuse
surface (B-4/B-5/B-6) — for **no current requirement**. The review *prevented building
the wrong thing*, which is the point of the pipeline. **Next step only if a
requirement appears:** a B-1 channel-identity design note, then re-spec around it.

---

## 0. What "multi-account" means here (and what it does NOT)

Two distinct things share the "different `accountReference` from a known sender"
shape and must not be conflated:

- **Rotation (LIVE today):** the sender rotated the payment xpub for the *same*
  logical account; the new request **supersedes** the old (DIP-15 §8.10 immutability
  → rotate via a new request). `apply_rotated_incoming_request`
  (`state/managed_identity/contact_requests.rs:337-407`) replaces `incoming_request`
  in place, tears down the stale external account, rebuilds from the new xpub. The
  sync sweep's `newest_received_per_sender` (`network/contact_requests.rs:811-829`)
  **discards all-but-newest per sender** — the comment (`:752-765`) calls this "the
  idempotency keystone."
- **Simultaneous multi-account (THIS spec):** the sender exposes *additional* live
  accounts that must **coexist** as separate channels (DIP-15 §8.4 "Recipients either
  ignore subsequent requests or prompt users to select destination accounts";
  §10.8 "additional contact requests require user acceptance; upon approval the new
  account reference joins `acceptedAccounts`").

These are **antithetical** — rotation's whole purpose is to *prevent* two live
channels per sender. Multi-account must *invert* that for **accepted** additional
accounts while keeping supersede for genuine rotations. The disambiguation is the
crux of this spec (§2.2).

**Out of scope:** the §10.8 *query-level* flood mitigation ("only the first request
to the bloom filter; filter blocked senders server-side") needs a registered
`dashpay` contract change and stays blocked (Contract track). This spec covers the
**client-side** multi-account model only.

---

## 1. Research — current state (verified)

### 1.1 What's already multi-account-ready
- **Upstream derivation (#813, merged, in `b4779fc`):** `AccountType::derivation_path()`
  for `DashpayReceivingFunds`/`DashpayExternalAccount` uses
  `ChildNumber::from_hardened_idx(*account_index)` (`key-wallet/.../account_type.rs:472-531`)
  — the friendship path honors a non-zero account.
- **Account collections** are keyed by `DashpayAccountKey { index, user_identity_id,
  friend_identity_id }` (`key-wallet/.../account_collection.rs:25-29`) — the
  account/UTXO layer already supports multiple accounts per (user, friend).
- **Provider/register signatures already take an account index:**
  `receiving_xpub_for(…, account_index, …)`, `account_reference(…, account_index,
  version)`, `register_contact_account(…, account_index, …)` (`network/contacts.rs:140`),
  `register_external_contact_account` derives `DashpayAccountKey { index }`.
  The `accountReference` masking already folds `account_index` into the low 28 bits
  correctly (`network/contact_requests.rs:514-551`).

### 1.2 The bottleneck — contact state collapses to `Identifier`
`ManagedIdentity` (`state/managed_identity/mod.rs:62-85`):
- `established_contacts: BTreeMap<Identifier, EstablishedContact>`
- `sent_contact_requests: BTreeMap<Identifier, ContactRequest>`
- `incoming_contact_requests: BTreeMap<Identifier, ContactRequest>`

`EstablishedContact` (`types/dashpay/established_contact.rs:14-51`) holds **exactly
one** `outgoing_request` + **one** `incoming_request`. It carries a dead
`accepted_accounts: Vec<u32>` (`:34`) + `add/remove_accepted_account` (`:138-146`)
with **zero production callers**.

### 1.3 Hardcoded account `0` on send/build (≈6 sites)
`network/contact_requests.rs:476` (`let account_index: u32 = 0;`), `contacts.rs:397`,
the build sweep `DashpayAccountKey { index: 0 }` (`contact_requests.rs:1398`), the
register-receiving builds (`:1614`, accept `:2259`).

### 1.4 `accepted_accounts` is lossy
- Codec round-trips it (`crypto/contact_info.rs:133,238-281`, test `:346-366`). ✅
- Publish hardcodes empty (`network/contact_info.rs:499-506`, "isn't populated yet").
- `set_contact_metadata` (`state/managed_identity/contact_requests.rs:279-313`)
  copies only `alias/note/display_hidden` — **drops `metadata.accepted_accounts`**.
- Not marshalled to FFI/Swift anywhere.

### 1.5 The recipient-ignores-`accountReference` asymmetry
DIP-15 makes `accountReference` a sender-private one-time pad the recipient **cannot
reliably un-mask** (the 4-way convention split; `DIP_CONFORMANCE_GAPS.md` §3). So the
recipient **cannot** recover the sender's real account number from the wire. It can
only treat the **raw `accountReference` u32** as an opaque channel discriminator, and
derive the actual addresses from the **decrypted xpub** (which is account-correct).
→ multi-account channels must be keyed by the **raw `accountReference`**, not an
unmasked account number.

---

## 2. Chosen approach

### 2.1 Re-key contact state by `(counterparty, accountReference)`
Replace the single-channel model with a per-contact set of channels keyed by the raw
`accountReference`:
- `EstablishedContact` becomes multi-channel: a `BTreeMap<u32 /*accountReference*/,
  ContactChannel>` where `ContactChannel` holds the `{outgoing_request,
  incoming_request, payment_channel_broken}` that are today flat on
  `EstablishedContact`. Metadata (`alias`, `note`, `is_hidden`, `accepted_accounts`)
  stays **per-contact** (one alias for the person, not per channel).
- `incoming_contact_requests` / `sent_contact_requests` re-key to
  `(counterparty, accountReference)`.
- Account registration already keys by `DashpayAccountKey { index }`; the channel's
  account index comes from the **decrypted-xpub-derived** account, but since we can't
  unmask, we allocate a **local channel index** per accepted accountReference and use
  it as the `DashpayAccountKey.index` (the xpub is account-correct regardless; the
  index only namespaces our local account collection).

### 2.2 Disambiguate rotation (supersede) vs new account (coexist) — by USER GATE
We cannot tell from the wire whether a new `accountReference` is a rotation or a new
account (§1.5). DIP-15 §8.4/§10.8 resolves this with a **user gate**:
- The **first** request from a sender → auto-established (channel 0), as today.
- A **subsequent** request with a new `accountReference` from an established contact →
  surfaced as a **pending additional-account request**, NOT auto-applied. The current
  auto-`apply_rotated_incoming_request` supersede is **replaced** by: enqueue as
  pending; the user chooses **"replace addresses" (rotation)** or **"add account"
  (coexist)**.
  - "Replace" → supersede (today's behavior, the channel's request is swapped).
  - "Add" → the `accountReference` joins `accepted_accounts`, a new coexisting channel
    is built, and the receival/external accounts are registered under a fresh local
    index.
- `accepted_accounts` is the **persistent record of which additional references the
  user accepted** — so the gate is sticky across sweeps/restarts (an accepted ref is
  never re-prompted; an un-accepted one is dropped per §10.8, not bloom-filtered).

This **inverts the idempotency keystone** (`newest_received_per_sender` collapse) for
accepted references: the sweep must keep every *accepted* `accountReference`'s newest
doc, and collapse only *within* an accountReference (rotation of that channel). That
is the load-bearing, highest-risk change (§4.1).

### 2.3 Send side — thread a real account (gated behind a UI affordance)
Thread an `account: u32` param from the send FFI through the ≈6 hardcoded sites. The
example app gains an optional "send from account N" affordance; default stays `0`.
**Not a standalone change** — only meaningful once §2.1 state can hold the result.

### Alternatives rejected
| Approach | Why rejected |
|---|---|
| Unmask `accountReference` to recover the account number, key by that | Recipient can't reliably un-mask (4-way convention split, §1.5). |
| Auto-accept every new `accountReference` as a new account | Violates §10.8 flood mitigation; an attacker floods accounts. |
| Keep single-channel, just stop dropping `accepted_accounts` (Slice A) | Inert today (nothing produces a non-empty value); preserves a field nothing writes — YAGNI. |
| Reuse rotation as the foundation | Rotation *prevents* coexistence by design (§0); it's scaffolding to bypass, not build on. |

---

## 3. Layered change map (task split)

| Layer | Change | Rough size |
|---|---|---|
| **T1 — Rust contact state** | multi-channel `EstablishedContact`; re-key the 3 maps to `(counterparty, accountRef)`; per-contact metadata; invert the sweep collapse to per-accountRef; user-gate additional accounts; populate `accepted_accounts` | large, the core |
| **T2 — `accepted_accounts` round-trip** | `set_contact_metadata` copies it; publish reads it; (independently shippable as the data-layer floor of T1) | ~15-30 LOC |
| **T3 — Changeset/persistence** | accountRef in `SentContactRequestKey`/`ReceivedContactRequestKey` + `established` map key; carry `accepted_accounts` | medium |
| **T4 — FFI** | `account_index`/`accepted_accounts` on `ContactRequestFFI` + persist callbacks; +1 send param; pending-additional-account surface | medium |
| **T5 — Swift/SwiftData** | accountRef in `PersistentDashpayContactRequest` unique key; per-account grouping in ContactsView/ContactRequestsView/ContactDetailView/AddContactView/SendDashPayPaymentSheet; "add account vs replace" prompt; send-from-account picker | large, UI-heavy |
| **T6 — Tests** | unit (re-key, coexist, user-gate, accepted_accounts round-trip, rotation-still-supersedes-within-a-channel); `dp_*` e2e multi-account send/receive (devnet) | medium |

The persistence (T3) + Swift (T5) layers need a **migration** for existing
single-channel rows (map the lone channel to `accountReference` of its stored
request).

---

## 4. Failure modes & risks (for reviewers to stress)

1. **Inverting the idempotency keystone (T1, highest risk).** `newest_received_per_sender`
   collapse and `apply_rotated_incoming_request` supersede are the mechanism that keeps
   the recurring sweep from thrashing. Splitting "collapse per sender" into "collapse
   per (sender, accountRef), keep all accepted refs" must not reintroduce the
   multi-doc sweep thrash that PR #3841 fixed (the `newest_received_per_sender`
   comment at `:752-765`). Needs the same red→green pinning as the original fix.
2. **Rotation vs add ambiguity.** If the user picks "replace" we must supersede the
   *right* channel; if "add" we must not later mistake the rotation of an added channel
   for yet another new account. Channels keyed by raw `accountReference` make a
   *rotation within a channel* indistinguishable from a *new account* unless the
   version nibble is consulted — but the recipient ignores `accountReference`. Resolve:
   does "rotation of an added account" even occur, and how is it keyed?
3. **Migration.** Existing persisted single-channel contacts (SQLite + SwiftData) must
   map to the new keyed shape without losing alias/note/hidden/broken state or
   double-counting payments.
4. **The `accountReference == 0` collision.** Today everything is accountRef `0`-ish;
   re-keying must handle the legacy `0` channel and a genuinely-new `0`-masked account
   (collisions are possible — `accountReference` uniqueness isn't guaranteed, DIP-15 §7).
5. **UI blow-up.** A contact rendering as N rows vs one row with N accounts; the send
   sheet picking an account; the "add vs replace" prompt. Scope creep risk.
6. **No requirement = speculative surface.** Building this without a driving use case
   risks shipping inert complexity (Rule 2). The spec must end with a go/no-go.

---

## 5. Verification plan

- **T2 (unit):** `set_contact_metadata` preserves `accepted_accounts`; publish emits the
  contact's accepted set; round-trip through the codec. (TDD red→green.)
- **T1 (unit):** an established contact accepts a second `accountReference` → two live
  channels; a rotation of channel 0 supersedes channel 0 only; the sweep does not thrash
  across two recurring passes (mirror the PR #3841 idempotency pin); an un-accepted
  additional request stays pending and is not watched.
- **Migration (unit):** a persisted single-channel contact loads as a one-channel
  multi-account contact with metadata intact.
- **Integration (`dp_*` e2e, devnet-gated):** send from a non-zero account; receive +
  accept a contact's second account; pay across both.

---

## 6. Open questions for review (resolve before any coding)

1. **Go/no-go:** is there an actual requirement for simultaneous multi-account, or does
   this stay deferred? (The spec's existence shouldn't force the build.)
2. **Rotation-within-an-added-channel (§4.2):** does it occur in practice, and how is a
   channel keyed if not by raw `accountReference`? (Possibly `(accountReference &
   0x0FFFFFFF)` ignoring the version nibble — but the recipient can't unmask… revisit.)
3. **Metadata granularity:** confirm `alias/note/is_hidden` are per-contact (per person)
   and only `accepted_accounts` + `payment_channel_broken` are per-channel.
4. **UI model:** one contact row with N accounts (recommended) vs N rows. Send sheet
   default account.
5. **Could T2 (accepted_accounts non-lossy) ship now** as a tiny data-preservation fix
   ahead of the rest, or does shipping an inert field invite confusion? (Lean: ship with
   T1, not standalone.)
6. **Migration safety** for existing devnet/testnet contacts.
