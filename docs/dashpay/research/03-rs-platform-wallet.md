# DashPay implementation map — `rs-platform-wallet` (+ FFI, storage)

Research date: 2026-06-10. Snapshot of the DashPay flow in the platform
wallet at the start of this work, with file:line citations and an
implemented/stub assessment per flow.

Scope packages:
- `packages/rs-platform-wallet` — library (logic)
- `packages/rs-platform-wallet-ffi` — C FFI surface (iOS/Swift)
- `packages/rs-platform-wallet-storage` — SQLite persistence

Key dependency facts (`rs-platform-wallet/Cargo.toml`):
- `dash-sdk` with features `["dashpay-contract", "dpns-contract", "wallet"]`
  (line 12). The DashPay system contract is loaded **locally** (no network
  round-trip) via `dpp::system_data_contracts::load_system_data_contract`.
- `platform-encryption` (line 13) — ECDH + AES-256-CBC for xpub/label encryption.
- `key-wallet` / `key-wallet-manager` (lines 16-17) — HD derivation, accounts,
  address pools, transaction builder.
- All state-transition broadcasting goes through `dash_sdk::platform::transition::*`
  (`PutDocument`, `PutContract`) and the SDK's `send_contact_request`.

---

## TL;DR status table

| Flow | Status | Where |
|------|--------|-------|
| Identity ↔ wallet (managed identities) | ✅ Implemented | `state/managed_identity/mod.rs`, `state/manager/*`, `network/identity_handle.rs` |
| DashPay contract load/cache | 🟡 Loaded-per-call (no cache) | `network/profile.rs:83`, `network/contact_requests.rs` (SDK fetches its own) |
| Profile — fetch/sync | ✅ Implemented | `network/profile.rs:64`, `:145` |
| Profile — create | ✅ Implemented (external signer only) | `network/profile.rs:240` |
| Profile — update | ✅ Implemented (external signer only) | `network/profile.rs:395` |
| Sync aggregator (`dashpay_sync`) | ✅ Implemented | `network/dashpay_sync.rs:16` |
| Sync contact requests (received) | 🟡 Implemented; no xpub decrypt in loop | `network/contact_requests.rs:322` |
| Send contact request | ✅ Implemented (seed-in-process only) | `network/contact_requests.rs:91` |
| Accept contact request | ✅ Implemented (reciprocal send) | `network/contact_requests.rs:466` |
| Reject contact request | 🟡 Local-only (no on-chain tombstone) | `network/contact_requests.rs:678` |
| Establish contact (auto) | ✅ Implemented | `state/managed_identity/contact_requests.rs` |
| Register receiving/external contact account | ✅ Implemented (seed-in-process) | `network/contacts.rs:100`, `:322` |
| Send money to a contact | ✅ Implemented | `network/payments.rs:93` |
| Record incoming payment | ✅ Implemented | `network/payments.rs:26` |
| Crypto: DIP-14 contact xpub / payment addrs | ✅ Implemented | `crypto/dip14.rs` |
| Crypto: account reference (DIP-15) | ✅ Implemented (but **unused** in send path) | `crypto/dip14.rs:147` |
| Crypto: auto-accept proof gen/verify | 🟡 Implemented but **dead code** | `crypto/auto_accept.rs` (`// TODO: Where and how we use these helpers?` :39) |
| Pre-send validation | 🟡 Implemented but **not called** by send path | `crypto/validation.rs:76` |
| Persistence round-trip (contacts/profile/payments) | ✅ Implemented | `wallet/apply.rs`, storage `schema/{contacts,dashpay}.rs` |
| FFI surface | ✅ Implemented | `ffi/src/{dashpay,dashpay_profile,contact_request,established_contact,contact}.rs` |

Legend: ✅ Implemented · 🟡 Partial/caveated · ❌ Stub/Missing.
**There are zero `todo!()`/`unimplemented!()`/`unreachable!()` in the DashPay
code paths** — the gaps are caveats, dead helpers, and local-only fallbacks,
not panics.

---

## 1. Identity ↔ wallet connection (managed identities)

### `ManagedIdentity` — the shared state object
`state/managed_identity/mod.rs:37-105`. One `ManagedIdentity` carries BOTH the
Platform `Identity` and ALL DashPay fields:
- `identity: Identity` (:39)
- `identity_index: Option<u32>` (:51) — the HD slot
  `m/9'/coin'/5'/0'/key_type'/identity_index'/key_id'`. `Some` ⇒ wallet-owned
  (can sign / derive ECDH). `None` ⇒ out-of-wallet observed identity (cannot sign).
- `established_contacts: BTreeMap<Identifier, EstablishedContact>` (:60)
- `sent_contact_requests` / `incoming_contact_requests` (:63, :66)
- `dashpay_profile: Option<DashPayProfile>` (:99)
- `dashpay_payments: BTreeMap<String, PaymentEntry>` keyed by txid (:104)

The manager keeps these in two buckets — `wallet_identities[wallet_id][identity_index]`
(signing-capable) and `out_of_wallet_identities[identity_id]` (read-only) — documented
at `mod.rs:27-35` and implemented in `state/manager/{mod,lifecycle,accessors,apply}.rs`.

### `IdentityWallet<B>` — the network façade
`network/identity_handle.rs:256-276`. A view over the shared
`Arc<RwLock<WalletManager<PlatformWalletInfo>>>` plus `Arc<Sdk>`, a
`WalletPersister`, an `AssetLockManager`, and a generic broadcaster `B`
(defaults to `SpvBroadcaster`). It owns ALL DashPay operations (the historical
separate `DashPayWallet` was merged — see module doc `network/mod.rs:1-16`,
`identity_handle.rs:1-21`). DashPay ops reuse the same signer / asset-lock plumbing.

### ECDH key derivation
`IdentityWallet::derive_encryption_private_key` (`identity_handle.rs:424-467`):
derives the sender's ECDH secp256k1 secret from the wallet seed at the DIP-9
identity-auth path (`m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_id'`). **Requires
the seed in-process** — this is the root of the watch-only caveat throughout DashPay.

### DashPay contract load/cache — 🟡
Loaded fresh **per call** from the bundled system contract:
`network/profile.rs:83-93` and `:255` (and the SDK loads its own copy inside
`send_contact_request` / `fetch_*`). There is **no shared cached `Arc<DataContract>`**
on the wallet — each operation re-`load_system_data_contract`s. Cheap (in-memory,
bundled) but redundant.

---

## 2. Profile

Types: `types/dashpay/profile.rs`.
- `DashPayProfile` (:25-40) — stored/displayed model (no raw avatar bytes; only
  `avatar_hash: [u8;32]` and `avatar_fingerprint: [u8;8]` survive).
- `ProfileUpdate` (:46-58) — input; carries raw `avatar_bytes` which the wallet
  hashes then drops.
- `calculate_avatar_hash` (SHA-256, :61) and `calculate_dhash_fingerprint`
  (perceptual dHash over a decoded image, :80) — both fully implemented.

### Fetch / sync — ✅
- `sync_profiles` (`network/profile.rs:64-139`): collect all managed identity ids,
  load DashPay contract, fetch each profile doc, cache via
  `managed.set_dashpay_profile(...)`. Clears the local cache when none on Platform.
- `fetch_profile_document` (`:145-219`): `Document::fetch_many` with
  `DocumentQuery` `profile WHERE $ownerId == id LIMIT 1`; maps
  `displayName / publicMessage / avatarUrl / avatarHash / avatarFingerprint`
  into `DashPayProfile`. (`bio` is aliased from `publicMessage`.)

### Create — ✅ (external signer only)
`create_profile_with_external_signer` (`network/profile.rs:240-388`):
1. load contract, 2. compute avatar hashes from bytes, 3. build `BTreeMap`
properties, 4. pick first HIGH/CRITICAL AUTHENTICATION ECDSA key (MASTER excluded
— see :308-314), 5. build a `DocumentV0` ("stub_document", :330 — the name is
benign, not a stub), 6. `put_to_platform_and_wait_for_response` (:356), 7. update
local cache. Real broadcast.

### Update — ✅ (external signer only)
`update_profile_with_external_signer` (`network/profile.rs:395-592`): fetches the
existing profile doc for its id + revision, bumps `revision + 1`, preserves avatar
fields when no new bytes, broadcasts via `PutDocument`. Real broadcast.

> Note: there are **no legacy non-signer** `create_profile` / `update_profile`
> variants in the current tree — only the `*_with_external_signer` forms exist.
> The docstrings reference `Self::create_profile` / `Self::update_profile` as the
> "legacy variant", but those methods are gone (grep returns nothing).

---

## 3. Sync (`dashpay_sync.rs`)

`network/dashpay_sync.rs:16-22` — `dashpay_sync()` is a 2-step aggregator:
`self.sync_contact_requests().await?` then `self.sync_profiles().await?`.
Failures propagate; partial progress not rolled back. ✅ Implemented.

`sync_contact_requests` (`network/contact_requests.rs:322-451`): for every managed
identity, `sdk.fetch_received_contact_requests(id, None)`; for each received doc,
skip if already tracked, otherwise parse `senderKeyIndex / recipientKeyIndex /
accountReference / encryptedPublicKey` (warn+skip on missing) and call
`managed.add_incoming_contact_request(...)` (which may auto-establish). Returns
the newly-discovered requests.

🟡 **Caveat**: the sync loop stores the **encrypted** `encryptedPublicKey` bytes
on the incoming `ContactRequest` but does NOT decrypt the contact's xpub or build
the external sending account during sync. Decryption + external-account
construction only happen on the **accept** path
(`register_external_contact_account`, see §5/§6). So a contact that becomes
established purely by sync (both requests arriving via sync) will have its xpub
sitting encrypted until `register_external_contact_account` is invoked.
`identity_sync.rs` (the periodic `IdentitySyncManager`) has **no** DashPay hooks —
DashPay refresh is driven separately through `dashpay_sync()` / the FFI.

The SDK side is real: `rs-sdk/src/platform/dashpay/contact_request_queries.rs`
(`fetch_sent_contact_requests` :33, `fetch_received_contact_requests` :76).

---

## 4. Send contact request

`send_contact_request_with_external_signer` (`network/contact_requests.rs:91-304`):
1. look up sender `ManagedIdentity` + `identity_index` (`IdentityIndexNotSet` error
   if out-of-wallet, :114).
2. `Identity::fetch` the recipient from Platform (:122).
3. resolve `sender_key_index` = first `Purpose::ENCRYPTION` key on sender (:136),
   `recipient_key_index` = first `Purpose::DECRYPTION` key on recipient (:148).
4. derive the DashPay **receiving** account xpub at
   `AccountType::DashpayReceivingFunds { index:0, user, friend }` and the sender's
   ECDH private key via `derive_encryption_private_key` (:163-198).
5. pick a HIGH/CRITICAL AUTHENTICATION ECDSA key for the document signature (:201).
6. build SDK `ContactRequestInput` + `SendContactRequestInput` wrapping the borrowed
   signer in `SignerRef` (:223-237).
7. **ECDH is `EcdhProvider::SdkSide`** (:240-261) — the wallet hands the SDK the
   sender's private key; the SDK does ECDH + AES-256-CBC encryption of the xpub
   internally (`rs-sdk/.../contact_request.rs:242-273`, exactly 96 bytes =
   16-byte IV + 80-byte ciphertext). `get_extended_public_key` closure (:266)
   returns the encoded receiving xpub.
8. `self.sdk.send_contact_request(...)` broadcasts the `contactRequest` document
   via `PutDocument` (`rs-sdk/.../contact_request.rs:438-448`).
9. mirror local state: build a `ContactRequest` and
   `managed.add_sent_contact_request(...)` (:277-298), then
   `register_contact_account(...)` to create the receiving account (:300).

✅ Implemented end-to-end. 🟡 **Caveat (documented :81-89):** step 4 derives ECDH
from the wallet seed → **watch-only wallets fail here**. Only `EcdhProvider::SdkSide`
is ever used in the wallet (grep confirms no `ClientSide`); a follow-up FFI to push
ECDH across the boundary is noted but not built.

> Inconsistency worth flagging: the locally-stored sent `ContactRequest` uses a
> placeholder `vec![0u8; 96]` for `encrypted_public_key` (:283) rather than the
> actual ciphertext the SDK produced — the real bytes are only on Platform.

---

## 5. Receive / approve / accept contact request

### Detect incoming
Via `sync_contact_requests` (§3) → `add_incoming_contact_request`
(`state/managed_identity/contact_requests.rs:87-127`).

### Auto-establish
`add_sent_contact_request` (:22-62) and `add_incoming_contact_request` (:87-127):
if the reciprocal request already exists, immediately build an `EstablishedContact`,
insert it into `established_contacts`, and emit a `ContactChangeSet.established`
(the matching pending entries are dropped per the changeset contract — no separate
tombstone). `accept_incoming_request` (:153-194) does the same when **both**
requests already exist locally. Heavily unit-tested (`contact_requests.rs` tests +
`managed_identity/mod.rs` tests).

### Accept (network)
`accept_contact_request_with_external_signer` (`network/contact_requests.rs:466-545`):
1. verify the incoming request is known.
2. capture the contact's encrypted xpub + key indices.
3. send the **reciprocal** request via
   `send_contact_request_with_external_signer` (§4) — this is what marks the contact
   established (auto-establishment fires when our sent request meets their incoming).
4. **best-effort** `register_external_contact_account(...)` (decrypt their xpub →
   build watch-only sending account); failure is logged, not fatal (:511-528).
5. return the auto-established `EstablishedContact`.

### Reject — 🟡 local only
`reject_contact_request` (`:678-714`): removes the incoming request locally; returns
`ContactRequestNotFound` if absent. Explicit `TODO` (:703) — no on-chain
`contactInfo` `display_hidden` document is written, so a reject does NOT sync across
devices. ("requires SDK support for document creation on arbitrary contracts which
is not yet available here" :672 — note this is slightly stale, since profile/contract
writes DO exist; only the `contactInfo` doc type is unwired.)

---

## 6. Established contact

`types/dashpay/established_contact.rs:14-35` — `EstablishedContact` holds:
`contact_identity_id`, `outgoing_request: ContactRequest`, `incoming_request:
ContactRequest`, plus local UI metadata `alias`, `note`, `is_hidden`,
`accepted_accounts: Vec<u32>`. It does **NOT** store derived shared keys or
derivation paths directly — those are reconstructed on demand from the two embedded
`ContactRequest`s (key indices) + the wallet seed. The actual receiving/sending
**accounts** live in the `key_wallet` `ManagedAccountCollection`
(`dashpay_receival_accounts` / `dashpay_external_accounts`), not on the contact.

Local mutators: `state/managed_identity/contacts.rs` (`add/remove/get
established_contact`), plus alias/note/hide setters on the type itself.
`network/contacts.rs:26-48` `established_contacts()` flattens contacts across both
identity buckets (with a `TODO` about cloning, :22).

Account registration:
- `register_contact_account` (`network/contacts.rs:100-156`): derives the
  `DashpayReceivingFunds` xpub and inserts a funds-bearing managed account so SPV
  watches incoming payments. Needs the seed (not watch-only safe).
- `register_external_contact_account` (`:322-516`): the **receive-from-contact-xpub**
  path. Derives our ECDH key, fetches the contact identity, computes the shared key
  via `platform_encryption::derive_shared_key_ecdh` (:434), decrypts their xpub via
  `platform_encryption::decrypt_extended_public_key` (:438), decodes the
  `ExtendedPubKey`, and registers a **watch-only** `DashpayExternalAccount`
  (immutable `Account` for the xpub + managed account for the address pool, :455-507).

Address matching for inbound payments: `match_incoming_dashpay_address`
(+ `_blocking` / `try_*`) and `match_in_collection` (`:181-260`) iterate
`dashpay_receival_accounts` and return a `DashpayAddressMatch`.

---

## 7. Send money to a contact

`send_payment` (`network/payments.rs:93-245`): ✅ Implemented.
1. resolve the contact's `DashpayExternalAccount` xpub from the immutable
   `wallet.accounts.dashpay_external_accounts` (errors if
   `register_external_contact_account` wasn't called first, :135).
2. derive the next unused address from the external account's address pool
   (`external_account.next_address(...)`, :166).
3. fund from the standard BIP-44 account 0, build a signed tx via
   `key_wallet`'s `TransactionBuilder` (`LargestFirst` selection, :192-203).
4. broadcast through the injected `self.broadcaster.broadcast(&tx)` (:209).
5. record a `PaymentEntry::new_sent(...)` on the sender's `ManagedIdentity`
   via `record_dashpay_payment` (:225-242).

Incoming payment recording: `try_record_incoming_payment` (`:26-60`): non-blocking
address match + spawn a task to record `PaymentEntry::new_received(...)`.

Payment types: `types/dashpay/payment.rs` — `PaymentEntry` (counterparty_id,
amount_duffs, memo, `PaymentDirection`, `PaymentStatus`), `DashpayAddressMatch`.

---

## 8. Crypto (`dip14.rs`, `auto_accept.rs`, `validation.rs`)

`crypto/dip14.rs` — ✅ all implemented + well-tested:
- `derive_contact_xpub` (:82): path `m/9'/coin'/15'/account'/(sender_id)/(recipient_id)`,
  last two segments DIP-14 256-bit non-hardened (`ChildNumber::Normal256` inside
  `key_wallet::bip32`). Built via `AccountType::DashpayReceivingFunds.derivation_path`.
- `calculate_account_reference` (:147): DIP-15 HMAC-SHA256 ASK28 ⊕ account-bits with
  4-bit version prefix. ✅ correct — but **not wired into the send path** (the send
  path hardcodes `account_reference = account_index = 0`; this helper is only used
  in its own tests). 🟡 unused.
- `derive_contact_payment_address` / `_addresses` (:189, :216): BIP-32 non-hardened
  derivation off the contact xpub → P2PKH. `DEFAULT_CONTACT_GAP_LIMIT = 10` (:235).

`crypto/auto_accept.rs` — 🟡 implemented but **dead code**. `generate_auto_accept_proof`
(:116) / `verify_auto_accept_proof` (:158) at path `m/9'/coin'/16'/timestamp'` with a
70-byte proof format, all unit-tested. But an explicit module-level
`// TODO: Where and how we use these helpers?` (:39) — nothing in the wallet calls them.
The send path accepts an `auto_accept_proof: Option<Vec<u8>>` from the FFI caller but
never **generates** one internally.

`crypto/validation.rs` — 🟡 implemented but **not invoked** by the live send path.
`validate_contact_request` (:76) checks sender ENCRYPTION key + recipient DECRYPTION
key types/purposes/disabled. Thoroughly tested, but `send_contact_request_with_external_signer`
does its own ad-hoc `find(...Purpose::ENCRYPTION...)` lookup and never calls this
validator.

`rust-dashcore` / `key_wallet` calls used: `secp256k1` (ECDH, ECDSA sign/verify),
`hashes::{sha256, hmac, Hash}`, `bip32::{ExtendedPubKey, ExtendedPrivKey, ChildNumber,
DerivationPath}`, `Address::p2pkh`, `Wallet::derive_extended_{public,private}_key`,
`AccountType::derivation_path`. ECDH + AES live in the sibling `platform-encryption`
crate (`derive_shared_key_ecdh`, `encrypt/decrypt_extended_public_key`,
`encrypt/decrypt_account_label`).

---

## FFI surface (`rs-platform-wallet-ffi`)

### Network-broadcasting DashPay (`src/dashpay.rs`)
- `platform_wallet_get_managed_identity(wallet_handle, identity_id, out) -> Result`
  — snapshot clone of a `ManagedIdentity` into `MANAGED_IDENTITY_STORAGE`.
- `platform_wallet_sync_contact_requests(wallet_handle, out_array) -> Result`
  — wraps `IdentityWallet::sync_contact_requests`; returns `ContactRequestHandleArray`.
- `platform_wallet_send_contact_request_with_signer(wallet_handle, sender_id,
  recipient_id, account_label, auto_accept_proof, auto_accept_proof_len,
  signer_handle, out_request_handle) -> Result` — routes to
  `send_contact_request_with_external_signer`. ECDH caveat documented (:206-212).
- `platform_wallet_accept_contact_request_with_signer(wallet_handle, request_handle,
  signer_handle, out_established_handle) -> Result`.
- `platform_wallet_reject_contact_request(wallet_handle, our_identity_id,
  contact_identity_id) -> Result` (local-only).
- `platform_wallet_fetch_sent_contact_requests(wallet_handle, identity_id,
  out_array) -> Result`.
- `platform_wallet_send_dashpay_payment(wallet_handle, from_identity_id,
  to_contact_identity_id, amount_duffs, memo, out_txid) -> Result`.
- `platform_wallet_contact_request_handle_array_free(*mut ContactRequestHandleArray)`.

### Profile FFI (`src/dashpay_profile.rs`)
- `managed_identity_get_dashpay_profile(identity_handle, out_profile,
  out_has_profile) -> Result` (reads cache).
- `platform_wallet_get_dashpay_profile(wallet_handle, identity_id, out_profile,
  out_has_profile) -> Result`.
- `platform_wallet_sync_dashpay_profiles(wallet_handle, out_synced_count) -> Result`.
- `platform_wallet_create_or_update_dashpay_profile_with_signer(wallet_handle,
  identity_id, display_name, public_message, avatar_url, avatar_bytes,
  avatar_bytes_len, do_create, signer_handle, out_profile) -> Result`
  — `do_create` toggles create vs update.
- `dashpay_profile_ffi_free(*mut DashPayProfileFFI)`. Flat struct
  `DashPayProfileFFI` (:18-27).

### Contact-request / established-contact field accessors
`src/contact_request.rs`: `contact_request_create`, `..._get_{sender_id,
recipient_id, sender_key_index, recipient_key_index, account_reference,
encrypted_public_key, created_at}`, `..._destroy`,
`managed_identity_get_{sent,incoming}_contact_request`.

`src/established_contact.rs`: `managed_identity_get_established_contact`,
`established_contact_get_{contact_id, contact_identity_id, outgoing_request,
incoming_request, alias, note}`, `..._set_{alias,note}`, `..._clear_{alias,note}`,
`..._is_hidden`, `..._hide`, `..._unhide`, `..._destroy`.

### Local-state-only contact ops (`src/contact.rs`) — legacy / in-memory
`managed_identity_get_{sent,incoming}_contact_request_ids`,
`managed_identity_get_established_contact_ids`,
`managed_identity_is_contact_established`,
`managed_identity_{send,accept,reject}_contact_request` — these mutate **local
state only** via a no-op persister (`ffi_noop_persister`), no Platform broadcast.
The module doc (`dashpay.rs:1-32`) says iOS flows should drive from the
`platform_wallet_*_with_signer` family instead; these remain for tests/bootstrap.

`src/contact_persistence.rs` — `OnPersistContactsFn` callback type for host-driven
contact persistence.

---

## Persistence round-trip (storage)

DashPay state is fully round-tripped through the changeset → apply → SQLite pipeline:
- Local mutators emit changesets: `add_{sent,incoming}_contact_request`,
  `set_dashpay_profile` (`identity_ops.rs:129`), `record_dashpay_payment`
  (`identity_ops.rs:152`) — all call `persister.store(cs.into())`.
- Apply (restore): `wallet/apply.rs:173-256` routes `cs.contacts` (sent/incoming/
  established) to the owning `ManagedIdentity` (orphans skipped with a log), then
  `dashpay_profiles` and `dashpay_payments_overlay` overlays (:240-256).
- Storage: `storage/src/sqlite/schema/contacts.rs` (single `contacts` table with
  `state ∈ {sent, received, established}`, both request blobs + 4 metadata columns;
  established upserts collapse/promote in one statement) and `schema/dashpay.rs`
  (`dashpay_profiles`, `dashpay_payments_overlay`).

---

## Most important gaps / risks

1. **Watch-only wallets cannot do DashPay sends/accepts.** Every send/accept derives
   the sender's ECDH key from the in-process seed
   (`identity_handle.rs:424`, `contact_requests.rs:190`). Only `EcdhProvider::SdkSide`
   is used. The planned "push ECDH across the FFI" follow-up is not built.
2. **Sync does not build sending accounts.** `sync_contact_requests` stores encrypted
   xpubs but never decrypts them or registers `DashpayExternalAccount`s — only the
   explicit accept path does. A contact established via two sync rounds may have no
   sending account until `register_external_contact_account` is called manually.
3. **DIP-15 account reference is computed but unused.** `calculate_account_reference`
   exists and is correct, but the send path hardcodes `account_reference = 0`.
4. **Auto-accept proofs are dead code** (`auto_accept.rs:39` TODO) — generated/verified
   nowhere; the send path takes a caller-supplied proof but never produces one.
5. **Pre-send validation is dead code** — `validate_contact_request` is never called
   by the live send path.
6. **Reject is local-only** — no on-chain `contactInfo` tombstone
   (`contact_requests.rs:703` TODO); rejections don't sync across devices.
7. **No DashPay contract caching** — re-loaded from the bundled system contract on
   every profile/contact-request operation.
8. **Locally-stored sent `ContactRequest` carries placeholder `vec![0u8;96]`** for
   `encrypted_public_key` (`contact_requests.rs:283`) instead of the real ciphertext.
9. **No legacy non-signer profile/contact-request methods** — only the
   `*_with_external_signer` variants exist, though docstrings still reference the
   removed legacy forms.
