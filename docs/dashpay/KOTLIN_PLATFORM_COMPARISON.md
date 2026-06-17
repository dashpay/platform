# DashPay: our Rust impl vs kotlin-platform / dashj / dash-wallet — deep comparison

Status: **complete** — all 5 areas. Method: 5 parallel agents, each diffing our code against the
canonical Android stack: `dashpay/kotlin-platform` (`dpp` module,
`org.dashj.platform.dashpay`), `dashpay/dashj` (core crypto/keychains), and
`dashpay/dash-wallet` (the app: sync, UI, DAOs). All on `master`. Findings are
code-verified with file:line on both sides.

**`android-dashpay` is the STALE predecessor (last push 2024-01); the live lib is
`kotlin-platform` (`org.dashj.platform:dash-sdk-*`, which dash-wallet depends on).**

---

## Headline (severity-ranked)

| # | Finding | Severity | Area |
|---|---------|----------|------|
| 1 | **`accountReference` ASK28 byte order** — we read `be(ASK[28..32])` (iOS conv.); dashj/Android reads `le(ASK[0..4])`. Proven-different values for identical inputs. | 🔴 **INTEROP-BREAK** | crypto / derivation |
| 2 | **`account'` path segment hardcoded `0'`** — key-wallet drops the account index from the friendship path; dashj derives under the counterparty's real account. Breaks when a counterparty uses account ≠ 0. | 🔴 **INTEROP-BREAK (latent)** | derivation |
| 3 | **Fetch truncates at 100, no pagination/high-water** — >100 contactRequests ⇒ newest buried permanently; dashj drains all via a `startAt` cursor loop. | 🔴 **BUG** | sync |
| 4 | **Sent payments stuck on `Pending` forever** — no `Pending→Confirmed` transition; the `contains_key` guard blocks status updates (only a *test* flips it). | 🔴 **BUG** | payments |
| 5 | **Contact-profile sync entirely absent** — we sync our *own* profile but never fetch contacts' displayName/avatar (`all_identities()` excludes contacts). | 🔴 **BUG / feature gap** | sync |
| 6 | **`update_profile` doesn't merge — wipes sibling fields** — editing one field (e.g. displayName) deletes publicMessage/avatarUrl on-platform; kotlin `Profiles.replace` read-modify-writes. | 🔴 **BUG (data loss)** | profile |
| 7 | **No incremental high-water + 10-min overlap** — re-fetch from start each sweep; lets #3 never self-heal. | 🟠 MISSING | sync |
| 8 | **`encryptedAccountLabel`: no space-padding + omitted when absent** — kotlin always pads to ≥16 chars and always emits; labels <16 chars currently **error** in our code. | 🟠 MISSING | crypto |
| 9 | **No per-contact tx-history query / no tx→contact reverse for *sent* txs** — data exists (`counterparty_id`) but no accessor; `match_in_collection` searches only receival pools. | 🟠 MISSING | payments |
| 10 | **Key selection narrower than canonical** — no AUTHENTICATION fallback on send (kotlin has one); DECRYPTION-first vs kotlin's ENCRYPTION-first. | 🟡 WORSE | crypto |
| 11 | multi-account contacts (one-request-per-direction; `accepted_accounts` unpopulated); contactInfo fetch also 100-truncated; send address-reuse if SPV drops our own broadcast. | 🟡 minor | model / sync / payments |

### Where we are OK or AHEAD (don't "fix")
- **encryptedPublicKey**: 69-byte compact `fp‖cc‖pk` → `IV(16)‖AES-256-CBC/PKCS7` = 96 B — **byte-identical** to dashj `serializeContactPub`.
- **Friendship path geometry** (`m/9'/coin'/15'/0'/A/B/index`), sender/recipient swap (receive vs send), and **DIP-14 256-bit non-hardened CKD** (32-byte BE feed) — **match dashj byte-for-byte** for account 0.
- **ECDH** `SHA256((y&1|2)‖x)` — matches (one byte-level assumption on dashj's agreement; recommend a known-answer test to lock).
- **Send-address advancement** (`currentAddress` semantics via the SPV pipeline) and **incoming detection** — equivalent; our `reconcile_incoming_payments` recovery path is **more robust** than dashj.
- **AHEAD:** we implement **contactInfo** (kotlin/dashj have *no* ContactInfo class); our **rotation idempotency** (`newest_received_per_sender`) beats their exact-`(sender,toUserId,ref)` dedup; **account/keychain self-heal** (`collect_account_build_candidates`) ≈ their `checkDatabaseIntegrity`; our sync **re-entrancy/shutdown** discipline is stricter.

---

## Area 1 — contact-request creation + crypto

- **[INTEROP-BREAK] accountReference ASK28** (`dip14.rs:221`): ours `u32_be(ASK[28..32])>>4`; dashj `BlockchainIdentity.getAccountReference` = `wrapReversed(ASK).toBigInteger().toInt() ushr 4` = `u32_le(ASK[0..4])>>4`. The two **reference clients (iOS vs Android) genuinely disagree** on this field; we chose iOS, dashj is Android. The on-chain census should pick the canonical one (likely dashj/Android). The `version` nibble (`>>28`) interoperates; only the low-28 masked bits diverge → breaks cross-impl rotation/unmask, not basic payment (recipient disregards the ref).
- **[MISSING] encryptedAccountLabel** (`contact_request.rs:319-334`): kotlin `padAccountLabel()` pads to ≥16 chars w/ spaces and *always* emits (even empty → 16 spaces → 48 B). We make it optional + unpadded; a label <16 chars trips our own `≥48` check and errors. Fix: pad to ≥16, trim on decrypt, always emit.
- **[WORSE] key selection** (`select_recipient_key_index`, `contact_requests.rs:395`): kotlin = ENCRYPTION-first, **AUTH/HIGH fallback**; ours = DECRYPTION-first, ENCRYPTION fallback, **no AUTH fallback** → we cannot send to an identity that only has an AUTH ECDSA key, which kotlin can. (Partly a deliberate key-separation choice — product decision.)
- **[DIFFERENT-OK]** entropy/doc-id (both `generate_document_id_v0`; our old consensus bug fixed), encryptedPublicKey (byte-identical), ECDH (recommend dashj KAT), IV/AES-CBC.

## Area 2 — sync / fetch

- **[BUG] pagination** (`contact_request_queries.rs:65,117`): single `limit:100, start:None`, no loop. kotlin `Documents.getAll` loops `startAt = last.id` while `size >= 100`, `retrieveAll`⇒`limit(-1)`. Ordered `$createdAt ASC` ⇒ **newest** dropped past 100, permanently.
- **[MISSING] high-water + 10-min overlap** (`PlatformSyncService.kt:346-372`, `DashPayContactRequestDao.kt:50-54`): kotlin `MAX(timestamp)` per direction, rewinds 10 min for skew; we have neither.
- **[BUG] contact-profile sync absent**: `sync_profiles` iterates `all_identities()` (own + out-of-wallet only, `accessors.rs:54`), never contacts; kotlin `updateContactProfiles` batch-fetches all contacts' profiles (`Profiles.getList`, chunks of 100, `whereIn $ownerId`). We never get contacts' displayName/avatar.
- **[DIFFERENT-OK / AHEAD]** both directions (we're more rotation-robust), account self-heal (parity+), contactInfo ordering (we're ahead — they have none), cadence/re-entrancy (stricter). Latent: our contactInfo fetch also 100-truncated.

## Area 3 — friendship key derivation / payment addresses

- **[INTEROP-BREAK latent] account' = 0'** (key-wallet `account_type.rs:486,509`; `contacts.rs:474` `let account_index = 0`): the path's account segment is hardcoded `0'` and the `index` field is dropped. dashj `FriendKeyChain.getContactPath` uses `contact.getUserAccount()` (receive) / `getFriendAccountReference()` (send). Disjoint address spaces if a counterparty uses account ≠ 0. **Compounds #1** (even with the index wired in, the ASK28 mismatch unmasks the wrong account). Fix needs an upstream key-wallet/rust-dashcore change.
- **[MATCH]** path geometry, ordering, DIP-14 256-bit CKD, send-chain reconstruction — all byte-identical for account 0. Gap limit 10 (DIFFERENT-OK, local concern).

## Area 4 — payments send/receive + tx↔contact

- **[BUG] Sent status stuck Pending** (`payment.rs:87`, `payments.rs:68,153`): `new_sent`=Pending; live recorder + reconcile + send all skip existing txids (`contains_key`), and **no production path** flips Pending→Confirmed (only a test does). UI shows all sends Pending forever. dashj derives status live from `TransactionConfidence`.
- **[MISSING] tx→contact reverse for sends** (`match_in_collection`, `contacts.rs:357`): searches only `dashpay_receival_accounts`, never `dashpay_external_accounts`. dashj `getFriendFromTransaction` scans both. (Compensated for our own sends by direct `counterparty_id` recording, but a recovered/other-device send isn't classifiable.)
- **[MISSING] per-contact tx-history query** (`getContactTransactions` equiv): data exists (`PaymentEntry.counterparty_id`) but no `filter by contact` accessor; the FFI getter returns the whole flat list.
- **[DIFFERENT-OK]** send-address advancement (`next_address`/`next_unused` + SPV `mark_address_used` = `currentAddress` semantics), incoming detection (more robust w/ reconcile), idempotency. Minor: no `mark_address_used` at broadcast ⇒ address-reuse if SPV drops our own tx.

## Area 5 — profile / contactInfo / data-model

- **[BUG] `update_profile` is destructive — it doesn't merge** (`profile.rs:412-428`): we build a **fresh** property map from only the `Some(...)` input fields, so any field the caller leaves `None` is **dropped from the new revision and deleted on-platform**. So editing just the display name **wipes** publicMessage + avatarUrl. kotlin's live path `Profiles.replace` does read-modify-write (`profileData.putAll(currentProfile.toObject())` then overlay). We already fetch the existing doc for id+revision (`profile.rs:354-392`) — seed the map from its `properties()` first. (avatarHash/fingerprint are the one exception we preserve; displayName/publicMessage/avatarUrl are not.) **User-facing data loss; quick fix.**
- **[AHEAD] contactInfo** — confirmed: `kotlin-platform` has **no `ContactInfo` class** (only `Contact`/`ContactRequest`/`ContactRequests`); `dash-wallet` has a TODO referencing `ContactInfo.accountRef` but never built it. We're the de-facto reference. Caveat: our wire format is unverified against any other client (none exists), and `displayHidden`-as-reject-signal is ours-only.
- **[DIFFERENT-OK] relationship model — derive vs materialize:** kotlin has **no persisted "established" and no "rejected" at all** — friendship is a read-time join over the flat `dashpay_contact_request` table (`requestSent && requestReceived ⇒ FRIENDS`). We materialize `established_contacts` + incoming/sent/**rejected** maps and collapse reciprocals into one `EstablishedContact`. Both valid; ours carries richer per-contact state.
- **[MISSING] multi-account contacts** (`contact_requests.rs:323-393`): kotlin keeps **every** `(userId,toUserId,accountReference)` row (a contact on multiple accounts ⇒ multiple rows); we keep **one request per direction** and a rotation *replaces* the prior (`accepted_accounts` exists but is never populated). Fine for the single-account common case; a structural gap for simultaneous multi-account (our own comments defer it).
- **[DIFFERENT-OK] avatarHash** = single SHA-256 both sides (matches); **avatarFingerprint** dHash byte/bit layout coincidentally matches BUT pixel pipeline differs (greyscale **average vs luma-weighted**, resize filter, 9×9 vs 9×8) ⇒ fingerprints **won't be byte-identical cross-client** — that's inherent to perceptual hashing (used for Hamming distance, never equality). **Do NOT write a cross-client exact-match test on the fingerprint.**
- **[DIFFERENT-OK]** profile field set identical (displayName/publicMessage/avatarUrl/avatarHash/avatarFingerprint); signing key HIGH-or-CRITICAL (ours) ⊇ HIGH (theirs) — superset, fine.

---

## Recommended fix priority

0. **`update_profile` merge (#6)** — quick, user-facing data-loss fix: seed the new property map from the existing doc's `properties()` before overlaying inputs (read-modify-write, like `Profiles.replace`). Smallest diff, biggest immediate user impact.
1. **`accountReference` ASK28 byte order (#1)** — flip to `u32_le(ASK[0..4])>>4` to match dashj/Android (the deployed-cohort canonical), fix `unmask_account_reference` symmetrically, add a **dashj known-answer test**. Decide iOS-vs-dashj canonical explicitly (they disagree).
2. **Sync correctness (#3/#6)** — the already-drafted `SYNC_CORRECTNESS_SPEC.md` (high-water + 10-min overlap + cursor pagination, both directions).
3. **Contact-profile sync (#5)** — fetch contacts' profiles (the Friends UI has no names/avatars without it); mirror `updateContactProfiles` (batch `whereIn $ownerId`, incremental).
4. **Sent payment Pending→Confirmed (#4)** — confirm-path must update-in-place, not skip-if-present.
5. **encryptedAccountLabel padding (#7)** — pad ≥16 chars, trim on decrypt, always emit.
6. **account' path segment (#2)** — upstream key-wallet change to use the `index` field; pass the real account on registration. Track (cross-repo).
7. **ECDH KAT, per-contact tx query (#8), key-selection AUTH fallback (#9)** — lower priority / product decisions.
