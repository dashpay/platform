# DashPay — Implementation Spec & Gap Analysis

> **Purpose.** A single working spec for getting the **full DashPay flow** — sync,
> create/update profile, send contact request, approve/reject contact requests,
> send money to a contact — *done and tested* in the **platform wallet**
> (`rs-platform-wallet` + FFI) and surfaced as a **nice UI** in the
> **SwiftExampleApp**.
>
> **Status (2026-06-10).** DashPay is **already ~80% implemented end-to-end.** This
> document maps the protocol, inventories what exists, isolates the gaps & bugs,
> and lays out the remaining work + test plan. It is *not* a greenfield design —
> it is a finish-and-polish plan.
>
> **Status update (2026-06-18) — the finish-and-polish work is essentially done
> on `feat/dashpay-m1-sync-correctness` (PR #3841).** Resolution of the Part-0 gap
> table: **G1, G2, G12, G13, G14, G15** (the P0 sync/wire/key-purpose blockers) and
> **G3, G6, G7, G8, G9, G10** — all **DONE** (M1–M4). **G5** reworked and shipped as
> a per-sender, reversible, **local-only Ignore** (Spec 2) across every layer incl.
> the SQLite persister; cross-device sync deferred to a future encrypted `profile`
> field (contract track — the `contactInfo` route was rejected for the R1 leak).
> **G11**: the `network/` layer now has unit coverage; the live cross-client e2e
> ride PR #3549 and stay blocked on devnet funding. **G4** (watch-only ECDH) is
> **deferred** with an amended design (needs xpub hooks, not just an ECDH hook).
> Three follow-on specs were written and **implemented** this pass:
> **`SYNC_CORRECTNESS_SPEC.md`** (Spec 0 — paginated/high-water sync + contact-profile
> cache + durable persistence), **`CONTACTINFO_FORMAT_SPEC.md`** (Spec 1 — privateData
> CBOR→DIP-15 varint), and Spec 2 (Ignore). Also resolved: `accountReference`
> byte-order (**keep ours** — recipient-ignored one-time pad, no interop break) and
> the friendship-path `account'` hardcode (fixed upstream in **rust-dashcore#813**,
> pulled in via the dashcore bump **PR #3936**). Remaining is all blocked on external
> resources (devnet funding for e2e/UAT; contract governance for cross-device ignore
> + DoS filter; an upstream rust-dashcore change for multi-account). See
> the DashPay backlog issue [#4020](https://github.com/dashpay/platform/issues/4020) for the authoritative item-by-item status.
>
> **How to read.** Part 0 is the TL;DR. Parts 1–2 are reference (protocol +
> architecture). Part 3 is the current-state inventory. Part 4 is the prioritized
> gap/bug list. Part 5 is the work plan. Part 6 is the Swift UI design. Part 7 is
> the test plan. The durable evidence base ships alongside this spec:
> [`INTEROP_DESK_CHECK.md`](./INTEROP_DESK_CHECK.md) (cross-client wire-format
> verdicts + testnet census) and [`CONTACTINFO_FORMAT_SPEC.md`](./CONTACTINFO_FORMAT_SPEC.md)
> (Appendix A). The transient working-research files that once backed the
> remaining citations were trimmed from the tree; they remain in this branch's
> git history (`docs/dashpay/research/`, up to the trim commit).

---

## Part 0 — Executive summary

### What works today (end-to-end, real broadcast)

- **Profile**: create / update / fetch / sync — `rs-platform-wallet`
  (`network/profile.rs`), FFI, and Swift `DashPayProfileEditorView` are all wired
  and broadcast real document state transitions.
- **Send contact request**: builds the `contactRequest` doc, ECDH + AES-256-CBC
  encrypts the receiving xpub (via `dash-sdk` → `platform-encryption`), signs,
  broadcasts. Wrapped in Swift (`AddFriendView`). ⚠ **Correction (2026-06-10):
  "works" was overstated — the G2 entropy bug meant every broadcast through this
  path was rejected by consensus until the M1 task-4 fix. The code path existed;
  it did not function. (Exactly what G11's zero-network-tests predicted.)**
- **Accept contact request**: sends the reciprocal request, decrypts the
  contact's xpub, registers a watch-only sending account. Wired in Swift
  (`ContactRequestRow` Accept).
- **Send money to a contact**: derives the next contact address, builds + signs +
  broadcasts an L1 tx, records the payment. Wired in Swift
  (`SendDashPayPaymentSheet`).
- **DIP-14 / DIP-15 derivation**: 256-bit non-hardened child derivation, the
  `m/9'/5'/15'/0'/<owner256>/<counterparty256>` friendship path, the two account
  types (`DashpayReceivingFunds`, `DashpayExternalAccount`), gap limit 20 — all
  implemented and test-vector-pinned in `rust-dashcore/key-wallet`.
- **Persistence**: contacts / profiles / payments round-trip through the
  changeset → SQLite pipeline. SwiftData mirror models exist.
- **Swift FFI coverage**: all ~14 DashPay/DPNS FFI functions are already wrapped
  on `ManagedPlatformWallet`.

### The gaps that block a *complete, correct* flow (detail in Part 4)

| # | Gap | Severity | Layer |
|---|-----|----------|-------|
| G1 | **Sync never builds sending accounts** — a contact who accepts you *while you're offline* has no spendable account after sync; `send_payment` fails until `register_external_contact_account` is manually called | **P0** | `rs-platform-wallet` |
| G2 | **`send_contact_request` entropy mismatch** — document-ID entropy diverges from the broadcast entropy (`rs-sdk` admits the "simplification"); severity needs verification against `PutDocument` | **P0** | `rs-sdk` |
| G3 | **`accountReference` hardcoded to 0** — DIP-15 masking unused; the unique index `(ownerId,toUserId,accountReference)` makes key rotation / re-send impossible | **P1** | `rs-platform-wallet` |
| G4 | **Watch-only wallets can't send/accept** — ECDH is derived from the in-process seed; only `EcdhProvider::SdkSide` is used, the `ClientSide` push-across-FFI path is unbuilt | **P1** | wallet + FFI |
| G5 | **Reject is local-only** — no tombstone at all (recurring sync would resurrect rejects — stage-1 fix in **M1**) and no `contactInfo` `displayHidden` doc (cross-device — stage 2 in **M3**) | **P1** | wallet + SDK |
| G6 | **Wrong fallback contract ID** — `rs-sdk` `#[cfg(not(feature="dashpay-contract"))]` path hardcodes the **DPNS** id (dead code in default builds, latent bug) | **P2** | `rs-sdk` |
| G7 | **Dead code**: `calculate_account_reference`, `validate_contact_request`, auto-accept proof gen/verify — implemented + tested but never called by live paths | **P2** | `rs-platform-wallet` |
| G8 | **Local sent-request placeholder** — stores `vec![0u8;96]` for `encrypted_public_key` instead of the real ciphertext | **P2** | `rs-platform-wallet` |
| G9 | **No contract cache** — the bundled system contract is re-loaded on every op | **P2** | `rs-platform-wallet` |
| G10 | **No `contactInfo` support** — alias/note/hidden private metadata never syncs across devices | **P2** | wallet + SDK |
| G11 | **Network layer is untested.** Primitives/state/persistence are well covered, but the *whole* `network/` layer (send/sync/accept/pay/profile-broadcast) has **0 tests**; no full send→sync→accept→pay integration test; Swift has **0** DashPay tests | **P0** | both |
| G12 | **DashPay sync is not in the recurring sync loop.** The background `IdentitySyncManager` syncs **token balances only**; `dashpay_sync()` (contact requests + profiles) runs **only on-demand via FFI** — it must be folded into the recurring loop alongside the other syncs | **P0** | `rs-platform-wallet` |
| G13 | **Sync never reconciles own sent requests** — after restore-from-seed or on a second device an established contact renders as a mere incoming request; Accept re-broadcasts a duplicate reciprocal and is **rejected forever** by the unique index | **P1** | `rs-platform-wallet` |
| G14 | **Wrong encrypted-xpub wire format** (desk-check 2026-06-10, `INTEROP_DESK_CHECK.md`): we encrypt the 107-byte DIP-14 `ExtendedPubKey::encode()` instead of DIP-15's **69-byte compact** (`fingerprint‖chaincode‖pubkey`) used by iOS+Android → our send fails its own 96-byte check; our receive can't parse mobile payloads | **P0** | `platform-encryption` + `rs-sdk` + wallet |
| G15 | **Key-purpose convention mismatch**: mobile clients use key 0 (AUTHENTICATION) for both key indices; our send/validation require ENCRYPTION/DECRYPTION-purpose keys → cross-client requests blocked both directions. Verify against a real testnet mobile contactRequest, then align | **P1** | wallet + `rs-sdk` |

### UI verdict

The Swift DashPay UI exists but is **buried** (Identities → IdentityDetail →
`Section("DashPay")`) and **utilitarian**. The plan (Part 6) **promotes it to a
first-class `DashPay` tab**, renders the missing outgoing-requests section, moves
lists onto reactive `@Query`, and polishes styling (AsyncImage avatars, empty
states, toasts). No new happy-path FFI is required.

### Recommended sequencing

**Milestone 1 (correctness)**: G11-seam, G12, G1+G13 (+G5 tombstone), G2, interop
desk-check, G11-Rust. → the offline-accept→pay path works, is integration-tested,
and the background sync is wired.
**Milestone 2 (UI)**: first-class DashPay tab + polish + Swift tests (Part 6/7).
**Milestone 3 (spec-completeness)**: G3, G5, G10 (accountReference + contactInfo
for rotation/hide/alias sync).
**Milestone 4 (hardening)**: G4 (watch-only ECDH), G6–G9 cleanup.
**Milestone 5 (invitations)**: asset-lock voucher + claim + auto-accept wiring
(new scope 2026-06-10; design pass first).

---

## Part 1 — What DashPay is, and the layered stack

**DashPay** (DIP-0015) is a Dash Platform application that creates *bidirectional
direct settlement payment channels* between two Dash **identities**. User-facing
model:

- **Username** → a **DPNS** name (DIP-0012) resolving to an identity. DashPay
  itself never stores usernames; it references identities by their 32-byte id.
- **Identity** (DIP-0011) → the cryptographic actor; holds keys + credit balance;
  signs all state transitions.
- **Profile** → public presentation (`displayName`, `publicMessage`, avatar).
- **Contact / friend** → an identity you have exchanged `contactRequest`
  documents with **in both directions**.
- **Pay a contact** → decrypt the xpub from *their* contactRequest addressed to
  you, derive the next L1 address, and pay it with an ordinary Dash transaction.
  DashPay is the *key-sharing / coordination* layer; value transfer is plain L1.

### The implementation stack (bottom → top)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ SwiftExampleApp        Views: FriendsView, IdentityDetailView (profile),   │
│ (packages/swift-sdk/   SendDashPayPaymentSheet, AddFriendView   [Part 6]   │
│  SwiftExampleApp)      State: PlatformWalletManager, AppState, SwiftData    │
├──────────────────────────────────────────────────────────────────────────┤
│ swift-sdk wrappers     ManagedPlatformWallet.{sync,send,accept,reject,pay,  │
│ (Sources/SwiftDashSDK) profile…}, ContactRequest, EstablishedContact,       │
│                        DashPayProfile, KeychainSigner   (thin, marshal-only)│
├──────────────────────────────────────────────────────────────────────────┤
│ rs-platform-wallet-ffi C ABI: platform_wallet_{sync_contact_requests,       │
│                        send_contact_request_with_signer, accept…, reject…,  │
│                        send_dashpay_payment, *_dashpay_profile_with_signer}  │
├──────────────────────────────────────────────────────────────────────────┤
│ rs-platform-wallet     IdentityWallet (network façade): contact_requests.rs,│
│ (the brains)           contacts.rs, payments.rs, profile.rs, dashpay_sync.rs│
│                        ManagedIdentity state; crypto/{dip14,validation,…}   │
├───────────────────────────────┬──────────────────────────────────────────┤
│ rs-sdk (dash-sdk)             │ platform-encryption                        │
│  dashpay/contact_request.rs:  │  derive_shared_key_ecdh (libsecp256k1 ECDH),│
│  create/send_contact_request, │  encrypt/decrypt_extended_public_key        │
│  EcdhProvider, queries        │  (AES-256-CBC + PKCS7), account-label crypto │
├───────────────────────────────┴──────────────────────────────────────────┤
│ rust-dashcore/key-wallet      DIP-9 paths, DIP-14 256-bit CKD, AccountType  │
│ (HD wallet primitives)        ::Dashpay{ReceivingFunds,ExternalAccount},    │
│                               managed accounts, gap limit 20, tx checking   │
├──────────────────────────────────────────────────────────────────────────┤
│ dashpay-contract              v1 schema: profile / contactRequest /         │
│                               contactInfo; id Bwr4WHCP…NS1C7                 │
└──────────────────────────────────────────────────────────────────────────┘
```

Key architectural facts:
- **ECDH + AES live in `platform-encryption`**, *not* in `key-wallet`
  (rust-dashcore has **zero** ECDH code). The wallet calls `dash-sdk`'s
  `send_contact_request`, which calls `platform-encryption` internally; the
  receive/decrypt path calls `platform-encryption` directly.
- **`key-wallet` only consumes an already-decrypted friend xpub**
  (`wallet_add_dashpay_external_account_with_xpub_bytes`).
- **The DashPay system contract is bundled** (`load_system_data_contract`), no
  network fetch needed.
- **Swift never orchestrates** — every multi-step DashPay op is *one*
  `platform-wallet` FFI call (per `swift-sdk/CLAUDE.md`).

---

## Part 2 — Protocol reference (authoritative numbers)

Condensed from the DIPs themselves (DIP-9/11/13/14/15,
cross-checked against the deployed v1 contract). **Where DIP prose and the v1
schema disagree, the schema wins.**

### 2.1 Friendship lifecycle

- A `contactRequest{ $ownerId: sender, toUserId: recipient }` is **one-directional**.
- **Established contact (DIP-15: "friendship") = both directions exist** (A→B *and*
  B→A). One request = "pending"; the reciprocal = "accept".
- **To pay X**, read **X's** request addressed to you (`$ownerId==X`,
  `toUserId==you`), decrypt its `encryptedPublicKey`, derive addresses.
- Contact requests are **immutable & non-deletable** (`documentsMutable:false`,
  `canBeDeleted:false`). Key rotation = a **new** request with a bumped
  `accountReference` version.

### 2.2 Friendship derivation path (DIP-15 + DIP-14)

```
m / 9' / 5' / 15' / 0' / <ownerIdentity256> / <counterpartyIdentity256> / index
   └─────── hardened ──────┘   └── non-hardened 256-bit (DIP-14) ──┘     └ non-hardened u32
```

- `9'`=feature purpose, `5'`=Dash (`1'` testnet), `15'`=DashPay, `0'`=account.
- The two 256-bit levels are the raw 32-byte identity ids (owner first). **Must**
  stay non-hardened (so a watch-only xpub at `…/0'` covers all contacts) and full
  256-bit (truncating to 31 bits is a DIP-14 security violation).
- Auto-accept proof keys use a **separate** path `m/9'/5'/16'/<timestamp>'`.

### 2.3 ECDH shared secret

libsecp256k1 ECDH (**not** raw X-coord): `sharedKey = SHA256( ((y[31]&1)|2) || x )`
of the shared point `d_self · Q_other`. The participating identity keys are
selected by `senderKeyIndex` / `recipientKeyIndex` (identity public-key `id`s,
encryption/decryption purpose). Both parties derive the identical 32-byte key →
the AES-256 key.

### 2.4 Encryption layout

- Plaintext = **compact** xpub `parentFingerprint(4) || chainCode(32) ||
  pubKey(33)` = **69 bytes** (not the 78-byte BIP32 xpub). *(Implementation note —
  corrected by the 2026-06-10 desk-check: our stack actually fed
  `ExtendedPubKey::encode()` in, which for the DashPay path is the **107-byte**
  DIP-14 form → 128-byte ciphertext → our own send path failed. See **G14**;
  reference clients confirm the 69-byte compact form.)*
- `encryptedPublicKey` = `IV(16) || AES-256-CBC-PKCS7(80)` = **exactly 96 bytes**.
- `encryptedAccountLabel` = `IV(16) || ciphertext(32–64)` = **48–80 bytes**.
- `contactInfo.privateData` uses **BIP32-derived** symmetric keys (self-encrypt),
  not ECDH; `encToUserId` uses AES-ECB.

### 2.5 `accountReference`

```
ASK        = HMAC-SHA256(senderSecretKey, extendedPublicKey)
AccountRef = (Version << 28) | (ASK[28 msb]  XOR  (Account & 0x0FFFFFFF))
```
Top 4 bits = version (rotation signal), low 28 bits = account number masked by a
PRF of the xpub. Uniqueness not required. The recipient un-masks the account and
reads the version.

### 2.6 DashPay v1 contract document types

Contract id **`Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7`**
(hex `a2a1…71bc`), owner all-zero. Full field/index tables in the deployed
schema, `packages/dashpay-contract/schema/v1/dashpay.schema.json`. Summary:

- **`profile`**: `avatarUrl`(uri,≤2048), `avatarHash`(32B), `avatarFingerprint`(8B),
  `publicMessage`(1–140), `displayName`(1–25). Avatar trio is `dependentRequired`.
  Unique index `$ownerId`; non-unique `$ownerId+$updatedAt`. Mutable.
- **`contactRequest`**: `toUserId`(32B id), `encryptedPublicKey`(**exactly 96B**),
  `senderKeyIndex`, `recipientKeyIndex`, `accountReference`, optional
  `encryptedAccountLabel`(48–80B), optional `autoAcceptProof`(38–102B, unencrypted).
  Required system fields incl. `$createdAtCoreBlockHeight`. Unique index
  `$ownerId+toUserId+accountReference`; timelines `toUserId+$createdAt` (received)
  and `$ownerId+$createdAt` (sent). Immutable.
- **`contactInfo`**: `encToUserId`(32B), `rootEncryptionKeyIndex`,
  `derivationEncryptionKeyIndex`, `privateData`(48–2048B encrypted; **DIP-15
  varint** `version`/`aliasName`/`note`/`displayHidden`/`acceptedAccounts` —
  contract enforces length only, see `CONTACTINFO_FORMAT_SPEC.md`). Unique index
  `$ownerId+root+derivation`. Privacy rule: don't publish until ≥2 established
  contacts.

---

## Part 3 — Current implementation state (inventory)

Master status matrix. **Legend:** ✅ implemented · 🟡 partial/caveated · ❌ missing.
Citations are abbreviated (file:line against this branch).

### 3.1 rust-dashcore `key-wallet` (HD primitives)

| Capability | Status | Evidence |
|---|---|---|
| DIP-9 DashPay root `m/9'/5'/15'` (`/1'` testnet) | ✅ | `key-wallet/src/dip9.rs:167-198` |
| DIP-14 256-bit non-hardened CKD (priv+pub) | ✅ | `bip32.rs:575-598,1533-1589,1817+`; vectors `:2521-2594` |
| `AccountType::DashpayReceivingFunds` / `DashpayExternalAccount` | ✅ | `account/account_type.rs:76-95,469-514` |
| Managed accounts, single pool, **gap limit 20** | ✅ | `managed_account_type.rs:97-118,706-749` |
| Tx checking routes contact funds | ✅ | `transaction_checking/account_checker.rs:501-518` |
| FFI: add receiving / add external(xpub) / get | ✅ | `key-wallet-ffi/src/wallet.rs:397,451`, `managed_account.rs:436,497` |
| ECDH / shared secret / xpub encryption | ❌ (by design — lives in `platform-encryption`) | repo-wide grep: none |
| Auto-create DashPay accounts at wallet init | ❌ (per-contact, after the fact) | `wallet/initialization.rs` |
| Match result carries identity ids | 🟡 (only `account_index`; reverse-lookup needed) | `account_checker.rs:144-153` |

### 3.2 `platform-encryption` + `rs-sdk` (crypto + send flow)

| Capability | Status | Evidence |
|---|---|---|
| `derive_shared_key_ecdh` (libsecp256k1) | ✅ | `rs-platform-encryption/src/lib.rs:24-34` |
| `encrypt/decrypt_extended_public_key` (AES-256-CBC, IV-prepend, 96B) | ✅ | `lib.rs:97-128` |
| `encrypt/decrypt_account_label` (48–80B) | ✅ | `lib.rs:139-171` |
| `Sdk::create_contact_request` / `send_contact_request` | ✅ | `rs-sdk/src/platform/dashpay/contact_request.rs:164,378` |
| `EcdhProvider::{ClientSide, SdkSide}` | ✅ (both defined; only SdkSide used upstream) | `contact_request.rs:31-54` |
| Queries: sent / received / all contact requests | ✅ | `contact_request_queries.rs:33,76` |
| SDK helpers for `profile` / `contactInfo` | ❌ (done via generic `Document`+`PutDocument`) | — |
| **Bug: send entropy ≠ doc-id entropy** | 🟡 **G2** | `contact_request.rs:431-435` (code comment admits it) |
| **Bug: fallback contract id = DPNS id** | 🟡 **G6** (dead in default build) | `dashpay/mod.rs:33` |

### 3.3 `rs-platform-wallet` (+ FFI, storage) — the brains

| Flow | Status | Evidence |
|---|---|---|
| Identity ↔ wallet (managed identities) | ✅ | `state/managed_identity/mod.rs:37`, `network/identity_handle.rs:256` |
| Profile fetch / sync | ✅ | `network/profile.rs:64,145` |
| Profile create / update (external signer) | ✅ | `network/profile.rs:240,395` |
| `dashpay_sync` aggregator | ✅ | `network/dashpay_sync.rs:16` |
| Sync received contact requests | 🟡 **G1** (ingest guard drops reciprocals; no xpub-decrypt / no external account built) | `network/contact_requests.rs:322,367-372` |
| Sync own sent requests (restore/multi-device reconcile) | ❌ **G13** | sync calls `fetch_received_contact_requests` only |
| Send contact request (seed-in-process) | ✅ / 🟡 **G4** | `network/contact_requests.rs:91` |
| Accept (reciprocal send + register external account) | ✅ | `network/contact_requests.rs:466` |
| Reject | 🟡 **G5** (local-only) | `network/contact_requests.rs:678` |
| Auto-establish on reciprocal match | ✅ | `state/managed_identity/contact_requests.rs` |
| Register receiving / external account | ✅ (UAT 2026-06-12: now also **persisted** — registrations were in-memory only, so accounts vanished on relaunch and restored friendship UTXOs were dropped `dropped_no_account`) | `network/contacts.rs` |
| Send money to contact | ✅ | `network/payments.rs:93` |
| Record incoming payment | ✅ (UAT 2026-06-12: the old `try_record_incoming_payment` had **zero callers** — receiver history was always empty. Replaced by live recording in the wallet-event adapter + an idempotent `reconcile_incoming_payments` step in the recurring sync) | `network/payments.rs`, `changeset/core_bridge.rs` |
| Crypto: DIP-14 xpub / payment addrs | ✅ | `crypto/dip14.rs` |
| Crypto: `accountReference` | 🟡 **G3/G7** (correct but unused; send hardcodes 0) | `crypto/dip14.rs:147` |
| Crypto: auto-accept proof | 🟡 **G7** (dead code, `// TODO` at `auto_accept.rs:39`) | `crypto/auto_accept.rs` |
| Pre-send validation | 🟡 **G7** (never called) | `crypto/validation.rs:76` |
| Persistence round-trip | ✅ | `wallet/apply.rs`, storage `schema/{contacts,dashpay}.rs` |
| Local placeholder `encrypted_public_key` | 🟡 **G8** (`vec![0u8;96]`) | `network/contact_requests.rs:283` |
| Contract cache | 🟡 **G9** (re-load per call) | `network/profile.rs:83` |
| FFI surface (sync/send/accept/reject/pay/profile) | ✅ | `ffi/src/{dashpay,dashpay_profile,contact_request,established_contact,contact}.rs` |

> **No `todo!()`/`unimplemented!()`/`unreachable!()` anywhere in DashPay paths** —
> all gaps are caveats, dead helpers, or local-only fallbacks, not panics.

### 3.4 SwiftExampleApp + swift-sdk

| Capability | Status | Evidence |
|---|---|---|
| All ~14 DashPay/DPNS FFI functions wrapped | ✅ | `Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift:1452-1779` |
| Wrapper objects: `ContactRequest`, `EstablishedContact`, `DashPayProfile` | ✅ | same dir |
| SwiftData mirrors: `PersistentDashpayProfile`, `PersistentDashpayContactRequest` | ✅ | `Persistence/Models/` |
| Contacts list + incoming requests + accept/reject | ✅ (utilitarian) | `Views/FriendsView.swift` |
| Add friend by DPNS name / identity id | ✅ | `FriendsView.swift` (`AddFriendView`) |
| Send money to contact sheet | ✅ (most polished) | `FriendsView.swift` (`SendDashPayPaymentSheet`) |
| Profile view / editor (DIP-15 avatar hashing) | ✅ | `Views/IdentityDetailView.swift:332,1169` |
| First-class DashPay tab | ❌ **(Part 6)** buried under Identities | `ContentView.swift` |
| Outgoing requests rendered | ❌ (loaded, not shown) | `FriendsView.swift` |
| Lists driven by reactive `@Query` | ❌ (reads live Rust snapshot) | `FriendsView.swift` |
| DashPay tests (unit / XCUITest) | ❌ **G11** | `SwiftTests/`, `SwiftExampleAppUITests/` |

---

## Part 4 — Gap analysis & bugs (prioritized)

### P0 — blocks a correct, complete flow

**G1 — Sync cannot establish contacts, and never builds sending accounts.**
Two compounding defects in `sync_contact_requests` (`network/contact_requests.rs:322`):
(1) **the ingest guard drops reciprocal requests** — any received doc whose sender is
already in `sent_contact_requests` is skipped (`:367-372`), so in the offline-accept
scenario the reciprocal request never reaches `add_incoming_contact_request` (the
only auto-establish trigger) and the contact stays pending-sent forever; (2) even
for contacts that do establish, sync stores the *encrypted* `encryptedPublicKey` and
never decrypts it or registers a `DashpayExternalAccount` — only the explicit
**accept** path does. **Consequence:** "they accepted me → I sync → I pay them"
fails twice over: the contact never establishes via sync, and `send_payment`
(`network/payments.rs:135`) has no sending account. **Fix:** (a) relax the ingest
guard so a received doc whose sender matches a `sent_contact_requests` entry flows
into `add_incoming_contact_request` (which auto-establishes and collapses the
pending entries); (b) on every sync pass, for **every established contact missing an
external account** (not only newly-established ones — this also repairs contacts
left unpayable by the accept path's best-effort registration), validate the
request's key indices via `validate_contact_request` (purpose ENCRYPTION/DECRYPTION
+ ECDSA key type — never ECDH against an unvalidated index; an attacker-crafted
index pointing at an AUTHENTICATION key silently derives a wrong shared secret and
poisons the account), then decrypt the xpub and register the account — and likewise
register a missing **`DashpayReceivingFunds`** account (derivable from the wallet's
own seed, no decryption needed): it is what makes *incoming* contact payments
visible to SPV, its only creation point today is the fresh-send path
(`contact_requests.rs:300`), and after restore-from-seed nothing rebuilds it —
incoming payments would land on unwatched addresses; (c) **failure
policy** — distinguish transient failures (network: retry next sweep) from permanent
ones (decrypt/decode failure: mark the contact "payment channel broken", surface to
FFI/UI, skip until the request changes — no unbounded retry). Must be seed-aware
(skip + log for watch-only until G4).

**G2 — `send_contact_request` entropy mismatch.**
`rs-sdk/.../contact_request.rs:431-435`: `create_contact_request` computes the
document id from entropy E1, but `send_contact_request` generates *fresh* entropy
E2 for `put_to_platform_and_wait_for_response`. The code comment admits the
"simplification". **Action:** verify whether `PutDocument` re-derives the id from
E2 (in which case the returned `ContactRequestResult.id` is merely *stale*, a
correctness wart) or whether the broadcast actually fails / duplicates. Thread the
*same* entropy through both. Pin with a test asserting `result.id == on-platform id`.

**G11 — Test coverage (precise breakdown).**
What's **well covered** (≈60 unit tests): crypto (`crypto/dip14.rs` ×10,
`validation.rs` ×8, `auto_accept.rs` ×6), the contact state machine
(`state/managed_identity/contact_requests.rs` ×12, `mod.rs` ×8), the DashPay types
(`established_contact`/`profile`/`contact_request`/`payment`), and persistence
(`wallet/apply.rs` ×26). Plus `tests/contact_workflow_tests.rs` (8 tests) — but
those are **pure in-memory handshake** tests using `noop_persister()` and **fake
identities** (`data: vec![1u8;33]`, not real keys), so they exercise the state
machine, *not* real ECDH/derivation/broadcast.

What's **completely untested**: the entire **`network/` layer** — the actual
broadcast/sync/pay paths. `grep #[test] src/wallet/identity/network/` → only
`registration.rs` has one. **Zero** tests for
`send_contact_request_with_external_signer`, `sync_contact_requests`,
`sync_profiles`, `accept_contact_request_with_external_signer`,
`register_external_contact_account`, `send_payment`, `create/update_profile`. And
**zero** DashPay tests in Swift. This is a quality-P0: the flow cannot be declared
"done and tested" without (a) network-layer tests via a mock SDK/broadcaster seam,
and (b) a real devnet/regtest end-to-end test. (Test plan: Part 7.)

**G12 — DashPay sync is not in the recurring sync loop.**
The background `IdentitySyncManager` (`manager/identity_sync.rs`, owned by
`PlatformWalletManager` at `manager/mod.rs:54,139`, run as a cancel-token loop with
a configurable interval and a re-entrancy guard) syncs **token balances only**.
`dashpay_sync()` (= `sync_contact_requests()` + `sync_profiles()`) is invoked
**only on demand via FFI** — i.e. the Swift app must poll it. There is **no
recurring DashPay refresh**. **Fix (the constraints matter more than the
placement):** `dashpay_sync()` is a method on `IdentityWallet` (needs the
wallet-manager lock, per-wallet persister, broadcaster), while `IdentitySyncManager`
is deliberately self-contained (constructed with sdk + persister only; documented as
not reaching into PlatformWallet/WalletManager) and its registry **skips identities
with empty token lists** — so the recurring DashPay pass must **NOT** be driven "per
registered identity" off the token registry (a DashPay-only identity with no watched
tokens would never sync). Instead, inject the wallets map (the same
`Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>` that
`PlatformAddressSyncManager` already receives at `manager/mod.rs:135-138`; snapshot
the wallet `Arc`s under a read guard per sweep) and iterate wallets calling
`wallet.identity().dashpay_sync()` per pass, reusing the existing
cadence/cancel/quiesce/re-entrancy machinery. Whether this lives inside
`IdentitySyncManager` or as a sibling `DashPaySyncManager` is an implementation
detail; coupling DashPay sync to the token registry is the failure mode to avoid.
**Error semantics:** log-and-continue per wallet/identity (matching the existing
loop's contract) — never fail-fast across identities. Note: wrapping `dashpay_sync`
alone only delivers per-*wallet* continue — `sync_contact_requests` currently
`?`-aborts its multi-identity loop on the first fetch error and `dashpay_sync`
propagates immediately, so the per-identity policy must be implemented *inside*
those loops. This recurring pass is the
natural home for the **G1** establish/decrypt/register sweep, the **G13** sent-side
reconcile, and the **G5** tombstone check. Keep the on-demand FFI entry points for
pull-to-refresh.

### P1 — spec-correctness / production-readiness

**G3 — `accountReference` hardcoded to 0.** The send path uses
`account_reference = 0` instead of `calculate_account_reference(...)`
(`crypto/dip14.rs:147`). Because the unique index is
`(ownerId, toUserId, accountReference)`, a second request to the same recipient
(key rotation, multi-account) **collides** and is rejected by Platform. Today this
"works" only because the full account xpub is shared directly (recipient decrypts
it and doesn't need to un-mask the account). **Fix:** wire
`calculate_account_reference` into the send path; add the version-bump path for
rotation; have the receive path tolerate / surface non-zero versions (DIP-15 §7.3
"sender rotated their addresses" notification). **Receive-side scope note (budget
into M3):** the in-memory maps, changeset keys, and SQLite contacts schema are
keyed by counterparty id alone, and the sync ingest guard skips requests from
already-established contacts — surfacing rotation requires re-keying
contact-request state + persistence by `(counterparty, accountReference)` and
letting rotation requests from established contacts through the guard. Without
this, `dp_005`'s "receive path surfaces the rotation" assertion is unimplementable.

**G4 — Watch-only wallets can't send/accept.** ECDH is derived from the
in-process seed (`identity_handle.rs:424`); only `EcdhProvider::SdkSide` is used.
For hardware/watch-only wallets, the `ClientSide` path (host supplies the shared
secret) must be pushed across the FFI. **Fix:** add an FFI/signer hook that returns
the ECDH shared secret for `(senderKeyIndex, recipientPubKey)` from the secure
element, and route `send_contact_request` / `register_external_contact_account`
through `EcdhProvider::ClientSide`. *(The example app holds the seed, so this is
not a demo blocker — but it is the right architecture.)* **FFI-hook design lands in
M3** (design-only task) so the wallet API doesn't churn in M4: the hook accepts
only the 32-byte ECDH **shared secret** from the host — never the sender's identity
private key across the ABI (the existing `rs-sdk-ffi`
`DashSDKContactRequestParams.sender_private_key` field is the antipattern to avoid,
and worth auditing in its own right).

**G5 — Reject is local-only, and the recurring loop will undo it.**
`reject_contact_request` (`network/contact_requests.rs:678`, `// TODO` at `:703`)
drops the local entry but writes no tombstone of any kind — and the still-on-platform
immutable document is re-ingested as a fresh incoming request on the next sync.
Today this is masked because sync is on-demand only; **the moment G12 lands, every
background sweep resurrects every rejected request on the same device.** **Fix (two
stages):** **M1 (with G12):** a locally-persisted rejected-request tombstone
consulted by the sync ingest path — keyed by **document id (or
`(sender, accountReference)`)**, NOT bare sender id: requests are immutable, so the
only legitimate way a once-rejected sender can ever re-request is a new doc with a
bumped `accountReference` (the rotation mechanism), and a sender-keyed tombstone
would silently block that forever with no un-reject affordance. Pinning test:
"rejected request does not reappear after a recurring re-sync — and a
bumped-`accountReference` request from the same sender *does*". **M3:** the
on-platform `contactInfo` `displayHidden` write (see G10) for cross-device sync.

**G13 — Sync never reconciles your own sent requests.** Sync only calls
`fetch_received_contact_requests`; the identity's own sent requests are never
fetched into state (the `fetch_sent_contact_requests` query exists but is
read-only). After restore-from-seed or on a second device, a mutually-established
contact renders as a mere incoming request; tapping Accept re-broadcasts a duplicate
reciprocal with the same `(ownerId, toUserId, accountReference)` triple, which
Platform rejects on the unique index — **Accept fails forever with no recovery
path**. **Fix (M1):** sync also fetches the identity's own sent contactRequest
documents and ingests them via `add_sent_contact_request` (auto-establish fires when
both sides are present) — **with a sent-side ingest guard symmetric to the received
side**: skip docs whose recipient is already in `sent_contact_requests` or
`established_contacts` (`add_sent_contact_request` has no such guard today, so an
unguarded recurring re-ingest creates phantom pending-sent rows + a changeset write
per contact per sweep). Any (re-)establish path must **merge into an existing
`EstablishedContact`** — `EstablishedContact::new` resets alias/note/is_hidden/
accepted_accounts, so naive re-establish wipes user metadata every sweep. Accept
detects an existing on-platform reciprocal and adopts it instead of re-broadcasting;
"adopt" includes the local registrations a fresh send performs (receiving-account
registration, G1(b)) and runs the same `validate_contact_request` gate before any
`register_external_contact_account` call — the gate applies to **all three paths**
(sync sweep, normal Accept, Accept-adopt). *Pin:* "established contact is stable
and metadata-preserving across two recurring sweeps".

**G14 — Wrong encrypted-xpub wire format (P0; found by the M1 desk-check,
`INTEROP_DESK_CHECK.md`).** DIP-15 and BOTH reference clients
(iOS dash-shared-core `ecdsa_key.rs:333-341`, Android dashj
`serializeContactPub()` with a hard `len == 69` receive check) use the compact
**69-byte** plaintext `parentFingerprint(4) ‖ chainCode(32) ‖ pubKey(33)`. Our
stack fed `ExtendedPubKey::encode()` into `encrypt_extended_public_key` — and the
DashPay account xpub ends in a `Normal256` child, so that's the **107-byte
DIP-14** serialization → 128-byte ciphertext → fails our own `== 96` assertion
and the contract's `maxItems: 96`. **Consequence:** our send path errored before
broadcast (nothing nonconforming reached chain — blast radius ≈ zero), and our
receive (`ExtendedPubKey::decode`, 78/107 only) rejects every mobile payload and
would mark the channel permanently broken. **Fix (M1 task 7):** compact 69-byte
assembly on send + compact parser on receive (path context reconstructs
depth/child-number); byte-exact vectors from the reference clients.

**G15 — Key-purpose convention mismatch (P1; same desk-check).** Mobile clients
populate `senderKeyIndex`/`recipientKeyIndex` with key id 0 — an
**AUTHENTICATION**-purpose ECDSA key — while we *send* selecting
ENCRYPTION/DECRYPTION-purpose keys and *validate* (G1(b)) requiring those
purposes. Cross-client requests would be blocked in both directions. **Action
(M1 task 8):** verify empirically against a real testnet mobile contactRequest,
then align — liberal-on-receive (accept the purposes mobile actually uses; keep
the ECDSA key-*type* gate), compatible-on-send (fall back to the mobile
convention when the recipient lacks a DECRYPTION-purpose key).

### P2 — cleanup / completeness

- **G6 — Wrong fallback contract id** (relabeled P2 — dead code in default builds):
  `rs-sdk/.../dashpay/mod.rs:33` hardcodes the **DPNS** id under
  `#[cfg(not(feature="dashpay-contract"))]` — a latent foot-gun. **Fix:** correct
  the constant to the DashPay id `Bwr4WHCP…NS1C7` or delete the fallback.

- **G7 — Dead code:** wire `validate_contact_request` into the send path (replace
  the ad-hoc `find(...)`) — note the *receive/sync* side of this validator is pulled
  forward into **M1** by G1(b); decide whether to ship auto-accept (then call the
  proof gen/verify) or delete it and its FFI param. **Acceptance criterion if
  shipped:** any handler acting on `autoAcceptProof` MUST call
  `verify_auto_accept_proof` before triggering automatic acceptance — sync stores
  the blob unverified today, so wiring auto-accept without the gate lets forged
  38–102-byte blobs auto-establish attacker contacts.
- **G8 — Local placeholder:** store the real 96-byte ciphertext on the local sent
  `ContactRequest` (`contact_requests.rs:283`) so the persisted/SwiftData row
  matches Platform.
- **G9 — Contract cache:** hold one `Arc<DataContract>` on the wallet instead of
  re-loading the bundled contract per op.
- **G10 — `contactInfo` support:** add SDK + wallet + FFI for the `contactInfo`
  document (self-encrypted alias/note/displayHidden/acceptedAccounts) so contact
  metadata and hides sync across a user's devices. Respect the "≥2 contacts before
  publishing" privacy rule.
- **Compact-xpub note:** DIP-15 specifies a 69-byte compact xpub plaintext;
  `platform-encryption` currently encrypts a 78-byte serialization (still 96 bytes
  out). Both sides of *this* implementation agree, so it interoperates with itself;
  verify against the reference DashPay clients (iOS/Android) before declaring
  cross-client compatibility. **Sequencing:** the desk-check (compare reference
  client code or captured vectors) is cheap and lands in **M1** (task 5) — a wrong
  wire format must be caught before three milestones of tests harden it; the live
  cross-client e2e stays in M4.

---

## Part 5 — Work plan (what to build, per milestone)

Each task notes the layer and the **test** that proves it (TDD: write the failing
test first — see Part 7 and the repo's TDD discipline).

### Milestone 1 — Correctness (the flow actually completes)

Ordered so the test seam exists before the TDD-gated tasks that need it.

1. **G11-seam: make the network layer testable.** The **fetch half needs no new
   seam** — use the SDK's built-in mock (`SdkBuilder::new_mock` +
   `expect_fetch`/`expect_fetch_many`, already used by `identity_sync.rs` tests)
   for the sync/establish tests below. The **put/broadcast half** cannot hide
   behind a dyn trait (`Sdk::send_contact_request` is generic over 7 type params):
   define ONE object-safe trait exposing only the concrete operations
   `IdentityWallet` performs (send contact request with SdkSide ECDH, put profile
   document), held as a new `IdentityWallet` field defaulting to an
   `Arc<Sdk>`-backed impl so public construction and FFI are untouched.
   (`send_payment` already takes an injected `broadcaster: B`.)
   **DONE (2026-06-10; revised 2026-07-04):** originally shipped as a
   Send-boxed `#[async_trait]` `DashPaySdkWriter` trait; the trait was later
   removed as an unused seam (no test ever injected it — the sync/establish
   tests mock the fetch half via `SdkBuilder::new_mock` instead).
   `network/sdk_writer.rs` now ships a concrete `SdkWriter` held as an
   `Arc<Sdk>`-backed `IdentityWallet` field: it still erases the
   7-type-param `send_contact_request` / `PutDocument` generics behind two
   concrete methods, it just isn't swappable.
2. **G12: fold DashPay sync into the recurring loop.** Per G12: inject the wallets
   map and iterate wallets calling `dashpay_sync()` — do **not** drive off the
   token registry; log-and-continue error semantics; keep the on-demand FFI entry
   points for pull-to-refresh.
   - *Test:* a recurring pass drives DashPay sync for every wallet **including
     identities with zero watched tokens**; re-entrancy + quiesce still hold.
   **DONE (2026-06-10):** sibling **`DashPaySyncManager`** (`manager/dashpay_sync.rs`,
   modeled on `PlatformAddressSyncManager`) — the in-struct option was rejected
   because `IdentitySyncManager` is registry-driven. Red→green pinned by
   `recurring_pass_syncs_every_wallet_including_zero_token_identities`; per-identity
   continue pushed into `sync_contact_requests`.
3. **G1 + G13 + G5-tombstone: sync establishes, reconciles, and builds accounts.**
   Relax the ingest guard (G1a); ingest own sent requests (G13); consult the
   persisted rejected-senders tombstone (G5 stage 1); then, for every established
   contact missing an external account: validate key indices → decrypt → register
   (G1b), with the transient/permanent failure policy (G1c). **Lock ordering:**
   collect candidates while the wallet-manager write guard is held, drop the
   guard, then call `register_external_contact_account` — it re-acquires read
   locks on the same tokio `RwLock`, which is **non-reentrant**; calling it inline
   under the write guard deadlocks on first execution (mirror the accept path's
   guard-drop ordering — `network/contact_requests.rs:466` drops the write guard
   before calling `register_external_contact_account`).
   - *Tests:* offline-accept→pay (Part 7, `dp_004`); rejected request does NOT
     reappear after a recurring re-sync; restore-from-seed then Accept does not
     double-broadcast (G13); permanent decrypt failure marks the contact unpayable
     and stops retrying.
   **DONE (2026-06-10):** tombstone keyed `(owner, sender, accountReference)`
   (new `rejected_contact_requests` SQLite table + `ContactChangeSet.rejected`);
   broken channel = `EstablishedContact.payment_channel_broken` (new `contacts`
   column + FFI accessor `established_contact_is_payment_channel_broken`);
   metadata-preserving `reestablish_preserving_metadata()`; sweep candidates
   collected under the write guard, registered after guard drop. 192 tests green.
   *Deviations:* (1) tests pin the decision logic + state machine; the full
   mock-SDK offline-accept→pay and accept-adopt flows live in `dp_004`/`dp_005`
   on #3549 (per Part 7.4) — too heavy to stub as unit tests; (2) the Swift
   persister bridge does NOT yet project `rejected` / `payment_channel_broken` —
   added to M2 plumbing (task 8).
4. **G2: entropy threading.** Fix `rs-sdk` `send_contact_request` to reuse the
   creation entropy; assert returned id == on-platform id.
   - *Test:* `rs-sdk` unit/integration pinning id equality.
   **DONE (2026-06-10) — severity verdict: REAL BROADCAST BUG.** `put_document`
   uses the document as-is (E1-derived id) with the supplied fresh entropy E2;
   drive-abci consensus recomputes `generate_document_id_v0(…, E2)`, compares to
   `base.id`, and rejects with `InvalidDocumentTransitionIdError` — **every
   `send_contact_request` through this path failed at consensus**. Fix: additive
   `entropy: Bytes32` on `ContactRequestResult`, reused by send; pinned by
   `contact_request_result_entropy_derives_returned_id` (red = inexpressible
   pre-fix; green post-fix). 136 lib + all test targets green; FFI ABI unchanged.
5. **Interop desk-check (verify-only).** Compare the compact-xpub plaintext
   (69B DIP-15 vs 78B current), ECDH derivation, and accountReference masking
   against reference DashPay iOS/Android client code or captured vectors; record
   the result. A mismatch found here re-scopes M1 before tests harden the wrong
   format; the live cross-client e2e stays in M4.
   **DONE (2026-06-10)** — `INTEROP_DESK_CHECK.md`. Verdicts:
   xpub plaintext **FAIL** (→ new **G14**, task 7 below); ECDH **PASS**;
   accountReference **PASS-for-now** (mobile ignores it on receive; our masking
   helper has two latent bugs for M3 — 107-byte HMAC input + ASK28 byte order,
   where iOS and Android also disagree with *each other*). Bonus hazard → **G15**
   (key-purpose convention).
7. **G14: compact-xpub wire format (re-scoped into M1 by task 5).** Send: assemble
   the 69-byte compact plaintext (`parentFingerprint(4) ‖ chainCode(32) ‖
   compressedPubKey(33)`) from the already-derived contact xpub instead of
   `ExtendedPubKey::encode()`; receive: parse the 69-byte compact (both sides
   already know the derivation path, so depth/child-number are reconstructable).
   Pin with byte-exact vectors mirroring the reference clients (iOS
   `ecdsa_key.rs:333-341`, dashj `serializeContactPub()` — quoted in
   `INTEROP_DESK_CHECK.md`). 69 → PKCS7 → 80 ‖ IV 16 = exactly 96 bytes.
   **DONE (2026-06-10):** codec in `platform-encryption`
   (`compact_xpub_bytes`/`parse_compact_xpub`, `COMPACT_XPUB_LEN=69`);
   `ContactXpubData::compact_xpub()` + `reconstruct_contact_xpub` in
   `crypto/dip14.rs`; rs-sdk callback contract = "69-byte compact", validated
   pre-encryption; receive reconstructs from `chain_code`+`pubkey` (metadata
   depth/child synthesized — non-hardened CKD unaffected, pinned by
   `reconstructed_xpub_derives_identical_addresses`); legacy 78/107 fallback
   branch kept. 194 platform-wallet tests green; FFI ABI unchanged (caller doc
   contract tightened).
8. **G15: key-purpose verification (decision gate, cheap).** Fetch a real
   mobile-created `contactRequest` + its sender identity from testnet; inspect
   `senderKeyIndex`/`recipientKeyIndex` purposes. Then align: likely
   liberal-on-receive (accept ECDSA keys of the purposes mobile actually uses)
   + compatible-on-send (fall back to the mobile convention when the recipient
   has no DECRYPTION-purpose key). Implementation in M1 if the verification
   confirms the mismatch; the validation gate from G1(b) stays for key *type*.
   **VERIFIED (2026-06-10, all 368 testnet contactRequests —
   `INTEROP_DESK_CHECK.md` §G15):** the "key 0 AUTHENTICATION" desk-check reading was
   stale. Dominant mobile cohort (223 docs): **unbound ENCRYPTION/MEDIUM key
   (id 2) for BOTH indices** (recipientKeyIndex → ENCRYPTION — mobile identities
   carry no DECRYPTION key); 2026 cohort (68 docs): contract-bound ENC(4)/DEC(5)
   — our convention. Consensus enforces neither purpose nor boundedness on these
   fields. **Alignment (task 9):** send — prefer recipient DECRYPTION, fall back
   to recipient ENCRYPTION; receive — accept ENCRYPTION for sender,
   ENC-or-DEC for recipient; keep the ECDSA type gate; purpose mismatch alone
   never marks a channel permanently broken. No AUTHENTICATION fallback.
9. **G15 alignment implementation** (per the verified verdict above): relax the
   sender/recipient purpose assertions in `rs-sdk` `create_contact_request`
   (`:200-239`) and `rs-platform-wallet` key selection + `validate_contact_request`
   wiring; tests for the mobile-cohort shape (ENC/ENC, unbound) and our own
   (ENC/DEC, bound).
   **DONE (2026-06-10):** recipient selection prefers DECRYPTION, falls back to
   ENCRYPTION (ECDSA gate kept, no AUTH fallback); validation gained a recipient
   purpose gate (AUTH was silently accepted before!) + `purpose_mismatch` flag;
   purpose mismatches log-and-skip, never `payment_channel_broken`; ECDH decrypt
   path confirmed index-generic (pinned). 204 platform-wallet + 139 dash-sdk
   tests green. **M1 complete** (task 6 e2e rides #3549, non-gating).
6. **G11-Rust: full-cycle e2e confirmation** (`dp_003`): `profile → send → sync →
   accept → established → pay`, on live testnet via the #3549 bank harness.
   **Not M1-exit-gating** (Part 7.4): M1 exits on tasks 1–5; this task is tracked
   on #3549 and lands when the framework does.

**coreHeight backfill rescan (DIP-15 §8.7/§12.6) — DONE (2026-06-24).** Surfaced by
the re-audit after M1's original plan: an incoming payment to a contact's receival
address that landed *before* the address was watched (restore-from-seed, second
device, or the offline-accept→pay window) was silently missed. Fixed by (a) re-import
scanning from birth-height `Some(0)` (`cba515aaf1`) so on-chain history isn't skipped,
and (b) `reconcile_dashpay_rescan` (`18483e4232`, a local-only step of `dashpay_sync`
in `manager/dashpay_sync.rs`) that lowers the wallet's SPV `synced_height` toward
`min($coreHeightCreatedAt)` over established receival contacts so dash-spv's filter
manager re-matches the now-watched addresses against blocks it already scanned. It uses
the inner unconditional height setter (no upstream change) with a per-contact
`dashpay_rescan_triggered` one-shot guard so the recurring sweep doesn't re-lower the
height every pass. The regression is safe (`synced_height` is the filter-scan
checkpoint, decoupled from the monotonic `last_processed_height`); the floor is clamped
to the engine's header/birth floor, and the one SPV caveat is that a rewind into a
never-stored filter range is retried silently every 30s rather than erroring.

### Milestone 2 — Swift UI (first-class, polished) + Swift tests

See Part 6 for the screen design. Tasks:

> **STATUS (2026-06-10): tasks 7–10 DONE** (Phases A–D on
> `feat/dashpay-m1-sync-correctness`). Delivered: FFI sync-control surface
> (`platform_wallet_manager_dashpay_sync_{start,stop,is_running,is_syncing,
> last_sync_unix_seconds,set_interval,sync_now}`), persister payload extended
> (callback arity 8→10: `payment_channel_broken` on `ContactRequestFFI`,
> `ContactRequestRejectionFFI` tombstones), payment-history getter
> (`managed_identity_get_dashpay_payments`); Swift SDK wrappers +
> `PersistentDashpayPayment` + `@Published dashPaySyncIsSyncing`; the full
> DashPay tab (`Views/DashPay/` — 7 files) with all §6.4 states and
> `dashpay.*` accessibility ids; simulator BUILD SUCCEEDED.
> **Spec deltas accepted:** (1) alias/note/hide are a UserDefaults-backed
> device-local store until M3's `contactInfo` (no SwiftData model added);
> (2) contact DPNS labels captured as an add-time hint (not persisted
> elsewhere); (3) AddContact ID-mode preview is cache-only (no
> fetch-profile-by-id FFI). Task 11 (tests) = Phase D.
> **Task 11 DONE (2026-06-10):** 15 SDK unit tests (persister bridge: broken-flag
> on both rows, tombstone scoped to `(owner,sender,accountReference)` w/ rotation
> survival, 10-arg C-callback round-trip, payment upserts, FFI marshalling) + 2
> UI smoke tests (§6.4 picker states; passed on-simulator). Phase D also found &
> fixed a changeset-atomicity defect (`persistDashpayPayments` missing the
> `!inChangeset` guard — red→green). Totals: swift test 29/29; app tests 237
> passed / 18 pre-existing network-gated skips; UI smoke green; BUILD SUCCEEDED.
> Full add→approve→pay XCUITest = documented TODO gated on funded testnet
> identities (tracks `dp_003`).

7. Add `RootTab.dashpay` + `DashPayTabView` with an active-identity picker
   (`ContentView.swift`, `SwiftExampleAppApp.swift`) — picker states per §6.4.
8. Extract/rebuild `ContactsView`, `ContactRequestsView` (incoming **+ outgoing**),
   `AddContactView`, `ContactDetailView`, `ProfileView`/editor, reusing the
   already-wrapped `ManagedPlatformWallet` methods; implement the §6.4 interaction
   states (DPNS resolution states, send-collision flow — **AddContactView only**,
   not the payment sheet — in-flight rows). Payment
   history requires the persister mapping + `PersistentDashpayPayment` model (§6
   intro). Additional persister-bridge plumbing from M1: project the
   `rejected` tombstones and `payment_channel_broken` flag into SwiftData (the
   Rust SQLite pipeline already persists them; the Swift `on_persist_contacts_fn`
   bridge does not yet).
9. Move lists onto `@Query [PersistentDashpayContactRequest]` /
   `[PersistentDashpayProfile]` with the §6.4 optimistic-overlay policy; refresh
   via `syncContactRequests()` + `syncDashPayProfiles()` in `.task` /
   pull-to-refresh, coordinated through the §6.4 single sync-in-progress signal
   (requires M1 task 2 — the three-caller invariant can't be exercised until the
   G12 background loop exists). **Realtime cadence (per §6.4):** the tab drives the
   background loop's interval to 4s on foreground / 15s on background via
   `setDashPaySyncInterval` (NavigationStack `onAppear`/`onDisappear`), and every
   local mutation (send/accept/QR-send/pay) fires a non-blocking `kickDashPaySync`
   so the counterparty side converges without waiting for the next tick.
10. Polish: AsyncImage avatars w/ initial-circle fallback, empty states, loading &
    error states, inline success feedback (§6.4), accessibility identifiers on
    every interactive control (for XCUITest).
11. **G11-Swift:** unit tests (wrapper round-trips) + XCUITest (add→approve→pay).
    (Part 7.)

### Milestone 3 — Spec completeness (rotation, hide, alias sync)

12. **G3:** wire `calculate_account_reference` + version bump into send;
    receive-path version handling + "addresses rotated" surfacing — **includes the
    receive-side re-keying** of contact-request state/persistence by
    `(counterparty, accountReference)` (see G3 scope note).
13. **G10 + G5 stage 2:** `contactInfo` document support (SDK + wallet + FFI) →
    cross-device reject/hide + alias/note sync.

    **DONE (2026-06-12), 4 commits:** crypto core (DIP-15 derivation
    `root/65536'+65537'/idx'`, AES-256-ECB encToUserId, IV‖CBC privateData;
    the `privateData` plaintext was initially a CBOR array but is being
    migrated to the **DIP-15 varint** format — the contract enforces length
    only, so it's a free convention; see `CONTACTINFO_FORMAT_SPEC.md` /
    Spec 1), stateless doc↔contact resolution (decrypt every owned doc's
    encToUserId), sync step 3 of the recurring pass, publish with the
    DIP-15 ≥2-contacts privacy gate (deferred publishes update local state
    only), FFI `platform_wallet_set_dashpay_contact_info_with_signer`,
    persister round-trip (alias/note/hidden on the established rows, both
    directions), and **contact restore at load** (new contacts array on
    `IdentityRestoreEntryFFI`) — without which the re-establish sweep wiped
    metadata during the deferred-publish window and contacts were invisible
    on offline launches. Verified on-sim: alias save → relaunch → survives
    and renders.
14. Swift UI for alias/note edit (reuse `EditAliasView`) now backed by
    `contactInfo` — remove the M2 "This device only" labels.

    **DONE (2026-06-12):** ContactDetailView reads alias/note/hidden off
    the `@Query` contact rows and writes through
    `ManagedPlatformWallet.setDashPayContactInfo`; ContactsView hidden
    filter + alias display moved off the UserDefaults meta store (which
    now only keeps the add-time DPNS hint); labels updated.

**Receive-side `encryptedAccountLabel` surfacing (DIP-15 §8.5) — DONE (2026-06-24).**
The send side already length-normalizes the label; the receive side now decrypts and
shows it. Decrypted in Rust at the two signer-bearing register sites (the drain
`RegisterExternal` Ok-branch + `accept_register_external_validated`, where the ECDH
`shared` key lives) by `store_contact_account_label` (`network/contact_requests.rs`),
stored on the derived `EstablishedContact.contact_account_label` field (reset in `new`,
`reestablish_preserving_metadata`, and `apply_rotated_incoming_request` so it never
goes stale). It is surfaced **incoming-only** — it is the contact's label for *their*
account, so it is derived strictly from the incoming request and projected onto the
incoming FFI row only (the outgoing row's label is one *we* sent and is never shown),
via `contact_persistence.rs`. Decrypt failures / garbage / control-chars sanitize to
`None` (cosmetic — never breaks the channel), and it resets on rotation. Renders as a
read-only "Their account" row through Swift `contactAccountLabel` → `ContactDetailView`.

15. **G4 design-only:** specify the FFI ECDH hook (shared-secret-only across the
    ABI — never a raw private key; see G4) so M4's implementation doesn't churn
    the wallet API.

    **DONE (2026-06-12) — design:**
    - **ABI surface (one new callback on the existing host-signer table** —
      the same registration path external-signable wallets already use for
      transaction signing**):**
      ```c
      int32_t (*ecdh_shared_secret_fn)(
          void *context,
          const uint8_t (*wallet_id)[32],
          const uint8_t (*identity_id)[32],
          uint32_t key_id,                      // sender's encryption key id
          const uint8_t (*counterparty_pubkey)[33],
          uint8_t (*out_shared_secret)[32]);    // SHA256((y&1|2)||x) — finished secret
      ```
      The host derives the identity encryption private key for
      `(identity_id, key_id)` from its keychain/secure element and computes
      the **finished DIP-15 shared secret host-side**. The private key never
      crosses the ABI (the `rs-sdk-ffi` `DashSDKContactRequestParams.
      sender_private_key` field is the antipattern this replaces; flagged for
      its own audit). Non-zero return = "host cannot produce the secret"
      (locked keychain, missing key): the operation fails with a typed
      `EcdhUnavailable` error and is NOT treated as a broken payment channel
      (our side failed, not the contact's request).
    - **Rust routing:** `send_contact_request` /
      `register_external_contact_account` branch on wallet key-residency:
      seed-resident wallets keep today's in-process derivation
      (`EcdhProvider::SdkSide`); external-signable wallets route
      `EcdhProvider::ClientSide { get_shared_secret }` where the closure
      calls the FFI hook. No public wallet-API signature changes — the
      provider choice is internal, which is what de-risks M4.
    - **Zeroization:** Rust wipes `out_shared_secret` (`Zeroizing`) after
      deriving the AES key; hosts are instructed to do the same with their
      intermediate private key (Swift: `withUnsafeTemporaryAllocation` +
      explicit reset, mirroring the signer callback's key handling).
    - **Same hook serves decrypt-side** (`register_external_contact_account`
      needs ECDH with the *contact's* pubkey at OUR key id) — the
      `counterparty_pubkey` parameter covers both directions; no second
      callback needed.

### Milestone 4 — Hardening / cleanup

16. **G4:** watch-only ECDH via `EcdhProvider::ClientSide` pushed across FFI
    (implements the M3 design).

    **DEFERRED with design amendment (2026-06-13):** implementation scoping
    found the M3 hook (ECDH shared secret only) is **insufficient** for true
    watch-only DashPay: the friendship-xpub derivations are hardened and
    seed-bound on BOTH flows — send derives the sender↔recipient receiving
    xpub (`m/9'/coin'/15'/account'/<us><them>`, recipient-dependent so not
    pre-derivable), and accept derives our receiving xpub for the new
    account. A watch-only host therefore needs **three** hooks: ECDH shared
    secret (designed in M3), friendship-xpub derivation, and
    receiving-account-xpub derivation — or one combined
    "derive-DashPay-context" hook returning `(compact_xpub, shared_secret)`.
    The contactInfo self-encryption keys (M3 task 13) are seed-bound the
    same way and need a fourth surface (or ride the combined hook).
    Since the example app attaches the seed at launch (this gap is
    explicitly not a demo blocker), shipping the ECDH-only ABI change would
    add churn without enabling any watch-only flow. Revisit as its own
    design+implementation slice when a hardware/watch-only host exists.
17. **G6:** fix/delete fallback contract id.

    **DONE (2026-06-13):** fallback corrected from the DPNS id to the
    deployed DashPay id.
18. **G7:** wire send-path validation; ship-or-delete auto-accept (verify-gate
    acceptance criterion applies if shipped — see G7).

    **DONE (send half, 2026-06-13):** the selected key pair gates through
    `validate_contact_request` before any ECDH/broadcast. Auto-accept:
    decision = **keep dormant** — it activates with M5 invitations behind
    the `verify_auto_accept_proof` hard gate (per Part 8.5), not deleted.
19. **G8/G9:** real local ciphertext; contract cache.

    **DONE (2026-06-13):** sent rows store the real 96-byte ciphertext off
    the broadcast document; the bundled DashPay contract is cached
    process-wide (OnceLock) replacing five per-call re-parses.
20. Live cross-client interop e2e (compact xpub, ECDH, accountReference) vs
    reference DashPay clients (the M1 desk-check verified the formats on paper).

    **BLOCKED-EXTERNAL (2026-06-13):** requires driving real DashWallet
    iOS/Android builds against a shared network — not runnable in this
    environment. The M1 desk-check (`INTEROP_DESK_CHECK.md`) + on-chain census remain
    the interop evidence; the contactInfo research (`CONTACTINFO_FORMAT_SPEC.md` Appendix A) found no
    reference client implements contactInfo at all, shrinking the live-e2e
    surface to contactRequest + payment addresses. Run manually when a
    mobile test build is available.

### Milestone 5 — Invitations (new scope, 2026-06-10; needs its own design pass)

Onboard users who don't have Dash yet: inviter creates an asset-lock-funded
credit voucher + link (DIP-13 invitation subfeature, `m/9'/5'/5'/3'`); invitee
claims it → identity created from the voucher → invitee's contact request to the
inviter carries an `autoAcceptProof` (path `m/9'/5'/16'/timestamp'`, helpers
already implemented in `crypto/auto_accept.rs`) → auto-established contact after
`verify_auto_accept_proof` (hard gate, see G7/Part 8.5). Scope before
implementation: a research+design slice (invitation create/claim wallet flows,
deep-link format, expiry/revocation, UI) — the platform wallet has the asset-lock
and identity-registration machinery to build on but no invitation flows today.

---

## Part 6 — Swift UI design (the "nice UI")

**Decision: promote DashPay to a first-class tab (Option B).** Lower-risk Option A
(polish in place under Identities) is the fallback if tab real estate is contested,
but a "nice DashPay UI" wants its own home. All screens reuse already-wrapped
`ManagedPlatformWallet` FFI — **no new network FFI**; the one new plumbing item is
**payment history** (map the Rust `dashpay_payments` changeset overlay in the Swift
persister into a new `PersistentDashpayPayment` SwiftData model + `@Query` — today
no FFI exposes `PaymentEntry` and no SwiftData model exists for it).

### 6.1 Navigation

Add `case dashpay` to `RootTab` (`ContentView.swift`), between `identities` and
`contracts`. Tab icon `person.2.fill`, title "DashPay".

```
DashPayTabView (NavigationStack)
├─ Active-identity picker (top) — most DashPay UIs assume one active identity;
│     menu of the wallet's managed identities (DPNS name → truncated id).
├─ Profile header card  → tap → ProfileView / ProfileEditorView
│     (empty state → "Set up your DashPay profile" CTA → ProfileEditorView)
├─ Username prompt card  → "Register a username" → RegisterNameView
│     (shown only when an on-chain DPNS check confirms the active identity
│      has no name; explains that without a username people can't find you
│      by name, and that the profile display name is cosmetic, not searchable)
├─ Segmented control:  [ Contacts | Requests ]
│   ├─ Contacts  → ContactsView (@Query established)
│   │     row tap → ContactDetailView → "Send Dash" / alias / note / hide
│   └─ Requests  → ContactRequestsView
│         ├─ Incoming (Accept / Reject)
│         └─ Outgoing (pending — NEW, currently unrendered)
└─ Toolbar:  + (AddContactView)   ·   refresh (sync)
```

Wire DashPay sync into the `.task` of `DashPayTabView` and/or the existing global
`GlobalSyncIndicator`: run `syncContactRequests()` then `syncDashPayProfiles()`.

### 6.2 Screens

**ProfileView / ProfileEditorView** (promote from `IdentityDetailView`)
- View: large avatar (`AsyncImage` w/ initial-circle fallback), `displayName`,
  DPNS handle, `publicMessage`. "Edit" button. Empty state → "Set up your DashPay
  profile" CTA (opens `ProfileEditorView` as a sheet — same target as "Edit").
- Editor: `Form` with `displayName` (≤25), `publicMessage` (≤140), `avatarUrl`;
  live char counters; on save fetch avatar bytes for DIP-15 hash/fingerprint; call
  `createDashPayProfile` / `updateDashPayProfile(…signer:)`.

**Username prompt** (`usernamePromptCard`, below the profile header)
- A second setup CTA, independent of the profile one: shown when the active
  identity has **no DPNS username** (confirmed by an on-chain `dpnsGetUsername`
  check in `.task` + on app-foreground, so an identity that already has one —
  just not yet cached, or registered meanwhile on another device — is never
  nagged; mirrors `IdentitiesView`'s lazy fetch. A found name is persisted and
  saved; a definitive empty result shows the card; a thrown error retries.
  Residual: a name registered on another device *while the user sits on this
  tab* clears on the next tab switch / app-foreground). Tap → `RegisterNameView`.
  Copy makes the username-vs-profile distinction explicit: a **username** is the
  searchable handle people type to add you (contact search); the profile
  **display name** is cosmetic and not searchable. On registration the Rust path
  persists `dpnsName`, so the prompt hides reactively via `@Query`.

**ContactsView**
- `@Query` established contacts (joined to `PersistentDashpayProfile` for
  display). Row = avatar + (alias → displayName → DPNS → truncated id) + last
  payment hint. Search bar. Pull-to-refresh = sync. Empty state → "Add your first
  contact".

**ContactRequestsView** (the **new outgoing section** is the headline UI gap)
- **Incoming**: row + Accept (`borderedProminent`) / Reject (`.tint(.red)`),
  relative timestamp, sender profile. On accept → success toast + move to Contacts.
- **Outgoing**: pending sent requests (`fetchSentContactRequests` /
  `getSentContactRequestIds`), "Pending" badge, sent timestamp. Currently loaded
  but never shown — render it.

**AddContactView** (restyle `AddFriendView`)
- Segmented: **Username (DPNS)** | **Identity ID**. DPNS mode: live prefix search
  (`searchDpnsNames`) with result rows (avatar + name); ID mode: paste + validate
  base58. Resolve → preview the target profile → "Send request" → `sendContactRequest`.

**ContactDetailView**
- Profile header; **Send Dash** (presents the polished `SendDashPayPaymentSheet`);
  payment history (from `PaymentEntry` via the `PersistentDashpayPayment` mapping —
  see §6 intro); editable **alias** / **note** and **Hide** toggle, each labeled
  **"This device only"** in M2 (until M3's `contactInfo` backing replaces the
  label) so users don't assume sync semantics that don't exist yet.

**SendDashPayPaymentSheet** (already polished — restyle only)
- Amount in DASH→duffs, spendable balance, over-spend block, recipient
  profile/avatar/DPNS, result txid. (Memo is local-only; keep the field hidden or
  label it "private note" until on-chain memo exists.)
- Zero-balance state: when spendable balance is 0 (after the async load), disable
  the amount field + Send and show "Your balance is 0 DASH — top up your wallet
  before sending." instead of an always-disabled interactive form.

### 6.3 Conventions (must match house style)

From `SwiftExampleApp/CLAUDE.md`:
- `@EnvironmentObject var walletManager: PlatformWalletManager`,
  `var appState: AppState`; `@Environment(\.modelContext)`.
- Lists via `@Query` on `Persistent*` (move off the live-snapshot read).
- Async FFI: `Task { @MainActor in … }`, `defer { isLoading = false }`, resolve
  wallet via `walletManager.wallet(for:)`, fresh `KeychainSigner` per submit,
  `errorMessage` red caption on catch.
- `Form`/`Section` for editors, `List`/`Section` with count headers for lists.
- SF Symbols (`person.2`, `person.badge.plus`, `paperplane`, `pencil`,
  `person.crop.circle`), blue accent, red destructive, green success,
  `.borderedProminent` primaries.
- Amounts entered in DASH, converted to duffs (`× 100_000_000`).
- Display precedence: alias → DashPay `displayName` → DPNS → truncated hex.
- **`.accessibilityIdentifier(...)` on every interactive control** (needed for
  XCUITest) — e.g. `dashpay.tab`, `dashpay.addContact`, `dashpay.request.accept`,
  `dashpay.send.amount`, `dashpay.send.confirm`.
- **Never orchestrate in Swift** — one FFI call per DashPay op.

### 6.4 Interaction states & edge cases (normative for M2)

- **Identity picker** (tab root) — three states: (1) no wallet loaded → disabled
  "No wallet loaded" label + link to the Wallets tab; (2) wallet but zero
  identities → "No identities yet" + CTA to the Identities tab; (3) ≥1 identity →
  menu. Exactly one identity → auto-select and hide the picker. Selection persists
  across launches via `@AppStorage`.
- **AddContactView (DPNS mode)** — four states: typing → searching (inline
  `ProgressView`) → not-found (inline message + clear-and-retry affordance, never a
  dead end) → found (profile-preview card; "Send request" enabled only from this
  state). ID mode: inline base58 validation gates the send button. (The current
  `AddFriendView` dead-ends on "DPNS name not found".)
- **Send-collision flow** — if the target already has an incoming request to us,
  alert "This person already sent you a request — Accept it instead?" with
  Accept / Continue anyway. (Sending anyway is protocol-valid; it just establishes
  the contact.)
- **Request rows in flight** — on Accept/Reject tap, replace both buttons with a
  `ProgressView` for that row (prevents double-tap → duplicate accepts); on
  success remove the row optimistically; on failure restore the buttons + inline
  error on the row.
- **Optimistic overlay over `@Query`** — accept/reject/send mutate Rust state and
  the persister callback lands later; bridge the latency window with a local
  `@State` overlay set of affected ids filtering the `@Query` results, cleared
  when the query reflects the change. (The old `loadFriends()` re-read pattern is
  incompatible with pure `@Query` reactivity.)
- **Single sync-in-progress signal** — one `@Published` flag on
  `PlatformWalletManager` observed by all three sync callers (`.task`,
  pull-to-refresh, the G12 background loop); a pull-to-refresh during an in-flight
  sync attaches to it instead of double-firing.
- **Realtime cadence (foreground-fast / background-slow)** — the G12 background
  loop runs on a tunable interval (`setDashPaySyncInterval`, clamped ≥ 1s Rust-side;
  default = `backgroundSyncSeconds` = 15s). The DashPay tab drops it to
  `foregroundSyncSeconds` = 4s at *effective foreground* — the tab is on screen
  **and** the app is active — and restores 15s otherwise. "On screen" is driven
  from the tab's **NavigationStack** `onAppear`/`onDisappear` (so drilling into a
  contact detail or presenting a sheet, neither of which fires the stack's
  `onDisappear`, keeps the fast cadence; only a *tab switch* relaxes it); "app
  active" is driven from `scenePhase`, so backgrounding the app while on the tab
  also relaxes to 15s. The cadence acts only on transitions. This keeps neither an
  inactive tab nor a backgrounded app sweeping every few seconds, while incoming
  requests / acceptances / payments surface in near real time when the user is
  actually looking. **Entry kick:** `setDashPaySyncInterval` only takes effect on
  the loop's *next* sleep (it stores an atomic, no wakeup — `dashpay_sync.rs:157`),
  so entering the foreground also fires one `kickDashPaySync` — otherwise a tab
  re-entry could wait out a leftover up-to-15s sleep before the first fast tick.
  (A Rust-side `Notify` on `set_interval` would shorten the in-flight sleep
  directly; deferred as an internal refinement — the entry kick achieves the same
  user-visible result app-side.) Best-effort: a not-yet-configured manager keeps
  its current interval.
- **Post-mutation sync kick** — after a local mutation (send request, accept,
  send-via-QR, pay) the handler fires a non-blocking `dashPaySyncNow()`
  (`kickDashPaySync`) so the counterparty's state and the established pair converge
  promptly instead of waiting a full poll tick. Non-blocking: the sheet dismisses
  right away and the Rust manager folds an in-flight pass into a no-op (the single
  sync-in-progress signal above). *Bounded, not instant:* if a pass was already
  running when the mutation landed, the kick no-ops and convergence waits for the
  next tick (≤ the foreground 4s) rather than enqueuing a coalesced re-run.
  Complements, doesn't replace, the optimistic `@Query` overlay — the overlay
  covers the sender's own row, the kick pulls the other side.
- **Success feedback** — reuse the existing inline success pattern
  (`SendDashPayPaymentSheet`'s green inline text); no new toast component in M2
  (the app has no shared toast — only a clipboard `CopiedToast`).
- **Broken payment channel** (surfaces G1(c)) — ContactsView row shows a warning
  badge; ContactDetailView disables Send Dash with "Payment channel broken — ask
  the contact to send a new request" (re-enables when a new request arrives).
- **Needs-unlock / verify-failed banner** (seedless wallets) — **DONE (2026-06-23;
  `9963923e05` Rust+FFI, `841802c587` Swift).** A signerless sweep enqueues
  contact-crypto ops it can't finish while the Keychain is locked;
  `pending_contact_crypto_count` (`network/contact_requests.rs`) → FFI
  `platform_wallet_pending_contact_crypto_count` (`dashpay.rs`) feeds the ~1 Hz Swift
  poller into a per-wallet `DashPayUnlockStatus` / `@Published dashPayUnlockStatus`,
  rendered as a banner in `DashPayTabView.swift` (orange "N contact(s) waiting to
  finish setup" + Unlock, red on seed-mismatch). The count **excludes**
  `ContactInfoDecrypt` ops — those re-enqueue every sweep, so counting them would
  falsely re-trip the banner ~15s after every unlock; only the account-build ops
  (`RegisterReceiving`/`RegisterExternal`) converge to 0 once the payment account is
  built.
- **Profile save flow** — on save: disable Save + inline `ProgressView`; success →
  dismiss the editor sheet; failure → re-enable Save + red caption below the form.
- **Payment history list** — empty state "No payments yet"; loading = single
  inline `ProgressView`; error = keep last-known list + inline caption.

---

### Multi-reviewer code review (2026-06-14) — 8 findings fixed

Five specialized reviewers (crypto-security, FFI-memory, sync-correctness,
Swift/iOS, silent-failures) audited the M1–M4 diff. Crypto + FFI-memory
boundaries came back clean. Correctness/silent-failure reviewers found bugs
the live UAT had missed (UAT only hit the pending-sent rotation path, which
the reject-tombstone masked). All fixed with red→green regression tests:

- **P0** rotation re-send to an ESTABLISHED contact reset the version to 0
  → unique-index rejection → contact unrotatable. Lookup now consults
  `established_contacts.outgoing_request` (`prior_sent_account_reference`).
- **P0** multi-doc sweep thrash: immutable docs from a rotated sender both
  returned every sweep, flipping state + rebuilding the external account
  forever. `newest_received_per_sender` collapses to newest-per-sender
  before ingest; `apply_rotated` is idempotent.
- **Critical** swallowed persist errors → memory/disk divergence (reject
  resurrection). New `PlatformWalletError::Persistence`; reject + send_payment
  propagate, self-healing sweep writes log.
- **H1** Sent payments lost at relaunch (map restored empty) → new
  `PaymentRestoreEntryFFI` + `restore_dashpay_payments` fold + Swift builder.
- **H2** deferred-publish lied as "synced" → 3-state `ContactInfoPublishOutcome`
  through the FFI; ContactDetailView shows the real state.
- Med: zero-ciphertext fallback → hard Err; contactInfo derivation-index
  high-water mark; Swift silent contact-drop now logged; crypto
  account_index/accountReference invariant documented.

230/230 Rust lib + FFI tests green, clippy clean, full iOS build green.
NOTE: on-device re-verify of H1/H2 pending — the sim SwiftData store was
reset environmentally (identities gone), so it needs the devnet identity
setup rebuilt first.

### Devnet UAT round 2 (2026-06-13) — rotation / reject / DPNS verified live

On paloma with three identities: **reject + tombstone** (rejected request
suppressed across forced re-sync), **G3 rotation end-to-end** (re-send from
the rejected sender broadcast with a bumped accountReference — accepted by
the unique index — and reappeared through the tombstone on the recipient:
the dp_005 scenario, live), **DPNS register → live search → found preview →
send** and the **not-found state** (inline + retry, no dead end), **accept
of the rotation request** (re-established, accounts rebuilt). Findings
fixed: optimistic pending-sent overlay leaked across identity switches
(now reset on picker change). Open UX item: "SPV client is not running"
dead-ends both Send Dash and identity creation — needs auto-start or a
"Start & retry" affordance (product decision pending).

## Part 7 — Test plan

Follow the repo TDD discipline (failing test first; red→green in the commit
message). DashPay's correctness-critical pieces are the crypto and the
state-machine handshake — those get the deepest coverage.

### 7.1 Rust — `rs-platform-wallet` / `rs-sdk` / `platform-encryption`

Already covered (keep, don't duplicate): crypto round-trips, the contact
state-machine handshake (`tests/contact_workflow_tests.rs` + inline), and
persistence (`wallet/apply.rs`). See G11 for the inventory.

Unit — **the missing tier is the `network/` layer** (currently 0 tests). Add behind
a mock SDK/broadcaster seam:
- **Recurring sync (G12):** a recurring pass drives `dashpay_sync` for each wallet
  — including identities with zero watched tokens (see G12: do not couple to the
  token registry); re-entrancy guard + `quiesce` shutdown still hold; interval
  changes are picked up.
- **Sync builds external accounts (G1):** given an established contact with an
  encrypted xpub, the sync pass decrypts it and registers a `DashpayExternalAccount`
  (and skips gracefully for watch-only).
- **Crypto/derivation wiring:** `calculate_account_reference` is actually used by
  the send path (G3) and round-trips (un-mask recovers account + version).
- **State machine** (extend existing): idempotent re-sync; accept when both present;
  reject removes incoming.

Offline crypto/encode tier (rs-sdk, no network) — follow the existing
`packages/rs-sdk/tests/fetch/` harness with `--features mocks,offline-testing`
(`Config` from `tests/.env`, `mock::Mockable` + recorded vectors):
- **G2 (entropy):** after `create_contact_request`, the returned id matches the id
  derived from the *broadcast* entropy. Pin id equality.
- contact-request wire-shape: `encryptedPublicKey == 96B`, properties map matches
  the v1 schema, `accountReference` round-trips.

**E2E tier — build on the existing framework (PR #3549,
`packages/rs-platform-wallet/tests/e2e/`).** This is the canonical "how we do e2e"
for this crate (see [Part 7.4](#74-alignment-with-the-existing-e2e-framework)):
gated behind the **`e2e` cargo feature**, funded by the testnet **`bank` wallet**
harness (`framework/bank.rs` — `BankWallet::load`, `fund_address`,
`cross_check_balance`), config via `tests/.env`
(`PLATFORM_WALLET_E2E_BANK_MNEMONIC`), one file per case under
`tests/e2e/cases/<prefix>_NNN_*.rs` registered in `cases/mod.rs`, run with
`cargo test -p platform-wallet --test e2e --features e2e -- --nocapture`. Add a
**DashPay case family** (proposed prefix `dp_*`), modeled on the shielded `sh_*`
suite (PR #3727) which stacks the same way:

- **dp_001 (profile):** `create_profile` → fetch from Platform → fields match;
  `update_profile` bumps revision.
- **dp_002 (send request):** fund 2 bank-derived identities; A
  `send_contact_request(B)`; assert the on-platform `contactRequest`
  (`encryptedPublicKey==96B`, key indices, accountReference) + id equality (G2).
- **dp_003 (full cycle — the "done" gate):** A `send_contact_request(B)` → B
  recurring-sync sees incoming → B `accept` → **both** established → A
  `send_payment(B)` confirms on L1 → B records incoming.
- **dp_004 (offline accept → pay, pins G1+G12):** A sends → B accepts → A offline →
  A's **recurring sync** runs → A `send_payment(B)` **succeeds** (external account
  built during the sweep). *Must fail before the G1/G12 fix, pass after.*
- **dp_005 (rotation, pins G3):** second `send_contact_request` to the same
  recipient with a bumped version is accepted (distinct `accountReference`); the
  receive path surfaces the rotation.
- **dp_006 (recurring cadence):** the background recurring sync refreshes
  contacts/profiles without an explicit FFI call — including for an identity with
  zero watched tokens (assert via the bank harness over a couple of sweeps).
- **dp_006b (foreground-fast cadence + post-mutation kick, §6.4):** entering the
  DashPay tab lowers the sweep interval to 4s **and fires one immediate sweep**
  (because `set_interval` only applies on the loop's next sleep); leaving the tab
  *or backgrounding the app while on it* restores 15s (`setDashPaySyncInterval`
  round-trips through the FFI). A send/accept fires an extra `dashPaySyncNow()`
  that no-ops when a pass is already in flight. UI-level cadence + the
  scenePhase/tab-visibility state machine + the entry kick are covered by a manual
  two-sim e2e — these are SwiftUI-lifecycle/wall-clock timing properties the
  simulator harness can't assert deterministically; the FFI set-interval/sync-now
  round-trip is unit-tested Rust-side.

### 7.2 Swift — `SwiftTests` + `SwiftExampleAppUITests`

Unit (`SwiftTests/SwiftDashSDKTests/`):
- `ContactRequest(ffi:)`, `EstablishedContact`, `DashPayProfile(ffi:)` /
  `DashPayProfileUpdate` round-trips and marshalling (32-byte id in/out, optional
  C-strings, `_free` correctness, no leaks).
- `PersistentDashpay*` SwiftData upsert from the persister callback.

Flow (mirror the existing `PlatformWalletIntegrationTests.swift` harness, testnet):
- send → sync → accept → established → pay, asserting SwiftData rows + balances.

XCUITest (`SwiftExampleAppUITests/`, keyed on accessibility ids):
- Open DashPay tab → AddContact by DPNS → request appears in Outgoing → (peer
  accepts) → appears in Contacts → open contact → Send Dash → confirm txid.
- Use the `simulator-control` skill for SwiftData inspection + screenshots in UAT.

### 7.3 "Definition of done" per flow

| Flow | Done when |
|---|---|
| Create/update profile | `dp_001` + Swift editor XCUITest green; profile visible to peer |
| Send contact request | `dp_002` + G2 (entropy) offline test + AddContact XCUITest green |
| Approve request | `dp_003` accept step → both established; Accept XCUITest green |
| Reject request | local reject unit test green; (M3) `contactInfo` hide syncs across devices |
| Send money to contact | `dp_003` pay step + **`dp_004` (offline accept→pay)** + Send XCUITest green |
| Sync (recurring) | `dp_004`/`dp_006` build external account on the recurring sweep; idempotency unit test green |

### 7.4 Alignment with the existing e2e framework

The platform-wallet e2e framework **already exists but is unmerged** — PR
**#3549** (`feat/rs-platform-wallet-e2e`, draft). DashPay e2e cases must be authored
**on that branch** (or rebased onto it after it merges); they are not standalone.
Conventions to follow exactly (from `tests/e2e/README.md`):
- Modeled on `dash-evo-tool/tests/backend-e2e/`; runs against **live Dash testnet**
  (v3.0) via DAPI, gated behind the `e2e` cargo feature.
- Funding via the **platform-address `bank` wallet** (seed in
  `PLATFORM_WALLET_E2E_BANK_MNEMONIC` / `tests/.env`); most DashPay cases never
  touch L1 except the `send_payment` step (which spends Core funds → needs the
  bank's Core balance, like CR-003/AL-001).
- Test attribute `#[tokio_shared_rt::test(shared, flavor = "multi_thread",
  worker_threads = 12)]`; context provider `TrustedHttpContextProvider`.
- New cases: add `tests/e2e/cases/dp_NNN_*.rs`, register in `cases/mod.rs`, document
  in `tests/e2e/TEST_SPEC.md` (pin accounting). The shielded suite (PR #3727,
  `sh_*`) is the worked example of stacking a feature-area suite on this framework.

**Sequencing implication:** the DashPay e2e suite rides #3549 — but **M1's exit
criterion is the mock-seam unit/integration tier** (no #3549 dependency), so M1 is
never blocked on the draft PR. `dp_003`/`dp_004` are the e2e *confirmation* of the
same behaviors, tracked on #3549 (authored stacked on it, or added right after it
merges). The offline crypto/encode tier likewise lands immediately.

---

## Part 8 — Risks, decisions, open questions

1. **UI shape — first-class tab vs polish-in-place.** Recommended: first-class
   `DashPay` tab (Part 6). *Decision owner: product.* Fallback documented.
2. **Cross-client interop. RESOLVED (2026-06-10, desk-check
   `INTEROP_DESK_CHECK.md`):** xpub plaintext FAIL → G14 fix in M1 task 7; ECDH PASS;
   accountReference PASS-for-now (+2 latent masking bugs noted for M3); new G15
   key-purpose hazard → verification gate in M1 task 8. Live cross-client e2e
   stays M4. ⚠ A side-finding: our stack was **not** self-consistent either —
   the 107-byte plaintext broke our own send path (see G14).
3. **Watch-only / hardware wallets (G4).** Out of scope for the demo app (it holds
   the seed) but required for production. **FFI-hook design lands in M3 (task 15)**
   — shared secret only across the ABI, never a raw private key (see G4);
   implementation in M4.
4. **`accountReference` semantics (G3).** Decide whether to keep "share full
   account xpub, ignore masking" (simpler, but breaks rotation via the unique
   index) or implement the DIP-15 masking + version flow. Recommended: implement it
   (M3) — rotation is a real user need and the unique-index collision is a latent
   bug.
5. **Auto-accept (G7). DECIDED (2026-06-10): keep.** Invitations are now in scope
   (Milestone 5) and are built on `autoAcceptProof`, so the helpers + FFI param
   stay (dormant until M5 wires them). **Hard requirement when wired:** the
   `verify_auto_accept_proof` gate before any automatic acceptance (see G7).
6. **`send_contact_request` entropy (G2). RESOLVED (2026-06-10):** real broadcast
   bug — consensus rejected every send (`InvalidDocumentTransitionIdError`).
   Fixed in M1 task 4; see the DONE note there.
7. **E2E framework dependency.** The DashPay e2e suite rides PR **#3549** (draft,
   unmerged). **M1's exit criterion is the mock-seam tier** (Part 7.4), so M1 never
   blocks on it; the `dp_*` cases are authored stacked on #3549 or right after it
   merges. *Open: name the owner who decides stack-vs-wait before M1 starts.*

---

## Part 9 — Related in-flight work (open PRs)

Surfaced from the live PR list — these intersect this plan and should be tracked /
coordinated rather than duplicated:

| PR | Branch | Relevance |
|----|--------|-----------|
| **#3549** (draft) | `feat/rs-platform-wallet-e2e` | **The e2e framework** the DashPay suite must build on (Part 7.4). |
| **#3727** (draft) | `test/rs-platform-wallet-shielded-e2e` | Shielded `sh_*` e2e suite — the **worked template** for a feature-area suite on #3549. |
| **#3787** | `codex/dashpay-dip15-contact-request-docs` | "DashPay contact request encryption guide" — cross-check against Part 2; avoid doc drift. |
| **#3639** | `feat/platform-wallet-external-signable-wallets` | External/signable wallets — the substrate for **G4** (watch-only ECDH via `ClientSide`). Coordinate before building G4. |
| **#3692** | `feat/platform-wallet-rehydration` | Watch-only rehydration from persistor — touches the same watch-only path as G4. |
| **#3817** | `feature/coinjoin-sweep-and-recovery` | DashSync→SDK migration context (the broader effort DashPay sits inside). |
| **#3750** (NO MERGE) | `feat/platform-wallet-consumer-hardening` | FFI/consumer hardening — may move FFI signatures the Swift layer depends on. |

---

### Appendix — evidence sources

- [`INTEROP_DESK_CHECK.md`](./INTEROP_DESK_CHECK.md) —
  cross-client (iOS DashSync / Android dashj) interop evidence + testnet census.
- [`CONTACTINFO_FORMAT_SPEC.md` Appendix A](./CONTACTINFO_FORMAT_SPEC.md) —
  contactInfo wire conventions (this repo sets the de-facto convention).

The transient working-research files (DIP paraphrase, SDK/contract survey with
worktree-relative file:line citations) were trimmed from the tree; find them in
this branch's git history under `docs/dashpay/research/`.
