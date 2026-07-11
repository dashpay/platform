# DashPay — DIP-15 + DIP-16 conformance gaps (code-verified audit)

> **Purpose.** A from-scratch re-audit of the DashPay implementation against the
> canonical [DIP-15](https://github.com/dashpay/dips/blob/master/dip-0015.md)
> (DashPay) and [DIP-16](https://github.com/dashpay/dips/blob/master/dip-0016.md)
> (Headers-First SPV synchronization — DIP-15 §12 is built on it), cross-checked
> against the **actual code** on `feat/dashpay-m1-sync-correctness` (not the
> self-reported status in `SPEC.md`/the backlog (now dashpay/platform#4020)). The goal was to catch anything the
> DIPs require that is **missing, stubbed, or only partially wired**, and to
> separate genuine gaps from deliberate divergences.
>
> **Date:** 2026-06-24. **Method:** six parallel code-reading passes (xpub/ECDH/
> key-purpose; accountReference/multi-account/DoS/label; coreHeight/block-rescan/
> sync-window; profile/contactInfo/DPNS; dash-spv rescan capability; full DIP-16
> 9-step sync ordering), each citing `file:line`, plus direct verification of the
> contested findings. SPV evidence is from the pinned `dash-spv` rev `b4779fc`
> (`rust-dashcore`), which the platform-wallet drives via `spv/runtime.rs`.
>
> **Headline.** The DashPay (DIP-15) core flow **fully conforms** and is in places
> *ahead* of the reference clients. The SPV layer (DIP-16) implements the hard
> parts for real (headers-first + checkpoints + masternode-list/quorum
> verification + compact filters) but **deliberately diverges** from the DIP's
> literal phasing (event-driven parallel managers, BIP157 instead of BIP37 for the
> confirmed path, L2 decoupled from L1). Only **two** DIP-15 gaps are
> under-tracked (one a real incoming-payment-loss risk); the rest are correctly
> tracked as deferred (blocked on external resources) or are well-reasoned
> divergences. The §12.6 block-rescan gap (§1.1) turns out to need only a small
> wallet-side trigger — the rescan engine already exists in dash-spv.

---

## 0. Conformance matrix (by DIP-15 section)

| DIP-15 area | § | Verdict | Evidence |
|---|---|---|---|
| Encrypted xpub = 69-byte compact `fp(4)‖cc(32)‖pk(33)` → 96-byte ciphertext | 8.6 | ✅ **FULLY** | `rs-platform-encryption/src/compact_xpub.rs` (`COMPACT_XPUB_LEN=69`); send asm `network/contact_requests.rs:480-491`; SDK 96-byte assert `rs-sdk/.../contact_request.rs:311-316`; KAT `dip14.rs::compact_xpub_is_69_byte_dip15_plaintext_not_107_byte_encode` |
| ECDH `SHA256(((y&1)|2)‖x)` | 8.3 | ✅ **FULLY** | `rs-platform-encryption/src/ecdh.rs:16-26` + hand-recomputed KAT `:56-85` |
| `senderKeyIndex`/`recipientKeyIndex` purpose policy (liberal receive, ENCRYPTION send fallback, no permanent break on purpose mismatch) | 8.3 | ✅ **FULLY** | send sel `contact_requests.rs:846-871`; validator `crypto/validation.rs:141-251`; purpose-only ≠ broken `:90-92` + drain `:1743-1764` |
| Friendship path `m/9'/coin'/15'/0'/owner256/cp256/index`, DIP-14 256-bit non-hardened CKD | 8.9 | ✅ **FULLY** (account 0) | `crypto/dip14.rs`; byte-identical to dashj per `INTEROP_DESK_CHECK.md` |
| `profile` (displayName/publicMessage/avatarUrl/avatarHash/avatarFingerprint) | 9 | ✅ **FULLY** | `types/dashpay/profile.rs:85-123` — real SHA-256 hash **and** real 8-byte dHash; non-destructive update `network/profile.rs:319-378` |
| Batched profile fetch `$ownerId in [ids]` (counterparties of new requests) | 9.10 | ✅ **FULLY** | `network/profile.rs:738-812` (`In` + required `orderBy`) |
| `contactInfo` (ECB `encToUserId`, CBC `privateData`, `65536'/65537'`, ≥2-contacts gate, varint privateData) | 10 | ✅ **FULLY** | `crypto/contact_info.rs:45-48,232-283`; `rs-platform-encryption/src/contact_info.rs`; gate `network/contact_info.rs:548-556` |
| `accountReference` value + version-bump rotation on re-send | 7, 8.4 | ✅ **send** / ⚪ **receive ignores (by design)** | `account_reference.rs:41-51`; version bump `contact_requests.rs:514-547` |
| `$createdAt` incremental fetch with 10-min skew back-off | 8.8, 8.12 | ✅ **FULLY** | `SYNC_OVERLAP_MS=600_000` → `contact_requests.rs:770-776`; `StartAfter` paging `contact_request_queries.rs:54-108` |
| `$createdAtCoreBlockHeight` populated | 8.7 | ✅ **FULLY** | server-side `document_create_transition/v0/mod.rs:253-256`; client sends `None` `rs-sdk/.../contact_request.rs:478` |
| DPNS name↔identity resolve/search/cache | 11 | 🟡 **PARTIAL** | works (`network/dpns.rs:281-362`); QR-build doesn't fall back to on-chain name |
| **L1 block re-scan from `min(coreHeightCreatedAt)` on new contact** | **8.7, 12.6** | ❌ **MISSING** | never read to drive a rescan; SPV exposes no rescan entry point |
| `encryptedAccountLabel` (48–80B, padded, decrypted) | 8.5 | ✅ **FULLY** | send length-normalized in the crypto primitive (`account_label.rs`); receive decrypted + surfaced via `store_contact_account_label` (incoming-only) → `ContactDetailView` (SPEC.md Milestone 3) |
| `acceptedAccounts` + first-request bloom gating / flood mitigation | 8.4, 10.8 | ❌ **MISSING** | codec only; unpopulated + dropped on ingest |
| Multi-account contacts (`Account ≠ 0`) | 7.1, 8.9 | 🟡 **DEFERRED** | `account_index` hardcoded `0`; blocked on upstream |
| QR auto-accept (`autoAcceptProof`, `m/9'/5'/16'/expiry'`, BIP21/72 URI) | 8.13 | ✅ **FULLY** (iOS-first) | `crypto/auto_accept.rs`; see `QR_AUTO_ACCEPT_SPEC.md` |
| Invitations (asset-lock voucher + claim onboarding, DIP-13) | — | ❌ **NOT STARTED** | queued as "NEXT" in the backlog (dashpay/platform#4020) |

---

## 1. Under-tracked gaps (the value of this audit)

### 1.1 🔴 No L1 block re-scan from `coreHeightCreatedAt` on new contacts — DIP-15 §8.7 + §12.6

**Status: MISSING and not mentioned anywhere in the existing docs.** This is the
only finding with an incoming-**payment-loss** character.

DIP-15 §8.7 / §12.6 require: when a wallet learns of a new contact request, it must
**resynchronize L1 blocks from the minimum `$coreHeightCreatedAt`** across the new
requests *after* inserting the new address spaces into its filters, so it doesn't
miss payments sent in the device-sync-speed-skew window (a payment that landed on a
DashPay address before that address was being watched).

What the code actually does:
- `$createdAtCoreBlockHeight` **is** captured and persisted on every request
  (`types/dashpay/contact_request.rs:38`), but is **never read** to drive a
  re-request. No "minimum across new contacts" is computed anywhere.
- Both account-registration paths — `register_external_contact_account`
  (`network/contacts.rs:389`) and `register_contact_account` (`:140`), called from
  the G1b sweep at `network/contact_requests.rs:1630,1820` — watch **forward only**
  and aren't even passed the height.
- A newly registered contact's addresses **do** enter the compact-filter match set
  (`monitored_script_pubkeys` enumerates `all_accounts()`), but only from the
  current scan pointer forward — nothing rewinds the pointer to backfill.

Consequence: the `G1(b)` sync fix rebuilds the address *watch* on restore-from-seed,
but does **not** backfill *history*. An incoming DashPay payment that arrived before
the receiving account was (lazily) registered — restore-from-seed, second device, or
the offline-accept→pay window — can be silently missed until some unrelated full
rescan happens to cover it.

**The fix is small — the rescan engine already exists.** dash-spv's `FiltersManager`
already performs a targeted backfill rescan whenever a wallet's `synced_height` drops
below the filter scan pointer: `tick` calls `wallets_behind(committed)`, takes the
min stale height, runs `reset_for_rescan()` + `start_download()`, and re-downloads
BIP157 filters from there, re-matches against the now-larger script set, and
re-requests the matching blocks (`dash-spv .../sync/filters/sync_manager.rs:213-236`,
`manager.rs:129-139`). So DIP-15 §12.6 is **a wiring task, not an SPV build**:
1. **platform-wallet (the actual gap):** when the G1b sweep registers a new DashPay
   account, lower that wallet's `synced_height` to
   `min($coreHeightCreatedAt over the just-built accounts) − 1`. The height is
   already on the `ContactRequest`; the existing `FiltersManager` does the rest.
2. **one small upstream piece (`key-wallet-manager`):** `WalletInterface::
   update_wallet_synced_height` is **forward-only by contract** — "a value below the
   current is silently ignored" (`wallet_interface.rs:127-129`). A backward rescan
   needs a new guard-bypassing method (e.g. `reset_wallet_synced_height_to(id, h)`),
   a small upstream change in the vein of rust-dashcore#813. (Optionally expose a
   thin `DashSpvClient::rescan_wallet_from(id, h)` convenience wrapper; the
   `SpvRuntime` would forward it.)

Constraints to respect: the backfill floor is the checkpoint the headers were seeded
from (`manager.rs:192`), and the BIP157 filter-headers/filters for that range must be
re-downloadable from peers. Per DIP-15 §12.6, re-request slightly beyond the minimum
height and avoid re-requesting the final ~10 blocks near the tip. It is a genuine
correctness gap, but a contained one — see §6.4 for how it relates to the DIP-16
filter layer.

### 1.2 🟡 `encryptedAccountLabel` — the "DONE" padding fix is dead code

**Status: PARTIAL, and it contradicts a backlog ("DONE + tests pin it") claim (now dashpay/platform#4020).**

The backlog P1 item records label padding to ≥16 chars (commit `2419159bb3`) as done. In
reality:
- The padded helper `IdentityWallet::encrypt_account_label` + `pad_account_label`
  (`network/account_labels.rs:19,49-64`) has **zero live callers** (verified by grep;
  only its own unit tests reference it).
- The **live** path — FFI `platform_wallet_send_contact_request_with_signer`
  (`rs-platform-wallet-ffi/src/dashpay.rs:236-269`) → `send_contact_request_with_external_signer`
  (`network/contact_requests.rs:374`) → `sdk_writer` → rs-sdk — passes the host label
  **raw**. The SDK encrypts it unpadded and hard-rejects `<48 || >80` bytes
  (`rs-sdk/.../contact_request.rs:319-330`). A **1–15-character label therefore errors
  the entire contact-request send** (16-byte plaintext block → 16 ciphertext + 16 IV =
  32 < 48). The FFI accepts a label, so this is reachable, not theoretical.
- The label is **never decrypted on receive**: the ingest path stores
  `encrypted_account_label` as raw bytes (`contact_requests.rs:2515`) and nothing calls
  `decrypt_account_label` (also dead code in `account_labels.rs:78-107`). The field is
  effectively write-only.

A later refactor (the seedless `ContactCryptoProvider`/`sdk_writer` seam) appears to
have orphaned the padded helper.

**Resolution (2026-06-24) — send side ✅ fixed; receive surfacing 🟡 remaining.**
The DIP-15 length normalization now lives in the single primitive
`platform_encryption::{encrypt,decrypt}_account_label`: a short/empty label is
space-padded to clear the 48-byte floor **and** an over-long label is truncated (on a
char boundary) to stay under the 80-byte cap — so **no** host-supplied label can error
the broadcast anymore (the review caught that the floor fix alone left a symmetric
`>80` long-label failure). The dead `network/account_labels.rs` helper was deleted (it
duplicated the convention). Red→green test
`account_label_is_always_a_valid_48_to_80_byte_field` pins both bounds + multi-byte +
the exact-48 boundary.

**Receive-side surfacing — RESOLVED (2026-06-24, 5-lens reviewed; folded into SPEC.md
Milestone 3).** The label is now decrypted in Rust at the two signer-bearing
register sites (drain `RegisterExternal` Ok-branch + `accept_register_external_validated`,
where the ECDH `shared` already lives) and stored on
`EstablishedContact.contact_account_label`. It is **direction-specific** — derived
strictly from the *incoming* request and projected onto the **incoming FFI row only**
(the outgoing row's label is one *we* sent and is never surfaced), so it does **not**
copy the symmetric `alias`/`payment_channel_broken` both-rows pattern. Decrypt
failures / non-printable garbage coerce to `None` (cosmetic — never breaks the
channel); rotation pre-clears the field so it never goes stale. Surfaced through
`ContactRequestFFI.contact_account_label` → `PersistentDashpayContactRequest
.contactAccountLabel` → a read-only "Their account" row in `ContactDetailView`.
Backfill of pre-feature contacts deferred (dev-only; DashPay unreleased).

**On-device UAT (paloma, 2026-06-25) found a SECOND, decisive bug + fixed it.**
The receive-side surfacing above had nothing to decrypt because the **recurring
sweep's ingest parser `parse_contact_request_doc` silently dropped
`encryptedAccountLabel`** (it read `encryptedPublicKey` + `autoAcceptProof` but not
the label). The send always attached the label and the decrypt was always correct —
the label just never reached the recipient's stored request. (This audit's earlier
"ingest works" claim cited the *sent*-request parser at `:2515`, missing that the
*received* path uses `parse_contact_request_doc`.) **Fix:** the parser now reads
`encryptedAccountLabel`; the sender's local bookkeeping also stores it off the
broadcast doc; and `AddContactView` gained an optional "Account label" field so
labels can be sent in-app. Unit tests missed the bug (they built the incoming
request *with* the label, bypassing the parser) — now pinned by
`parse_contact_request_doc_carries_encrypted_account_label` (red→green). **Verified
full e2e on paloma:** send (48-byte label on-chain) → fresh sweep ingest
(`enc=48`) → accept decrypt (`contactAccountLabel="Bob savings acct"`, incoming row
only / outgoing null) → ContactDetail shows "Their account: Bob savings acct".

---

## 2. Tracked-and-deferred gaps (acknowledged; blocked on external resources)

These are real DIP-15 gaps, but the existing docs already record them with a correct
blocker — not oversights.

| Gap | DIP-15 § | Blocker | Doc ref |
|---|---|---|---|
| **True multi-account (`Account ≠ 0`)** — `account_index` hardcoded `0` at the only send site (`contact_requests.rs:476`); friendship path structurally `…/15'/0'/…`. (Key *rotation* via version-bump **is** live.) | 7.1, 8.9 | upstream `rust-dashcore#813` (honor the `index` field) | backlog dashpay/platform#4020 P1/P2 |
| **`acceptedAccounts` + §10.8 flood mitigation** — varint codec carries the field, but publish hardcodes it empty (`network/contact_info.rs:499-506`) and `set_contact_metadata` (`managed_identity/contact_requests.rs:289-299`) **drops** it on ingest. No "first request → bloom filter, additional → require acceptance" gating. | 8.4, 10.8 | query-level DoS filter needs a registered contract change | backlog dashpay/platform#4020 Contract track |
| **Cross-device ignore sync** — ignore is local-only; a per-sender `contactInfo` leaks the ignored target (timing correlation, R1). | 10.7 | needs an encrypted field on the `profile` contract (governance) | backlog dashpay/platform#4020 Contract track |
| **DPNS-name on-chain fallback in QR auto-accept build** — `build_auto_accept_qr` (`rs-platform-wallet-ffi/src/dashpay.rs:801`) uses the locally-cached name; empty for imported/devnet identities. `resolve_name` exists but isn't called from the QR path. | 11 | none (small follow-up) | backlog dashpay/platform#4020 P3 |
| **DashPay Invitations** — asset-lock voucher + claim onboarding (DIP-13 sub-feature `3'`). | — | new feature (L1 funding + identity registration + deep-link) | backlog dashpay/platform#4020 "NEXT" |
| **Devnet/testnet e2e + full add→approve→pay XCUITest** | 11 | funded test harness | backlog dashpay/platform#4020, `SPEC.md` Part 7 |

---

## 3. Deliberate divergences (correct decisions, not bugs)

- **`accountReference` ASK28 byte order** uses the **iOS** convention
  (`be(ASK[28..32])>>4`); iOS and Android genuinely disagree, and the field is a
  sender-private one-time-pad the **recipient ignores** (`unmask_account_reference` is
  only ever called by the sender's own re-send path), so there is no on-chain interop
  break. Documented + KAT-pinned.
- **Reject → reversible local-only `ignore`** (per-sender mute), matching Android's
  Accept/Ignore model. No on-chain artifact (R1 privacy).
- **Retained 78/107-byte xpub `decode()` fallback** in `network/contacts.rs:447-461` —
  documented insurance for local-only legacy rows; never participates in on-wire
  encoding (send only ever emits 69 bytes; the SDK rejects non-69 before encryption).

### 3.1 Reference-client (dashj / kotlin-platform) source pointers

For re-checking our behavior against the canonical Android stack — `dashpay/kotlin-platform`
(`org.dashj.platform.dashpay`, the live lib), `dashpay/dashj` (core crypto/keychains),
and `dashpay/dash-wallet` (the app: sync, UI, DAOs), all on `master`. (`android-dashpay`
is the **stale** predecessor, last push 2024-01 — do not diff against it.) The
reference-side anchors that pin each cross-client comparison:

| Concern | Reference-client anchor |
|---|---|
| `accountReference` ASK28 byte order | `BlockchainIdentity.getAccountReference` = `wrapReversed(ASK).toBigInteger().toInt() ushr 4` (= `u32_le(ASK[0..4])>>4`; we use the iOS `be(ASK[28..32])>>4` — §3 above) |
| Friendship path (receive vs send account) | `FriendKeyChain.getContactPath` — `contact.getUserAccount()` (receive) / `getFriendAccountReference()` (send) |
| `contactRequest` pagination (drain past 100) | `Documents.getAll` loops `startAt = last.id` while `size >= 100`; `retrieveAll` ⇒ `limit(-1)` |
| High-water + 10-min skew overlap | `PlatformSyncService.kt:346-372`, `DashPayContactRequestDao.kt:50-54` (`MAX(timestamp)` per direction) |
| Batched contact-profile fetch | `updateContactProfiles` → `Profiles.getList` (chunks of 100, `whereIn $ownerId`) |
| Non-destructive profile update | `Profiles.replace` — read-modify-write (`profileData.putAll(currentProfile.toObject())`, then overlay) |
| `encryptedAccountLabel` padding | `padAccountLabel()` — pad to ≥16 chars with spaces, always emit |
| Recipient-key selection | kotlin = ENCRYPTION-first with AUTH/HIGH fallback |
| Sent-tx status (live, not stored) | derived from `TransactionConfidence` |
| tx→contact reverse (both directions) | `getFriendFromTransaction` scans sent + received pools |
| Account/keychain self-heal | `checkDatabaseIntegrity` |

**Perceptual-hash caveat — do NOT write a cross-client exact-match test on
`avatarFingerprint`.** The dHash byte/bit layout coincidentally matches dashj, but the
pixel pipeline differs (greyscale **average vs luma-weighted**, resize filter, 9×9 vs
9×8), so fingerprints **will not be byte-identical cross-client**. That is inherent to
perceptual hashing — the fingerprint is used for Hamming distance, never equality — so a
cross-client exact-match assertion is wrong by construction.

---

## 4. Correction to the existing docs

- **`SPEC.md` G3 ("`accountReference` hardcoded to 0", deferred to M3) is STALE.**
  Code verification shows the send path computes a **real** `accountReference` and
  does **version-bump rotation** on re-send (`contact_requests.rs:514-551`,
  `account_reference.rs:41-51`). Only the *account-number* multi-account case remains
  at `0` (§2 above). The leftover comment `sdk_writer.rs:114` ("DashPay account
  reference (currently 0)") is rotted and should be corrected.

---

## 5. Where the implementation is *ahead* of the reference clients

For calibration (don't "fix" these):
- A real `contactInfo` document type — `kotlin-platform`/`dashj` have **none**.
- A genuine 8-byte dHash `avatarFingerprint` (commonly stubbed/zeroed elsewhere).
- Hand-recomputed ECDH + 69-byte-xpub known-answer tests (not just doc-comment trust).
- Stricter sync re-entrancy/shutdown discipline and a more robust
  `reconcile_incoming_payments` self-heal than dashj.

---

## 6. DIP-16 (Headers-First SPV synchronization) conformance

DIP-15 §12 requires sync to follow **DIP-16**. The SPV client lives in the
`dash-spv` crate (rev `b4779fc`), driven by `packages/rs-platform-wallet/src/spv/`.

**Two architectural facts frame every verdict:**
1. dash-spv is **not** a literal 4-phase sequential state machine. It is an
   **event-driven coordinator** that spawns 8 independent managers (block-headers,
   filter-headers, filters, blocks, masternode, chainlock, instantsend, mempool),
   each in its own tokio task, progressing reactively off a `SyncEvent` bus
   (`dash-spv/src/sync/sync_coordinator.rs:33-62,197-250`). DIP-16's *phases* are
   realized as concurrent managers, not ordered stages.
2. The **confirmed-tx receive path uses BIP157/158 compact filters** (pulled from
   peers, matched locally against wallet scripts) — **not** BIP37 bloom. BIP37
   `filterload` exists *only* in the optional mempool (unconfirmed-tx) manager.
   DIP-16 step 8 literally says "construct a bloom filter"; the implementation
   substitutes BIP157 for the confirmed path — a deliberate, stronger-privacy
   deviation.

### 6.1 Conformance matrix (DIP-16 9-step + phasing + locator)

| DIP-16 element | Verdict | Evidence (`dash-spv` unless noted) |
|---|---|---|
| Step 1 — chain height from **multiple** peers | ✅ IMPLEMENTED (uses `max`, not soft-consensus) | `network/pool.rs:105-151` |
| Step 2 — headers-first from checkpoints + chain/PoW validation | ✅ IMPLEMENTED | `chain/checkpoints.rs:158-725`; `sync/block_headers/pipeline.rs:56-113`; `validation/header.rs:17-46` |
| Step 3 — terminal masternode list + quorums | ✅ IMPLEMENTED | `sync/masternodes/sync_manager.rs:255`; `manager.rs:575` |
| Step 4 — intermediate MN lists to verify quorums | ✅ IMPLEMENTED | `sync/masternodes/sync_manager.rs:42-147,369` |
| Step 5 — verify quorums (real, not stubbed) | ✅ IMPLEMENTED | `sync/masternodes/manager.rs:487,577` |
| Step 6 — retrieve identities | 🟡 PARTIAL (independent, best-effort) | platform-wallet `manager/identity_sync.rs:76,397-437`; `wallet_lifecycle.rs:421` |
| Step 7 — retrieve platform data | ✅ IMPLEMENTED (independent timer) | platform-wallet `manager/dashpay_sync.rs:404-474` |
| **Steps 2–7 ordered in one phase** | ⚪ NOT MODELED (intentional) | `manager/mod.rs:103-195` — no cross-coordinator gating |
| Step 8 — compact-filter build (confirmed path) | ✅ IMPLEMENTED | `sync/filters/manager.rs:654,734,779` |
| Step 8 — **DashPay/contact addresses in filter** | ✅ IMPLEMENTED (once receival acct exists) | `key-wallet .../wallet_info_interface.rs:302-316`; reg `network/contacts.rs:223,233` |
| Step 8 — filter set on **all** peers | 🟡 PARTIAL (eventually-all, looped not atomic; BIP37 mempool only) | `sync/mempool/sync_manager.rs:173-193`; `network/mod.rs:174-176` |
| Step 9 — sync-from block / wallet-birthday checkpoint | ✅ capability present; birthday auto-drive soft | `chain/checkpoints.rs:138-145`; `sync/filters/manager.rs:171-175` |
| Block-locator shape ("last 10 + prev checkpoint + genesis") | 🟡 PARTIAL (single-hash, checkpoint-segmented) | `network/mod.rs:102-107`; `sync/block_headers/segment_state.rs:67-69` |
| Named 4-phase state machine | 🟡 PARTIAL (generic `SyncState`, no named phases) | `sync/progress.rs:9-18` |

No `todo!`/`unimplemented!`/stub markers were found in the masternode/quorum or
header/filter sync paths — the hard cryptographic parts are real.

### 6.2 DIP-16 deviations (audit findings — mostly intentional, none are dead stubs)

1. **No 4-phase ordering; L2 sync decoupled from L1.** Identity/platform sync run on
   independent timers with zero gating on SPV header/masternode completion. Notably,
   platform-data **proof verification does not consume the local SPV quorum state** —
   `SpvRuntime::get_quorum_public_key` exists but no sync manager calls it; proofs go
   through the SDK/DAPI path. This is the largest DIP-16 conformance gap, but appears
   to be a deliberate UX choice (don't block L2 on full L1 sync).
2. **Single-hash block locator** instead of the DIP's multi-hash fork-recovery
   locator. Safe under checkpoint-segmented parallel download (each segment anchor is
   a validated checkpoint/tip), but a literal non-conformance with no genesis/previous
   fallback hashes in a request.
3. **Height aggregation is `max`, not soft-consensus** — one dishonest peer
   advertising a high `start_height` inflates the sync target. Minor, but worth a note.
4. **BIP37 mempool filter is set per-peer in a loop**, not an atomic broadcast.
5. **Birthday-by-timestamp start is available but not obviously auto-driven** from
   platform-wallet (`get_sync_checkpoint(creation_time)` exists; default start resumes
   from persisted `synced_height`/config height).
6. **DashPay address coverage is conditional** — addresses are watched only *after*
   the contact's funds-bearing receival account is registered; there is no pre-emptive
   watch. This is the DIP-16-layer facet of the §1.1 gap (below).

### 6.3 DIP-16 does NOT mandate the §12.6 rescan — confirmed

Direct fetch of DIP-16 confirms it specifies **no** "re-request blocks from height N
after the address set grows" mechanism. Its filter section says only that Platform-app
address spaces "can be used" in the filter and "a client should set this filter on all
connected peers." The rewind-on-new-address behavior is a **DIP-15 §12.6** obligation
layered on the DIP-16 base — so §1.1 is a DIP-15 gap, not a DIP-16 one.

### 6.4 The rescan engine already exists at the DIP-16 filter layer

Relevant to §1.1: dash-spv's filter manager **already implements** the rescan
machinery — `reset_for_rescan()` rolls `committed_height` back and replays when a
wallet's `synced_height` drops below scan progress, and an in-flight `rescan_batch`
re-scans when new gap-limit scripts appear mid-batch
(`sync/filters/manager.rs:129-139,468-505`). It is just never *triggered* for the
DashPay backfill case, because nothing lowers `synced_height` to the contact's
`$coreHeightCreatedAt`. That is why §1.1's fix is a small wallet-side trigger plus one
upstream guard-bypass method, not an SPV build.

---

## 7. Recommended priority

1. **§1.1 coreHeight block re-scan (DIP-15 §12.6)** — the only untracked
   correctness/payment-loss item. Now scoped small: a wallet-side `synced_height`
   rewind on new-contact registration + one upstream `reset_wallet_synced_height_to`
   method; the dash-spv `FiltersManager` rescan engine already does the rest.
2. **§1.2 account-label** — ✅ DONE. Send length-normalization fixed; receive-side
   decryption + UI surfacing implemented (incoming-only) per
   SPEC.md Milestone 3. DIP-15 §8.5 now fully conforms.
3. **DIP-16 deviations (§6.2)** — mostly intentional; if any is worth hardening it is
   #1 (consider sourcing proof-verification quorum keys from the local SPV engine) and
   #3 (height soft-consensus). Track, don't rush.
4. Everything in §2 stays blocked on its external dependency; §3 is intentional.
