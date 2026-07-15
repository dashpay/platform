# DashPay Invitations (DIP-13 sub-feature 3') — Implementation Spec

> **Status:** SHIPPED on PR #4041 (2026-07-14) — create + claim + reclaim + persistence + UI,
> three review-fix rounds folded, funded testnet e2e green (TEST_PLAN DP-12..19, `AI_QA/QA004`).
> The original design pass (2026-07-08, §1–§14 below) is kept as rationale; **§0 records where
> the as-built implementation deliberately diverged.** Where §0 and a later section disagree,
> §0 wins.

Tracked as the "NEXT" item in the DashPay backlog (dashpay/platform#4020); called out in
`SPEC.md` Milestone 5 and `DIP_CONFORMANCE_GAPS.md`.

---

## 0. As-built delta (supersedes the marked sections below)

1. **Link envelope = the LEGACY query format, not the §6 binary blob (supersedes §6, §7).**
   The 2026-07-13 legacy-compat rework (owner decision; contract in §0A) replaced the hand-rolled versioned payload with the
   query form shared with dash-wallet Android / dashwallet-iOS, so links are field-level
   cross-claimable: `dashpay://invite?du=<username>&assetlocktx=<txid>&pk=<WIF>&islock=<hex|null>`
   `[&display-name=…][&avatar-url=…]` (also parses `https://invitations.dashpay.io/applink?…`).
   Emit strict / parse lenient. Consequences:
   - The link carries the funding **txid**, not the embedded proof → **claim-by-fetch**: the
     invitee refetches the funding tx by txid (bounded retry for DAPI propagation lag, both
     byte orders), reconstructs the proof, and selects the credit output by matching
     `voucher_credit_script(pk)`.
   - **No expiry field on the wire** — the §5.1/§8/§10 "claim refuses a past-expiry link"
     mechanism does not exist in the as-built claim; `expiry_unix` survives only as inviter-side
     local display metadata. The economic bounds are the amount caps.
   - **No inviter identity id on the wire** (`inviter_id` always zeroed) — the contact bootstrap
     resolves the id from the `du` username via DPNS at claim time. `InviterInfo.username` is
     `Option`: a display-name/avatar-only link is metadata-only (`has_inviter == true`,
     `inviter_username == nil`, no bootstrap).
   - Amount is not on the wire (claim preview shows "—").
2. **Claim accepts ChainLock invites too (amends §5.1).** `islock` absent or literal `"null"`
   ⇒ a `ChainAssetLockProof` is reconstructed (requires the funding tx to be chain-locked).
   Create still emits only InstantSend links — a slow-IS ChainLock fallback at create is
   rejected as a *link* but the funded lock is recorded first and stays reclaimable.
3. **Amounts (amends §5/§8/§9):** `MIN_INVITATION_DUFFS = 300_000` (0.003 DASH — a smaller
   voucher can fund neither a claim nor a register-reclaim, discovered by funded e2e),
   `MAX_INVITATION_DUFFS = 5_000_000` (0.05 DASH), Swift default **0.03 DASH**.
4. **Persistence as-built (amends §4.2):** the `InvitationChangeSet` flows through
   `PlatformWalletPersistence::store()` to each backend — the SQLite backend's
   `V003__invitations` table, and on iOS the FFI `on_persist_invitations_fn` bridge into the
   SwiftData `PersistentInvitation` model (SwiftData is the UI source; no Rust rehydrate; §0B). Persist failures are signaled end-to-end
   (nonzero callback → rolled-back round → `create_invitation` errors), not best-effort.
5. **Durability + ordering hardening (review rounds 1–3; the per-finding log lives in the
   PR #4041 review threads + commit messages):** the pre-broadcast gate persists **and flushes** the
   invitation funding-index pool (aborting before broadcast on failure); creation refuses
   non-durable backends (`PlatformWalletPersistence::persists_durably()`); the funded-asset-lock
   flow is split so the invitation record is persisted immediately **after broadcast, before the
   proof wait** — an interrupted create can no longer orphan a funded voucher.
6. **Reclaim shipped (extends §1 scope):** an unclaimed voucher is recovered as identity
   **credits** (top-up an existing identity or register a new one; the L1 amount was
   OP_RETURN-burned). Already-consumed handling is classified via the persisted
   `reclaimInFlight` marker: marker unset ⇒ provably a foreign claim (neutral "already
   claimed"); marker set ⇒ **explicitly ambiguous** (the marker proves only that a local
   consume attempt started, not that it landed — a racing claim is indistinguishable, so the
   row resolves to the conservative terminal `Claimed` with an ambiguity message, never an
   inferred `Reclaimed`) — see `AI_QA/QA004` step 6 for the exact classifier arms.
7. **QA contract as-built:** TEST_PLAN §4.10 rows **DP-12..DP-19** (not just DP-12..15) +
   `AI_QA/QA004_invitation_reclaim.md`; funded e2e evidence recorded there.

---

## 0A. As-built link envelope & legacy interop (absorbs the legacy-compat spec)

The interop contract is **field-level parity with the live legacy wallets, emit
strict/canonical, parse leniently** — exactly as tolerantly as the live Android wallet.
Byte-for-byte parity is NOT the contract (the two legacy wallets differ in param order and in
scheme/host). The on-chain primitive and derivation path (`m/9'/coin'/5'/3'/idx'`) are
identical across all three wallets — no consensus change.

### 0A.1 Wire format

**Emit (canonical, what we produce):**
```text
dashpay://invite
  ?du=<inviter DPNS username>                 # required to emit; optional on parse
  &assetlocktx=<funding txid, lowercase BIG-ENDIAN display hex>
  &pk=<voucher credit-burn key, WIF, COMPRESSED, network-correct>
  &islock=<InstantSend lock, lowercase hex>   # or omit (see below)
  [&display-name=<inviter display name>]
  [&avatar-url=<inviter avatar url, single %-encoded>]
```
- Parse **by field name, order-independent**; accept **both** the `dashpay://invite` scheme
  and the `https://invitations.dashpay.io/applink` host (iOS legacy links use the latter).
- **`pk`**: WIF, **compressed** flag set (the credit-output hash uses the *compressed*
  pubkey — wrong compression ⇒ wrong `hash160` ⇒ claim fails), network byte `0xCC` mainnet /
  `0xEF` testnet-family.
- **`assetlocktx`**: emit lowercase big-endian display hex; on claim parse leniently — try
  as-given, then **retry byte-reversed** on a fetch miss (old iOS links are little-endian).
- **`islock`**: OPTIONAL, with two absence forms — param missing **and the literal string
  `"null"`** (Android emits `"null"` for a chainlock-confirmed invite). Absent/`"null"` ⇒
  reconstruct a **`ChainAssetLockProof`** at claim, never reject. The hex is not
  self-describing: decode as the modern deterministic **ISDLOCK**; the ancient
  non-deterministic ISLOCK is unrepresentable in rust-dashcore and fails closed (documented
  limitation — no live producer exists).
- **Validity (lenient superset of both wallets):** require `assetlocktx` + `pk`
  present/non-blank; never reject solely on a missing `du` or missing/`"null"` `islock`.

### 0A.2 Claim-by-fetch

The link carries the funding **txid**, not a proof, so claim reconstructs it (mirrors Android
`TopUpRepository.obtainAssetLockTransaction`):
1. Fetch the tx by `assetlocktx` via `Sdk::get_transaction` (bounded retry/backoff for DAPI
   propagation lag; reversed-retry per §0A.1).
2. Fail-fast guards: fetched txid matches `assetlocktx` (either byte order); when an islock is
   present, `islock.txid == fetched tx.txid`.
3. **Derive `output_index` by script match** — scan the fetched tx's `credit_outputs` for the
   output whose `script_pubkey` == `voucher_credit_script(pk)`; never hard-code index 0.
4. Build `InstantAssetLockProof` (islock present) or `ChainAssetLockProof` (absent/`"null"`;
   requires the tx to be chain-locked), then submit through the **unchanged**
   `put_to_platform_and_wait_for_response_with_private_key`.

Consensus enforces pk↔output, islock↔tx, and identity_id↔outpoint — all fail closed; the
local guards are fast-fail UX + correct index selection, not theft prevention.

### 0A.3 Consequences of the legacy format

- **No inviter identity id on the wire** — only `du`. `InviterInfo = {username?, display_name?,
  avatar_url?}` and the invitee resolves the inviter's id from `du` via DPNS at
  contact-bootstrap. A `du`-less link is metadata-only (`has_inviter == true`,
  `inviter_username == nil`, no bootstrap possible).
- **No expiry on the wire** — the pre-network staleness gate is gone; the real bounds are the
  amount caps + reclaim. The inviter-side local record keeps expiry for display only.
- **Amount is not on the wire** — the claim preview shows "—" pre-fetch.

### 0A.4 Amounts (onboarding tiers)

`MIN_INVITATION_DUFFS = 300_000` (0.003 DASH, == Android `DASH_PAY_INVITE_MIN`; a smaller
voucher can fund neither a claim nor a register-reclaim — found by funded e2e).
`MAX_INVITATION_DUFFS = 5_000_000` (0.05 DASH). Swift create default **0.03 DASH** = identity
+ a normal DPNS name (Android `DASH_PAY_FEE`). The contested-name tier (0.25) is **deferred**
until contested registration is wired into the claim flow.

### 0A.5 Transport

The custom `dashpay://` scheme is the shipped, first-class transport (QR / share sheet /
in-person). The legacy wallets' AppsFlyer OneLink wrapper is **externally blocked** (Android
team creds; brand domain + template) and tracked separately (#4096-adjacent); note that
OneLink discloses the plaintext `pk` to AppsFlyer server-side — an accepted, documented
regression vs a self-contained link, bounded by the amount cap + reclaim. The custom scheme's
same-device interception limitation is documented in `Info.plist` + §6.1.

---

## 0B. As-built persistence & reclaim (absorbs the Swift-persistence spec)

### 0B.1 Persistence bridge

`InvitationChangeSet` (structurally an `asset_locks`-style `BTreeMap` upserts +
`BTreeSet` removals) flows through `PlatformWalletPersistence::store()` to each backend: the
SQLite backend's `V003__invitations` table, and on iOS the **push-callback FFI bridge**
(`on_persist_invitations_fn` → `persistInvitationsCallback` → SwiftData
`PersistentInvitation`), mirroring the asset-lock wiring. Key properties:
- **SwiftData is the UI source; push-only, no Rust→Swift rehydrate.** A SwiftData wipe loses
  only list *visibility* — never funds or key re-derivability (`funding_index` re-derives the
  voucher key).
- **Persist failures are signaled, never swallowed:** a skipped write returns nonzero from the
  callback, failing the (invitation-only) `store()` round and surfacing an error from
  `create_invitation` instead of reporting a voucher that never reached SwiftData.
- **Outpoint key seam:** both the upsert and the removal path derive the unique
  `outPointHex` via `PersistentAssetLock.encodeOutPoint` verbatim (key-form drift is pinned by
  `InvitationPersistenceTests`).

### 0B.2 Reclaim

The invitation's DASH is **burned into an `OP_RETURN`** at create time — the credit output
exists only in the tx payload as a Platform-side authorization, never as an L1 UTXO — so
"reclaim" means: **the inviter consumes the still-unclaimed voucher into a Platform identity
of their own, recovering the value as credits** (mechanically, claiming your own invitation).
UI copy always says "recovered as identity credits", never "DASH returned".

- **Primitive:** consume the tracked lock via
  `FromExistingAssetLock { out_point, consume_invitation_voucher: true }` — the inviter's own
  signer re-derives the voucher key at `9'/coin'/5'/3'/funding_index'` internally (no key
  export). Two user-picked targets: **top-up an existing identity** or **register a new
  one**. The `consume_invitation_voucher` flag is the reclaim flow's **explicit
  authorization**: every generic resume/top-up path passes `false` and the funding resolver
  refuses `IdentityInvitation`-typed locks, so a shared voucher can never be silently
  consumed into an unrelated local identity (the Swift resumable-registrations surface also
  excludes `fundingTypeRaw == 3` rows).
- **Race / already-consumed:** no L1 double-spend exists (no shared UTXO); Platform
  deterministically rejects the second consume
  (`IdentityAssetLockTransactionOutPointAlreadyConsumed` — the loser wastes only an ST fee).
  The Swift side classifies via the persisted `reclaimInFlight` marker, which is saved
  (required — the consume may not run on a failed save) only immediately before the on-chain
  consume: marker unset ⇒ provably the invitee claimed first (row → `Claimed`, neutral
  "This invitation was already claimed." — claimant not named); marker set ⇒ **explicitly
  ambiguous** — the marker proves only that a local consume attempt started, not that it
  landed (a racing claim between crash and retry is indistinguishable), so the row resolves
  to the conservative terminal `Claimed` with an ambiguity message, never an inferred
  `Reclaimed` (`Reclaimed` is written only by a success observed in-flow). The local
  "is not tracked" resume-guard failure with the marker set is surfaced as an explicit
  ambiguity error (status unchanged — there is no on-chain proof of consumption at all).
  The decision is the pure, unit-tested `classifyReclaimFailure(error:hadPriorReclaimInFlight:)`
  seam; see `AI_QA/QA004` step 6 for the verified classifier arms.
- **Status lifecycle:** `Reclaimed`/`Claimed` are written by the Swift UI on the local row
  (SwiftData is the UI source; create is the only Rust emitter).

---

## 1. Problem & goal

DashPay onboarding today assumes the new user already **has** a Dash identity (which
requires L1 Dash to fund the ~0.0002 DASH asset lock that registers it). That is a
chicken-and-egg wall for inviting a friend who has never touched Dash: they can't receive a
payment (no identity → no contact) and can't register an identity (no funds).

**DIP-13 "Identity Invitation Funding keys" solves this.** An existing user (the *inviter*)
pre-funds an asset lock at a dedicated derivation sub-feature, hands the one-time private key
+ the asset-lock proof to a friend (the *invitee*) as a link, and the invitee registers
**their own new identity** funded by that voucher — no L1 Dash required on the invitee's
side. The invitation optionally bootstraps the DashPay contact in the same act (the invitee's
contact request to the inviter carries a DIP-15 `autoAcceptProof`, so it auto-establishes).

**Goal:** implement invitation **create** (inviter) and **claim** (invitee) end-to-end across
`rs-platform-wallet` + `rs-platform-wallet-ffi` + `swift-sdk` + `SwiftExampleApp`, with unit
+ integration tests, a testnet funded e2e, and QA-contract scenarios.

### Non-goals
- **No byte-for-byte interop with the production iOS/Android DashWallet invitation link.** We
  can't drive those builds in this environment (same constraint the auto-accept spec accepted:
  iOS-first, DIP-faithful where the DIP defines a format, normative-for-us where it is silent).
  The **on-chain** artifacts (asset lock, IdentityCreate, contactRequest) are consensus formats
  and *are* interoperable; only the off-chain **link envelope** is ours. See §7 for the interop
  decision once the reference format is confirmed.
- **No new on-chain artifact.** Invitations reuse the existing AssetLock special-tx, the
  IdentityCreate transition, and a plain contactRequest.
- **No auto-accept bearer key in the invitation (v1).** The contact-bootstrap is a *normal*
  contact request (see §2 design change); no `dapk` is embedded.
- **No invitation for identity-less inviters in v1** beyond the pure funding voucher: the
  contact-bootstrap requires the inviter to hold a registered identity. A voucher from an
  identity-less funder still works as pure onboarding funding; it just carries no inviter to
  contact.
- **Advisory expiry, not consensus revocation.** The voucher key controls an on-chain asset
  lock that never expires; the payload's `expiry` is an **advisory** bound (the claim UI refuses
  a stale link; the inviter is prompted to reclaim). True "revocation" is the inviter racing to
  *reclaim* the unclaimed lock (a race it can lose if the link already leaked — §8 Finding 6). A
  dedicated revoke UI is a follow-up.

---

## 2. The model — two roles, three on-chain acts

1. **Inviter (Bob, has funds + identity).**
   - Derives a one-time ECDSA **voucher key** at the DIP-13 invitation path
     `m/9'/coin'/5'/3'/funding_index'` (sub-feature `3'`).
   - Builds + broadcasts an **asset lock** paying `amount` duffs to that key, and waits for an
     **InstantSend** proof (§5.1 — fast, self-contained; a short IS-scoped expiry covers
     staleness).
   - **Optionally ticks "send a contact request back to me"** — if checked, the link carries the
     inviter's identity id + username; if not, it's a pure funding voucher.
   - Emits a `dashpay://invite?...` link carrying: **voucher private key**, **asset-lock
     proof (IS)**, **advisory expiry**, and *(if opted in)* **inviter identity id + username +
     display name**. The voucher key is re-derivable from `funding_index`, so it is **never
     persisted**; only the funding index + outpoint are tracked (for recovery + status).
2. **Invitee (Carol, no funds).**
   - Opens the link → decodes (voucher key, proof, optional inviter info).
   - Registers **her own new identity** with keys derived from **her** seed at
     `m/9'/coin'/5'/0'/0'/identity_index'/…`, funded by the imported `(proof, voucher_key)` via
     the SDK's in-process raw-key path (§5.2). No L1 Dash on Carol's side.
   - **If the link carries inviter info, Carol is *asked* "establish contact with \<sender\>?"** —
     on confirm, a *normal* contactRequest Carol→Bob is sent via the shipped
     `send_contact_request` path; Bob sees it in his Requests and accepts. Opt-in on both ends
     (inviter checkbox + invitee prompt); no bearer auto-accept key is embedded.

> **Design change from the first draft (security review Finding 1 + reference behavior).** The
> first draft embedded a DIP-15 auto-accept `dapk` in the link so the contact would auto-establish
> with zero taps on the inviter. That is **removed**: auto-accept's safety rests entirely on a
> **1-hour TTL**, which is fundamentally incompatible with an invitation that is claimed hours-to-
> days later — a link long-lived enough to be useful would be a long-lived auto-accept bearer
> credential against the inviter (anyone finding a stale/posted link could make the inviter
> publish an encrypted friendship xpub to them). The production wallets don't do this either:
> their claim flow (`sendContactRequestToInviterUsingInvitationURL`) sends a **plain** contact
> request. So v1 auto-sends a normal contactRequest; zero-tap acceptance is the inviter's own
> orthogonal auto-accept setting, not baked into the shared link. (Embedding a short-TTL dapk with
> an explicit "expired → manual request" fallback is a possible v2 nicety — deferred.)

The consensus acts (asset lock, IdentityCreate, contactRequest) are all already implemented and
tested; invitations are the **orchestration + off-chain envelope + key-handoff** around them.

---

## 3. What already exists (reuse inventory — first-hand code read)

| Capability | Where | Reused for |
|---|---|---|
| **Invitation funding derivation** `AssetLockFundingType::IdentityInvitation` (sub-feature `3'`), `accounts.identity_invitation` xpub, storage/recovery/persistence all wired | `asset_lock/build.rs:200-216` (`peek_next_funding_address`), storage `schema/accounts.rs`, `asset_lock/sync/recovery.rs:427`, `persistence.rs:3633` | **Create**: derive the voucher key + build the voucher asset lock |
| **Full funded-asset-lock flow** `create_funded_asset_lock_proof(amount, account_index, funding_type, identity_index, signer) -> (AssetLockProof, DerivationPath, OutPoint)` (build → track → broadcast → IS wait → CL-upgrade → attach proof) | `asset_lock/build.rs:305-417` | **Create**: build the voucher lock |
| **IS→CL upgrade** `upgrade_to_chain_lock_proof(out_point, None)` | `identity/network/registration.rs:186-197,247-250` | **Create**: force a CL proof before export |
| **Register identity from a raw asset-lock private key** `Identity::put_to_platform_and_wait_for_response_with_private_key(sdk, proof, asset_lock_proof_private_key: &PrivateKey, identity_signer, settings)` | `rs-sdk/.../put_identity.rs:50-59,146+` | **Claim**: register invitee identity funded by the imported voucher — **core claim needs no new SDK code** |
| **Bare claim FFI (external proof + one-time key)** `dash_sdk_identity_put_to_platform_with_instant_lock` / `_with_chain_lock(sdk, …proof bytes…, private_key:[u8;32], signer, settings)` | `rs-sdk-ffi/src/identity/put.rs:29,211` | Lower layer under the platform-wallet `claim_invitation` wrapper (no Swift binding yet) |
| **`AssetLockProof::Instant` embeds the full tx + islock** (self-contained); `Chain` = outpoint+height (Platform resolves tx) | `asset_lock_proof/instant/…:38`, `…/chain/…:24` | **Link**: serialize the proof directly — no separate txid + L1 fetch |
| **Consensus verifies the create sig against the asset-lock output's P2PKH hash** | `identity_create/state/v0/mod.rs:222-245` | Security trust anchor (§8): holder of the voucher key == who may create the identity |
| **Seedless register (self-funded)** `register_identity_with_funding(AssetLockFunding, identity_index, keys_map, identity_signer, asset_lock_signer, …)` | `identity/network/registration.rs:121` | Template; claim uses the raw-key variant instead |
| **Sanctioned raw-scalar export (path-gated)** `ContactCryptoProvider::export_auto_accept_private_key(&path)` / resolver hook | `contact_requests.rs:63`, `mnemonic_resolver_core_signer.rs:353` | **Create**: template for the new path-gated `export_invitation_private_key` (§5.3) |
| **Send a normal contactRequest** `platform_wallet_send_contact_request_with_signer(...)` | FFI `dashpay.rs:225` | **Claim**: auto-send the plain contact-bootstrap invitee→inviter (no dapk) |
| **Register/resume identity FFI (external signer)** `platform_wallet_register_identity_with_funding_signer`, `platform_wallet_resume_identity_with_existing_asset_lock_signer` | FFI `identity_registration_funded_with_signer.rs` | Template for the new claim FFI marshaling |
| **Asset-lock build FFI + tracked-lock listing** `asset_lock_manager_build_transaction`, `create_funded_proof`, `list_tracked_locks` | FFI `asset_lock/build.rs`, `asset_lock/manager.rs` | Create FFI + inviter-side status |

**Net: the funding-derivation family and both consensus signing paths already exist.** The new
code is (a) the create orchestration + voucher-key export, (b) the claim orchestration, (c) the
`dashpay://invite` envelope codec, (d) inviter-side invitation persistence, (e) FFI + Swift + UI.

---

## 4. Interface / data flow per layer

### 4.1 Rust — new module `wallet/identity/network/invitation.rs` (+ codec in `crypto/invitation.rs`)

**Create (inviter):**
```
async fn create_invitation<AS, CP>(
    &self,
    amount_duffs: u64,            // rejected if 0 or > MAX_INVITATION_DUFFS
    funding_account_index: u32,   // BIP44 account supplying the L1 UTXOs
    inviter: Option<InviterInfo>, // id + username + display_name (contact-bootstrap)
    expiry_unix: u32,             // advisory; the FFI sets now + MAX_INVITATION_TTL_SECS
    asset_lock_signer: &AS,       // funds the asset-lock (funding-input + credit-output)
    crypto_provider: &CP,         // exports the voucher scalar (path-gated resolver)
) -> Result<Invitation, PlatformWalletError>
```
where `inviter: Option<InviterInfo>` is `Some` only when the inviter ticked "send a
contact request back to me" (§ owner decision). Steps: (1) **bound the amount**
(`0 < amount_duffs ≤ MAX_INVITATION_DUFFS`) and the expiry (non-zero), else err;
(2) `create_funded_asset_lock_proof(amount, funding_account_index, IdentityInvitation, signer)`
→ `(IS proof, path, out_point)` — **the builder auto-selects the next unused funding index** and
returns its derivation `path`; **keep the IS proof, no CL upgrade** (§5.1); (3) **export the
voucher private key** via the seedless resolver hook, **path-gated to the fully-hardened
`9'/coin'/5'/3'/idx'`** (§5.3); (4) build the `Invitation` struct + `dashpay://invite` URI (§6);
(5) **persist an invitation record** through the wallet persister (§4.2) — created status,
outpoint, funding_index (from `path`), amount, expiry, optional inviter info; **the voucher key is
never persisted** (re-derived from `funding_index`).

**Claim (invitee):**
```
async fn claim_invitation(
    &self,
    invitation: ParsedInvitation,   // decoded from the URI
    identity_index: u32,
    keys_map: BTreeMap<u32, IdentityPublicKey>,  // invitee's own new-identity keys
    identity_signer: &IS,           // invitee's identity-key signer
    establish_contact: bool,        // invitee's answer to "establish contact with <sender>?"
) -> Result<Identity, PlatformWalletError>
```
Claim **bypasses the wallet's `AssetLockFunding` machinery** — the deliberately-removed
`UseAssetLock` variant (external proof through the tracked-lock resolver) is *not* revived; the
invitee owns neither the lock's inputs nor its tracking and can't drive its IS→CL fallback, so
claim submits the imported proof directly. Steps: (1) **validate the parsed invitation before
any network act** (§8 Finding 5): proof is an **Instant** proof; the voucher pubkey is the
credit-output's P2PKH target (`proof.output() → credit_outputs[output_index]`); expiry not
past — fail loud with a specific error otherwise; (2) build the placeholder `Identity` with
`keys_map`; (3)
`placeholder.put_to_platform_and_wait_for_response_with_private_key(&sdk, invitation.proof,
&invitation.voucher_key, identity_signer, settings)` → new `Identity` — **wrap this submit in
`submit_with_cl_height_retry`** (feasibility Note A): the direct raw-key SDK call bypasses
`register_identity_with_funding`, so it doesn't inherit that helper's retry on a transient
CL-height-too-low (10506); without the wrapper a transient reject is a hard claim failure; (4)
local bookkeeping
(add to IdentityManager, breadcrumbs) — best-effort, non-propagating (mirrors
`register_identity_with_funding` Step 4); (5) if `invitation.inviter` present **and
`establish_contact`** (the invitee said yes to the prompt), **send a normal contactRequest**
invitee→inviter via the shipped `send_contact_request` path (the new invitee identity as
sender). Idempotent/re-sendable if step 5 fails after step 3 succeeds (§10). If the invitee
declines, the identity is still created — just no contact.

### 4.2 Rust — inviter-side persistence (proper persister integration — owner decision)
**A first-class persisted invitation record, through the existing wallet persister system**
(not an ad-hoc KV blob). Follow the established DashPay changeset → persister → SwiftData-model
pattern already used for contact requests / payments (`rs-platform-wallet` changeset overlays +
`rs-platform-wallet-storage` migration + the Swift `Persistence/Models` `@Query` models —
research-swift map). Concretely:
- **Rust storage (`rs-platform-wallet-storage`):** a new `invitations` table via a migration
  (mirroring `asset_locks` `V001__initial.rs:247`), columns `wallet_id, outpoint, funding_index,
  amount_duffs, expiry_unix, status (created|claimed|reclaimed), inviter_opt_in, created_at,
  claimed_identity_id?`. **No secret column** — the voucher key is re-derived from `funding_index`
  (§5.3), never stored.
- **Rust changeset (`rs-platform-wallet`):** an `InvitationChangeSet` emitted by create/reclaim
  and by the sync that flips *created → claimed* (detected by the tracked asset-lock's outpoint
  being consumed on Platform / the invitee's inbound contactRequest), queued onto the persister
  exactly like `AssetLockChangeSet` / the DashPay overlays.
- **Swift:** a `PersistentInvitation` SwiftData model registered in `DashModelContainer`, driving
  a `@Query` "Sent invitations" list (`InvitationsView`).

Recovery still leans on re-derivation: an unclaimed invitation's voucher key is re-derived from
its `funding_index` to re-package or reclaim (the asset-lock row already tracks the lock's
lifecycle for the actual reclaim submit). The invitations table adds the durable, queryable
*status* surface the UI needs.

### 4.3 FFI (rs-platform-wallet-ffi) — new `invitation.rs`
- `platform_wallet_create_invitation(wallet, amount_duffs, funding_account_index,
  inviter_identity_id: *const [u8;32] /*nullable*/, inviter_username: *const c_char /*nullable*/,
  expiry_unix: u32, core_signer_handle, out_uri: **c_char, out_outpoint: *mut OutPointFFI)
  -> Result`. **Only `core_signer_handle`** (the asset-lock/Core signer) is needed — pure voucher
  creation registers no identity, so there is no identity `signer_handle` (feasibility Note B).
  `now`/`expiry_unix` is passed in from Swift (FFI can't read the clock deterministically — same
  convention as `build_auto_accept_qr`).
- `platform_wallet_claim_invitation(wallet, uri: *const c_char, identity_index,
  identity_pubkeys, identity_pubkeys_count, signer_handle /*invitee identity signer*/,
  establish_contact: bool, out_identity_id: *mut [u8;32], out_identity_handle: *mut Handle)
  -> Result`. `establish_contact` is the invitee's answer to the "establish contact with
  \<sender\>?" prompt (only acted on if the link carries inviter info). Reuses
  `decode_identity_pubkeys` + the managed-identity insert from
  `identity_registration_funded_with_signer.rs`. Note: a **bare** identity-create-from-external-
  proof FFI already exists one layer down — `dash_sdk_identity_put_to_platform_with_chain_lock`
  / `..._with_instant_lock(sdk, …proof bytes…, private_key: *const [u8;32], signer, settings)`
  (`rs-sdk-ffi/src/identity/put.rs:29,211`). We do **not** call that bare FFI from Swift for
  claim: the platform-wallet `claim_invitation` wrapper is needed so the new invitee identity is
  registered in the wallet's `ManagedIdentity` storage **and** the contact-bootstrap fires — it
  calls `put_to_platform_and_wait_for_response_with_private_key` internally, then does bookkeeping
  + the bootstrap send. (No `core_signer_handle` is needed on claim: the asset-lock signature
  uses the imported raw voucher key, not a wallet-derived one.)
- `platform_wallet_list_invitations(...)` + free helpers for the inviter status list.
- String/URI input validation identical to the auto-accept FFIs (null checks, length caps).

### 4.4 Swift (swift-sdk + SwiftExampleApp)
Current services (note: `PlatformService`/`WalletService`/`UnifiedAppState` were **removed**):
`AppState` (owns the `SDK`, network), `PlatformWalletManager` (per-network, DashPay sync
lifecycle), `ManagedPlatformWallet` (**all identity/DashPay FFI calls live here**). **All Swift
↔ Rust FFI work MUST go through the `swift-rust-ffi-engineer` agent** (repo `CLAUDE.md` rule).
The **DIP-15 auto-accept QR flow is the copy-template** for both directions.
- swift-sdk wrappers on `ManagedPlatformWallet`:
  - `createInvitation(amountDuffs:fundingAccount:expiry:) async throws -> InvitationLink`
    (idiom of `registerIdentityWithFunding` `ManagedPlatformWallet.swift:3370` — long-running L1
    build, so wrap with a Controller+Coordinator triad like `IdentityRegistrationController`).
  - `claimInvitation(uri:identityIndex:) async throws -> ManagedIdentity` (idiom of
    `sendContactRequestFromQR` `:1758`).
- SwiftExampleApp UI (under the DashPay tab, `App/Views/DashPay/`):
  - **Create**: a "Create invitation" action (beside "Add me QR" in `DashPayProfileView.swift:74`)
    → amount entry **+ a "send a contact request back to me" checkbox** (drives the optional
    inviter info) → share sheet with the link + a QR (reuse `generateQRCode`).
  - **Claim**: a toolbar button + sheet mirroring `AddViaQRSheet` (`DashPayTabView.swift:830`)
    (paste/scan the `dashpay://invite` link) → register identity → **if the link carries inviter
    info, prompt "establish contact with \<sender\>?"** → pass the answer as `establish_contact` →
    `kickDashPaySync` → the new identity (+ optional contact) land via `@Query`.
  - **Invitations list** (created + status): a new `InvitationsView` (`@Query` over
    `PersistentInvitation`, §4.2), reached via a toolbar `NavigationLink` (like the Ignored link
    at `:151`).
  - **Deep link (net-new plumbing):** no `onOpenURL`/`CFBundleURLTypes` exist today. Add the
    `dashpay` URL scheme to `SwiftExampleApp/Info.plist` and `.onOpenURL { … }` on the
    `WindowGroup` in `SwiftExampleAppApp.swift:105`, routing to `RootTab.dashpay` + the claim
    sheet; reuse the `AddViaQRSheet` URI-parse as the model.
- `FundingType.identityInvitation = 3` already exists in Swift
  (`ManagedAssetLockManager.swift:36`, `KeyWalletTypes.swift:14`).
- **Framework build:** `DashSDKFFI.xcframework` is a generated artifact (not committed); rebuild
  via `packages/swift-sdk/build_ios.sh --target sim` after any FFI/header change, then the
  `xcodebuild` app build (§ repo CLAUDE.md). Always clean+rebuild after header changes.

### 4.5 QA contract
The authoritative QA contract is **`packages/swift-sdk/SwiftExampleApp/TEST_PLAN.md`** (driven by
the `simulator-control` skill; dashboard at `dashpay.github.io/qa-dashboard-site`). Add rows to
**§4.10 DashPay** as **DP-12+** in the existing format:
`| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |`. Planned rows:
- `DP-12 | Create invitation | Cross | Common | … | funding | DashPay → Create invitation → platform_wallet_create_invitation (builds L1 asset lock; needs testnet funds).`
- `DP-13 | Claim invitation | Platform | Common | … | | Paste/scan dashpay://invite → platform_wallet_claim_invitation → new identity + contact.`
- `DP-14 | Invite→claim e2e (two wallets) | Cross | Thorough | … | multiwallet | Create on A, claim on B, contact auto-establishes both ends (cf. DP-11).`
- `DP-15 | Reject malformed / already-claimed invitation | Platform | Uncommon | … | | Bad link + reused link both fail loudly, no side effects.`
(Secondary: the `AI_QA/` MCP playbooks — add a `QA004`-style invite→claim walkthrough if useful.)

---

## 5. The three technical cruxes (de-risked first-hand; §11 spikes confirm)

### 5.1 Proof type — DECIDED: InstantSend (owner decision 2026-07-08)
`AssetLockProof` has two variants with very different self-containment (confirmed
`asset_lock_proof/mod.rs:40`):
- **`InstantAssetLockProof { instant_lock, transaction, output_index }`** — embeds the **full
  funding tx + the InstantLock**. Self-contained (Platform validates the islock against the
  embedded tx). This is what the **reference iOS/Android wallets export** (`islock` + they carry
  the txid and re-fetch the tx). Fast to produce (just wait for the IS lock). **Risk:** Platform
  rejects an islock whose quorum has rotated or is too old relative to Platform's core height
  (`is_instant_lock_proof_invalid` + the IS→CL retry in `registration.rs`). An invitation that
  sits **unclaimed** for a long time can go stale.
- **`ChainAssetLockProof { core_chain_locked_height, out_point }`** — tiny (outpoint + height);
  Platform resolves the tx from Core by outpoint. **No staleness window** (chain-locked is
  permanent), so an unclaimed invitation stays valid indefinitely. Cost: the inviter waits for a
  ChainLock at create (≈ up to a block or two, low-minutes).

**DECISION (owner, 2026-07-08): export an InstantSend proof.** Faster create (no CL wait), matches
the reference wallets, and the `InstantAssetLockProof` embeds the full tx + islock so the link is
fully self-contained (the invitee never fetches anything from L1). `create_funded_asset_lock_proof`
returns exactly this for a fresh tx (its `validate_or_upgrade_proof` only upgrades to CL when the
tx is *old* — not the case at create), so the invitation path **keeps the IS proof, no forced CL
upgrade**.

> **Slow-IS fallback must be enforced (Rust-core review H1).** `create_funded_asset_lock_proof`
> *also* falls back to a ChainLock proof if the IS lock doesn't propagate within its 300s
> preference window. Since the invitee's `validate_claimable` accepts only an InstantSend proof,
> `create_invitation` **must reject a returned ChainLock proof** — else it would emit a
> `dashpay://invite` link the invitee silently rejects (a dead voucher: funds locked, no signal).
> On this rare path create returns a clear error; the funding lock stays tracked/reclaimable, and
> the inviter retries. *(A future robustness option is to accept a Chain proof on claim too —
> it never goes stale — skipping the local credit-output pre-check since a Chain proof carries no
> embedded tx; deferred, as it deviates from the literal Instant-only decision.)*

**Staleness mitigation = a short, IS-scoped advisory expiry (not an IS→CL upgrade in v1).** The
one real risk is that Platform rejects a *stale* islock (quorum rotated). Rather than build an
invitee-side IS→CL upgrade (which needs the embedded tx re-tracked — non-trivial, and the
external-proof `UseAssetLock` path was deliberately removed), v1 sets the invitation's advisory
`expiry` conservatively **inside the IS validity window** (default ~24h, ≤ `MAX_INVITATION_TTL`):
the claim path refuses a past-expiry link up front with a clear "invitation expired — ask the
sender for a new one," so an about-to-go-stale proof is never submitted. Cheap, no fund risk (the
inviter simply re-creates), and the inviter's asset lock is reclaimable after expiry. **Future
enhancement (not v1):** an invitee-side IS→CL upgrade from the embedded tx to extend the window to
days/weeks. *(Note: this makes the create FFI's identity-signer moot as before, and the claim's
`submit_with_cl_height_retry` wrapper — feasibility Note A — still applies to the IS submit.)*

### 5.2 Claim is ordinary identity registration with imported funding
`put_to_platform_and_wait_for_response_with_private_key(proof, voucher_key, identity_signer)`
already does exactly what claim needs. The invitee's identity keys come from the invitee's own
seed (normal registration); only the **funding** `(proof, voucher_key)` is imported. **No new
SDK code for the core claim.** The `identity_invitation` account is an inviter-only concept —
the invitee never derives sub-feature `3'`.

### 5.3 Exporting the voucher private key is a deliberate bearer-credential export
The architecture's invariant is "private keys never cross the FFI boundary as raw bytes," and
the signer-driven builder deliberately **withholds** the credit-output private key (it returns
`AssetLockCreditKeys::Public((pubkey, path))`, `build.rs:117`). The invitation **is** a raw-key
handoff (the whole point), so exporting it is a scoped, documented exception — exactly like the
auto-accept `dapk` blob, which already exports a bearer private key in a QR.

**Key choice:** **HD-derived at `m/9'/coin'/5'/3'/index'`** (not a JS-style random key). HD makes
it DIP-13-recoverable — the wallet can re-derive/scan unclaimed invitation funding txs and let
the user reclaim/resend (DIP-13's explicit recommendation) — at the cost of needing an export
step. (A random ephemeral key, JS-SDK precedent `createAssetLockTransaction.ts:26`, exports
trivially but is unrecoverable; rejected.)

**Export = a NEW seedless resolver hook, path-gated to the exact invitation sub-feature
(security review Finding 2 — normative).** The create FFI is **seedless** (it drives a
`MnemonicResolverCoreSigner`, not a resident `Wallet`), so there is no `&Wallet` to
`derive_extended_private_key` on for the real host — v1 must add a raw-scalar export on the
resolver, exactly mirroring the sanctioned precedent
`export_auto_accept_private_key(&path) -> SecretKey` (`mnemonic_resolver_core_signer.rs:353`,
`ContactCryptoProvider` `contact_requests.rs:63`). **The new `export_invitation_private_key(&path)`
MUST gate on the full path** `comps.len()==5 && comps[0]==9' && comps[2]==5' && comps[3]==3'` —
**not** merely `comps[2]==5'`, because feature `5'` is shared with identity-registration
(`5'/0'`,`5'/1'`), top-up (`5'/2'`), etc.; a loose gate would let a caller exfiltrate the user's
**own** identity-funding keys. Add a negative test mirroring
`export_auto_accept_private_key_gates_to_the_auto_accept_path`.

**Never persist the key.** Because it is HD-derived, the inviter re-derives it from the seed
whenever it re-packages or reclaims. Storage tracks only funding index + outpoint (§4.2). The
returned URI (which *contains* the plaintext key) is treated as a secret end-to-end: no logging,
no analytics, sensitive-pasteboard flag on the Swift side (§8 Finding 3).

> **This export hook is v1 critical path, not a follow-up (feasibility Finding 5, BLOCKING).**
> Production/example-app wallets are **seedless at steady state** (`Wallet::new_external_signable`,
> no root key — `persistence.rs:158-163`); only the *first-ever* session has a resident seed. So
> the "derive from a resident `Wallet`" idea is a **dead end**: create a wallet Monday (seed
> resident), relaunch Tuesday (external-signable) → tap "Create invitation" →
> `wallet.derive_extended_private_key(path)` errors and the existing
> `export_auto_accept_private_key` rejects the `5'/3'` path (it gates to `16'`), so **no link can
> be produced.** The fix is the new gated `export_invitation_private_key` on
> `MnemonicResolverCoreSigner` + a `ContactCryptoProvider`-style method (seedless + seed impls,
> cf. `contact_requests.rs:63/188`) + its FFI — a dedicated implementation slice (§13 slice 2).

---

## 6. The `dashpay://invite` link envelope — a single versioned blob

> **SUPERSEDED (2026-07-13, §0.1):** the shipped envelope is the legacy query format
> (`du`/`assetlocktx`/`pk`/`islock` — see §0A), not this blob.
> Kept for the design rationale it records (secret handling, caps, transport notes still apply).

**Decision: one opaque, versioned payload** behind a `dashpay://invite?data=<base58(payload)>`
deep link (keeping the reference's `dashpay://invite` scheme name for familiarity), **not** the
reference's six loose query params. Rationale in §7. The payload is a small versioned blob in a
**hand-rolled little-endian binary encoding** (deliberately *not* serde/bincode — the crate's
`serde` feature is optional and off, and `AssetLockProof` is internally-tagged so bincode-serde
rejects it), so the envelope can evolve without breaking older links. The as-built wire order
(see `crypto/invitation.rs`) is:

```text
wire = version:u8               // = 0
     ‖ voucher_key:[u8; 32]     // one-time ECDSA private key (secret; zeroized)
     ‖ expiry_unix:u32(LE)      // ADVISORY, IS-scoped (§5.1); not consensus
     ‖ inviter_present:u8       // 0 = none, 1 = InviterInfo follows
       [ identity_id:[u8; 32]
       ‖ username:len-prefixed  // DPNS name (whom the invitee's contactRequest targets)
       ‖ display_present:u8 [ display_name:len-prefixed ] ]
     ‖ asset_lock:len-prefixed  // InstantSend proof (§5.1) — embeds tx + islock; LAST, length-prefixed
       // NO auto-accept dapk — v1 sends a normal contactRequest, invitee-confirmed (§2)
```
- Serializing the `InstantAssetLockProof` directly means the link **embeds the full funding tx +
  islock**, so the invitee needs **no L1 tx fetch** (an improvement over the reference, which
  carried only the txid). Link size is a few hundred bytes → base58 ~a few hundred chars: fine
  for a deep link and a QR.
- **Length-cap the `data=` param before decode (§8 Finding 5, LOW).** The base58-**char** cap on
  the input *before* decoding is the DoS mitigation (mirrors the `dapk` cap in
  `parse_dashpay_contact_uri`). Note: `AssetLockProof`'s consensus bincode decode is **already
  bounded and panic-free** on arbitrary bytes (dashcore `MAX_VEC_SIZE`, finite cursor, all
  `Result`-based — verified), so the residual is only "a huge blob is fully buffered," which the
  pre-decode char cap closes. A fuzz test is cheap insurance, not a blocker.
- The codec pair in `crypto/invitation.rs` is
  `encode_invitation_uri(voucher_key: &SecretKey, asset_lock: &AssetLockProof, expiry_unix: u32,
  inviter: Option<&InviterInfo>) -> Result<String, _>` and
  `parse_invitation_uri(uri: &str) -> Result<ParsedInvitation, _>`, fully unit-tested (round-trip +
  every malformed rejection). A plain `https://…` fallback host can wrap the same `?data=` for
  users without the app installed — deferred (no hosting in v1; see §6.1).

### 6.1 Transport security & the custom-scheme limitation

The `data=` payload is a **bearer credential**: whoever reads the plaintext link controls the
voucher and can claim (front-run) it. Because the app registers the `dashpay://` **custom URL
scheme**, any other app that also registers `dashpay` can intercept an invite link on the same
device and steal the claim. The load-bearing mitigation is therefore **economic, not transport**:
`MAX_INVITATION_DUFFS` caps the loss at 0.01 DASH, and the inviter can reclaim an unclaimed voucher
(best-effort race). The advisory expiry does **not** bound a leak (a leaked-link holder ignores it).

A hardened production transport would use **Universal Links** (HTTPS + a hosted
`apple-app-site-association`, `associated-domains` entitlement) or another verified handoff so the
OS can't hand the link to an impostor app. That is **out of scope for this example app** — it
needs hosting infrastructure the sample doesn't have, and the amount cap already bounds the blast
radius — but it is the recommended path for the production wallet and is tracked as a follow-up.
The `?data=` shape is transport-agnostic, so moving from the custom scheme to a Universal Link is a
routing change, not an envelope change.

---

## 7. Interop decision — ~~RESOLVED: ship our own self-contained envelope~~

> **REVERSED (2026-07-13, §0.1):** the as-built codec adopts the reference wallets' legacy
> query format for field-level cross-claimability with dash-wallet iOS/Android. The analysis
> below (dead FDL delivery, JS SDK never had invitations) remains accurate — only the
> conclusion changed, by owner decision, once cross-wallet claimability was prioritized.
Research (research-reference, primary sources) settled this:
- The production iOS (DashSync) + Android (dash-wallet) wallets use an **identical plaintext
  URL-query payload**: `du` (username), `display-name`, `avatar-url`, `assetlocktx` (**txid
  only**, 64-hex), `pk` (**WIF** private key), `islock` (hex InstantLock). The invitee **fetches
  the full funding tx from L1 by txid**, then registers using the embedded islock.
- That link was distributed via **Firebase Dynamic Links**, which **Google shut down
  2025-08-25** — the hosted `invitations.dashpay.io/link` short-links now **404**. So even the
  production wallets' *share layer is already broken* and must be reworked.
- **The JS SDK never had an invitation API** — invitations existed only in the two native apps.

**Conclusion:** there is little value matching a legacy wire format whose delivery mechanism is
dead. We ship our own **self-contained, versioned** envelope (§6). The **only** things we must
NOT diverge on are the **on-chain / consensus** semantics — the DIP-13 `3'` derivation and the
islock / asset-lock-proof shapes Platform consensus accepts — because those are what actually
interoperate. This mirrors the auto-accept spec's "iOS-first, DIP-faithful where defined,
normative-for-us where silent" stance. (If byte-interop with a future reworked DashWallet is ever
required, matching is a localized codec change; the on-chain acts already interoperate.)

---

## 8. Security
*(Folds a 4-lens security review: no CRITICALs — the core crypto is sound; findings are must-fix
hardening + honest-framing fixes. Verified-clean floor: in-flight IdentityCreate is
non-malleable, double-claim is deterministic, the invitee never risks its own funds.)*

- **Consensus trust anchor (why this is safe at all).** Platform validates the IdentityCreate's
  outer signature against the **asset-lock output's P2PKH public-key hash**
  (`identity_create/state/v0/mod.rs:222-245`) and the identity id is `hash(outpoint)` — so a
  network observer who does *not* hold the voucher key cannot swap in their own keys and steal an
  in-flight claim, and two racers target the *same* id (consensus commits exactly one). Every
  claim-theft attack reduces to **"who holds the link."** The invitee's own identity keys sign
  the per-key witnesses separately.
- **Bearer credential — the load-bearing leak mitigation is the amount cap + reclaim, NOT the
  expiry (Rust-security-review LOW-2 honesty fix):**
  - **Amount cap enforced in Rust (Finding 4).** `create_invitation` rejects
    `amount_duffs > MAX_INVITATION_DUFFS` — the *actual* bound on a leaked link's blast radius
    (a direct FFI caller / headless host / UI bug can't exceed it). Never UI-only.
  - **Expiry is a UX / reclaim signal, not a leak bound.** A malicious *finder* of a leaked link
    holds the voucher key + proof and can submit directly, **ignoring the honest UI's expiry
    check** — so `expiry_unix` does not bound a leaked-link window. What it *does* do: (a) stop an
    **honest** invitee from submitting an about-to-go-stale IS proof (§5.1), and (b) give the
    inviter a clear reclaim-after signal. Advisory, not consensus. (The FFI sets a sensible
    default expiry from `MAX_INVITATION_TTL_SECS`; clamping it in Rust is symmetry, not security.)
  - **Single-use** (asset lock consumed on first claim → deterministic reject thereafter), funds
    are the inviter's to give; the inviter can race to **reclaim** an unclaimed voucher (a race it
    can lose if already leaked — §8 Finding 6).
- **The link is plaintext key material — treat the URI as secret end-to-end (Finding 3).** The
  create FFI returns the URI (which *contains* the voucher key) as a C string that flows through
  Swift + a `dashpay://invite` deep-link handler (handlers routinely log URLs) + clipboard
  (iOS Universal Clipboard syncs across devices) + the share sheet. Requirements: **no logging /
  no analytics** of the URI; secret/`Zeroizing` types Rust-side; a **sensitive-pasteboard** flag
  Swift-side; the voucher key is **never persisted** (re-derived from `funding_index`, §5.3).
- **Inviter self-claim / front-run is a real griefing/DoS vector against the invitee (Finding 6 —
  honesty fix).** *Not* "no third-party risk." The inviter can front-run or reclaim after handoff,
  denying the invitee onboarding mid-flow with no signal it was the inviter's doing. No fund theft
  (funds are the inviter's), but real denial. Likewise **"reclaim = revocation" is a race the
  inviter can lose** if the link already leaked — reclaim is best-effort, and the advisory expiry
  is the actual bound. Documented as an accepted, honestly-stated limitation.
- **Untrusted proof on claim — validate before submit (Finding 5, LOW after re-verify).** The
  `AssetLockProof` bincode decode is already bounded/panic-free; the §6 pre-decode length cap is
  the DoS mitigation (keep it). The genuinely useful part is **fail-fast UX, not a security gap**:
  cheap **local pre-submit checks** — the proof is an **Instant** proof (§5.1), the advisory
  expiry is not past, and the **voucher pubkey-hash ∈ the selected credit output**
  (`proof.output() → credit_outputs[output_index]`) — so a malformed/hostile/stale link fails with
  a clear error instead of an opaque consensus reject. The
  credit-output-pubkey binding is itself consensus-enforced, so this cannot be *bypassed* to steal;
  it only improves the error.
- **Unauthenticated envelope (Finding 7 — documented, no v1 fix).** Nothing signs the bundle, so a
  MITM on the *link channel* can substitute the whole invite. Blast radius is limited (the
  contact only forms toward whatever inviter identity is in the link; an attacker can at most make
  the invitee contact the attacker's own identity — achievable with a normal contact request
  anyway). Reduces to "bearer-link trust = channel trust"; envelope signing wouldn't help (the
  channel is the trust root).
- **Privacy (Finding 8, LOW).** Because id = `hash(outpoint)`, the inviter knows the invitee's
  future identity id before they claim, and that id is inviter-chosen. Noted.
- **Malformed / hostile link:** every field size-capped before decode; a bad link fails loudly
  with no side effects.

---

## 9. Decisions (RESOLVED — owner, 2026-07-08)
1. **Proof type: InstantSend** (§5.1). Fast create, self-contained link; staleness covered by a
   short IS-scoped advisory expiry (claim refuses past-expiry), not an IS→CL upgrade in v1.
2. **Contact-bootstrap: opt-in on both ends.** Inviter ticks "send a contact request back to me"
   (→ inviter info in the link); the invitee is *asked* "establish contact with \<sender\>?" at
   claim and only then is a normal contactRequest sent. In v1. No auto-accept dapk (§8 Finding 1).
3. **Inviter persistence: proper wallet-persister integration** (§4.2) — a first-class
   `invitations` table + changeset + `PersistentInvitation` SwiftData model, not a KV blob. In v1.
4. **Link scheme:** our own self-contained versioned blob (§7).
5. **Amount / TTL:** Rust-enforced `MAX_INVITATION_DUFFS` (default a sensible identity-reg +
   small-balance amount; confirm exact duffs during spikes) and `MAX_INVITATION_TTL` bounded to
   the **IS validity window** (default ~24h) since the proof is InstantSend.

---

## 10. Failure modes
- **Insufficient inviter balance to fund the lock** → create fails pre-broadcast, funds
  untouched (reservation released — existing `create_funded_asset_lock_proof` rejection path).
- **InstantSend lock never arrives at create** → `create_funded_asset_lock_proof`'s 300s IS wait
  elapses and (for a fresh tx) it surfaces an error; the tracked lock is resumable (inviter can
  retry or reclaim). We do **not** force a CL upgrade (§5.1).
- **Stale IS proof (claimed too late)** → the advisory expiry makes the claim refuse *before* the
  IS lock could be rejected by Platform; the inviter re-creates. (Extending the window via an
  invitee-side IS→CL upgrade is a post-v1 enhancement.)
- **Invitee claims an already-claimed / inviter-front-run link** → Platform rejects (lock
  consumed); claim returns a clear "invitation already used" error; no identity created. (This is
  also the inviter-front-run griefing outcome, §8 Finding 6.)
- **Malicious inviter hands a mismatched/IS/expired proof** → caught by the claim pre-submit
  checks (§4.1 step 1 / §8 Finding 5) → fail loud, no blind submit.
- **Claim interrupted after identity created but before contact-bootstrap sent** → the identity
  exists (self-heals into the invitee's IdentityManager on next re-sync); the contact request is
  re-sendable (idempotent — the send path adopts an existing friendship). Not a data-loss path.
- **Malformed / truncated / oversize link** → parse/size-cap error, no side effects.
- **Invitee has no seed / can't derive identity keys** → claim fails before any network act.
- **Voucher never claimed AND inviter loses seed (§8 Finding 9, LOW)** → L1 Dash stranded in the
  lock (asset locks are one-way). Mitigated by HD re-derivation from `funding_index` — this stays
  a generic "lost your seed" problem, not invitation-specific.

---

## 11. Spikes (before implementation — task #11)
1. **S1 — raw-key claim end-to-end (offline):** in a `rs-platform-wallet` integration test,
   build an asset lock at `IdentityInvitation`, derive the voucher key, and drive
   `put_to_platform_and_wait_for_response_with_private_key` against a mock/echo SDK to confirm
   the proof + raw-key + invitee-identity-signer triple registers an identity. Confirms §5.2.
2. **S2 — seedless voucher-key export + path gate:** add `export_invitation_private_key(&path)`
   on the resolver/provider mirroring `export_auto_accept_private_key`
   (`mnemonic_resolver_core_signer.rs:353`), and prove the gate: it exports for
   `9'/coin'/5'/3'/idx'` and **rejects** `9'/coin'/5'/0'/…` (identity-auth), `…/5'/1'/…` (reg
   funding), `…/5'/2'/…` (top-up) — the Finding-2 negative test. Confirms §5.3.
3. **S3 — create keeps the IS proof + persistence round-trip:** confirm
   `create_funded_asset_lock_proof(IdentityInvitation)` returns an **Instant** proof for a fresh
   tx (no auto-upgrade), and that an `InvitationChangeSet` round-trips through the persister
   (`created` row readable back). Confirms §5.1 + §4.2.
4. **S4 — link envelope codec:** implement + unit-test `encode/parse_invitation_uri`
   (round-trip + malformed) — cheap, do first.

---

## 12. Test / verification plan
- **Rust unit:** invitation URI codec (round-trip + every malformed rejection incl. the
  pre-decode length cap); voucher blob round-trip; the **export-path-gate negative test** (§5.3 /
  S2, Finding 2 — the blocking one: exports `5'/3'`, rejects `5'/0'`,`5'/1'`,`5'/2'`,`16'`);
  create-invitation **rejects `amount > MAX_INVITATION_DUFFS` and `expiry > now+MAX_TTL`**
  (Finding 3/4); claim **pre-submit checks reject** a non-Instant proof and a voucher-pubkey ∉
  credit-output (Finding 5 — fail-fast); expired-link rejection. (Optional insurance: a fuzz test
  that `parse_invitation_uri` on arbitrary bytes never panics — not a blocker, decode is already
  bounded.)
- **Rust integration (`rs-platform-wallet`):** the S1 offline flow as a permanent test; the
  create→export→re-derive-from-`funding_index` round-trip (recovery); reclaim-unused path.
- **FFI:** null/oversize/bad-URI input validation; create→parse round-trip; claim marshaling
  (identity handle inserted, id out); assert the URI is not emitted to logs.
- **Swift:** `build_ios.sh` green; wrapper unit tests for encode/decode boundaries.
- **Testnet funded e2e (task #13):** fund an inviter wallet via the **built-in faucet**
  (Wallet → Receive → "request from testnet", `TestnetFaucetService` → `faucet.thepasta.org`) →
  register the inviter identity + DPNS name → `create_invitation` → parse the link in a **second**
  wallet with no funds → `claim_invitation` → assert the invitee identity exists on Platform and
  (if bootstrap) the contact auto-establishes after the inviter's drain. This is the acceptance
  gate. Can run headless (Rust integration against testnet) and/or two-simulator on-device.
- **On-device (two sims):** create on sim A, claim on sim B, contact appears on both.
- **QA contract:** the scenarios from §4.5.

---

## 13. Commit slicing (implementation order)
1. `crypto/invitation.rs` codec (payload struct + `encode/parse_invitation_uri` + length cap) +
   tests (S4).
2. **Voucher-key export (v1 critical path — feasibility Finding 5):** gated
   `export_invitation_private_key` on `MnemonicResolverCoreSigner` (gate `9'/coin'/5'/3'/idx'`) +
   `ContactCryptoProvider`-style method (seedless + seed impls) + the path-gate negative test (S2).
   Without this the seedless host cannot produce a link at all.
3. `network/invitation.rs` create (slice-2 export + keep IS proof + amount/expiry caps) + claim
   (raw-key submit wrapped in CL-height retry + Instant-proof pre-submit checks + optional
   invitee-confirmed contactRequest) helpers + unit tests (S1).
4. **Inviter persistence (§4.2):** `invitations` migration + `InvitationChangeSet` + status sync.
5. FFI `platform_wallet_create_invitation` (core signer only) / `_claim_invitation`
   (`establish_contact` param) + tests (marshaling mirrors `identity_registration_funded_with_signer.rs`).
6. swift-sdk wrappers on `ManagedPlatformWallet` + `PersistentInvitation` SwiftData model (**via
   `swift-rust-ffi-engineer`**).
7. SwiftExampleApp: create sheet (amount + "send request back" checkbox), claim sheet (with the
   "establish contact with \<sender\>?" prompt), `InvitationsView` list, + `dashpay://invite`
   deep-link handler (`Info.plist` scheme + `.onOpenURL`).
8. QA-contract rows (TEST_PLAN.md §4.10 DP-12+).
9. Testnet e2e evidence + docs (`SPEC.md` Milestone 5 as-built, `DIP_CONFORMANCE_GAPS.md` row).

---

## 14. Multi-agent spec-review resolutions (2026-07-08)
Four research streams (wallet/SDK/Swift/reference) + three adversarial spec reviews
(feasibility / security / scope). Folded:
- **Feasibility — core mechanic CONFIRMED** (claim independence proven at `v0_methods.rs:65-78`;
  create/CL/FFI confirmed). **One blocker: seedless voucher-key export** — the resident-`Wallet`
  idea is a dead end (production wallets are `new_external_signable`); promoted to **v1 critical
  slice 2** (§5.3, §13). Should-fixes folded: bounded CL wait (§5.1/§4.1), claim submit wrapped in
  CL-height retry (§4.1), create FFI drops the spurious identity signer (§4.3).
- **Security — no CRITICALs.** Two blockers folded: (1) the **dapk TTL contradiction** →
  auto-accept dropped, plain contactRequest bootstrap (§2); (2) **export path-gating** to
  `9'/coin'/5'/3'/idx'` with a negative test (§5.3). Hardening folded: Rust amount cap, advisory
  voucher expiry, secret/no-log URI (§8 Finding 3/4); honesty fixes (self-claim = griefing/DoS,
  reclaim = a race — §8 Finding 6). Proof-parse worry **downgraded to LOW** on re-verify (bincode
  is already bounded; the length cap is the mitigation; pre-submit checks are fail-fast UX).
- **Reference/interop** — the production link format is dead (FDL shutdown); ship our own
  self-contained versioned envelope, preserve only on-chain semantics (§7).
- **Scope** — scope levers threaded (single versioned blob §6; reuse over new code throughout).
- **Owner decisions (2026-07-08, sync gate):** (1) **InstantSend** proof, not ChainLock —
  staleness handled by a short IS-scoped expiry (§5.1); (2) contact-bootstrap **opt-in on both
  ends** — inviter checkbox + invitee "establish contact?" prompt (§2, §4.1); (3) **proper
  wallet-persister** integration for invitations, not a KV blob (§4.2). All in v1.
