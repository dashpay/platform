# contactInfo `privateData` — DIP-15 varint format (migrate off CBOR)

Status: **IMPLEMENTED** (2026-06-18) — DIP-15 varint codec in `crypto/contact_info.rs`,
byte-vector + compat tests. (The tolerant minor-version decode stays available for a
future additive field, but **ignore state does NOT ride contactInfo** — R1 found
that leaks who you ignored; ignore is local-only, cross-device via a future encrypted
profile field. See the R1 item in the backlog, dashpay/platform#4020.)
Owner: platform-wallet / platform-encryption
Relates to: Spec 2 (Ignore, adds `relationshipState`), `BLOCK_SPEC.md`,
`CONTACTINFO_FORMAT_SPEC.md` Appendix A.

## Format decision: DIP-15 varint, NOT CBOR (2026-06-18)

`contactInfo.privateData` is an **opaque encrypted byteArray** — the registered
contract validates only its **length** (`byteArray:true, minItems:48,
maxItems:2048` in `dashpay.schema.json`); the field description's "…encoded as an
array in cbor" is **advisory documentation, not a structural constraint**. The
plaintext inside the AES-256-CBC ciphertext is therefore a writer/reader
convention we are free to choose — and we choose **DIP-15**, the authoritative
protocol spec, so we interop with DIP-15-compliant clients (the reference
`dash-wallet` / `kotlin-platform` will follow the DIP when it implements
contactInfo). **No contract change is needed** (length-only validation accepts any
48..2048-byte ciphertext). No client decodes contactInfo today, so this is a free
window: we set the de-facto format and it matches the DIP.

> An earlier pass briefly "reconciled" this the other way (keep CBOR, per the
> schema *description* + `CONTACTINFO_FORMAT_SPEC.md` Appendix A). That over-weighted an advisory
> description as binding. Corrected: the contract enforces length only, DIP-15 is
> authoritative — use varint.

This is **Spec 1** of the DashPay-privacy track. (The minor-version forward-compat
seam — §3 — remains for any future additive field. Note: ignore state is **not**
carried here — R1 found a per-sender `contactInfo` leaks who you ignored, so ignore
is local-only with cross-device deferred to a future encrypted `profile` field.)

---

## 1. Problem

Our `contactInfo.privateData` codec (`crypto/contact_info.rs::encode/decode_private_data`)
emits a **CBOR array** `[aliasName, note, displayHidden, padding?]`. **DIP-15
defines a different format** (verified against `github.com/dashpay/dips/dip-0015.md`,
§"Contact Info" / §"Encrypting Private Data"):

- **Serialization:** "the private data should be serialized in the same way as
  done for **Dash message data**" (dip-0015.md:811) — i.e. the Bitcoin/Dash
  protocol binary format (var-int-length-prefixed strings/arrays), **not CBOR**.
- **Fields (v0), in order:**
  | # | Field | Type | Encoding |
  |---|-------|------|----------|
  | 0 | `version` | uInt32 | `major << 16 \| minor` (dip-0015.md:771) |
  | 1 | `aliasName` | String | var-int length + UTF-8 |
  | 2 | `note` | String | var-int length + UTF-8 |
  | 3 | `displayHidden` | uInt8 | 1 byte |
  | 4 | `acceptedAccounts` | array<uInt32> | var-int count + u32s (dip-0015.md:805) |
- **Crypto:** AES-256-CBC with the `rootEncryptionKey/(2^16+1)'/idx'` derived key
  (we already do this — only the *plaintext serialization* changes).

**Our gaps vs DIP-15:** (a) CBOR instead of Dash-message varint; (b) **no
`version` field**; (c) **no `acceptedAccounts`**.

**Why now (the one cheap window):** verified 2026-06 that **no client decodes
`contactInfo.privateData` today** — `android-dashpay` has no `ContactInfo` class
(the schema is bundled as JSON only); `dash-wallet` has only a `// TODO: choose
the contactRequest based on the ContactInfo.accountRef value`. So there is **no
reader to break.** When `dash-wallet` implements its TODO it will follow DIP-15
(varint), not our CBOR — so if we don't align now, the two clients won't interop.
We're the only writer; fix the wire format while it's free.

## 2. Goal

- Replace the CBOR codec with the **DIP-15 Dash-message varint** serialization,
  including the `version` field and `acceptedAccounts`.
- Adopt DIP-15's **major/minor version forward-compat** model.
- **Define** (not yet populate) `reject` / `block` fields as a **minor-version
  extension**, so a later spec can sync reject/block via contactInfo without
  another format change.
- No behavior change to alias/note/hidden; pure wire-format + versioning.

## 3. DIP-15 versioning model (verbatim, because it drives everything)

dip-0015.md:771-776: `version = major << 16 | minor`.
- **Major** change = **incompatible**: a client that doesn't understand the major
  version **discards the whole contactInfo**.
- **Minor** change = "most likely additional fields": an un-updated client
  "should still be able to parse the first fields … and **ignore data past the
  final field known in the version**."

Consequence (this answers the "won't old clients break?" question): **adding
`reject`/`block` is a MINOR bump** → a DIP-15-v0 reader parses `version …
acceptedAccounts` and ignores our trailing fields. **No breakage.** Only a major
bump locks old readers out. So:
- our baseline = **major 0, minor 0** (DIP-15 v0 fields exactly);
- our reject/block extension = **major 0, minor 1** (appended fields);
- decoders MUST be **tolerant**: read the fields the known minor defines, ignore
  trailing bytes; on an unknown **major**, discard.

## 4. The reject/block fields — DEFINED but NOT ADOPTED (R1, resolved 2026-06-18)

> **Resolution.** This field was the proposed cross-device carrier for
> reject/block. It is **not implemented** and is **not part of the shipped DIP-15
> codec** (which carries only `aliasName` / `note` / `displayHidden` /
> `acceptedAccounts`). Per R1, a `contactInfo` about a *non-established* sender
> leaks *who* you ignored (the timing-correlation argument below), so **Ignore is
> local-only** (Spec 2) and cross-device sync is deferred to a future **encrypted
> field on the `profile` document** (contract / governance track) — NOT to
> `contactInfo`. The design below is retained for reference only.

Appended after `acceptedAccounts`, present from **minor 1** (design only — unused):

| # | Field | Type | Meaning |
|---|-------|------|---------|
| 5 | `relationshipState` | uInt8 | 0 = active, 1 = declined, 2 = blocked (extensible) |

Rationale for a single `relationshipState` byte over two bools: declined/blocked
are mutually-exclusive states of one relationship; one enum is smaller, avoids
the "both set" ambiguity, and extends cleanly (e.g. 3 = muted). `displayHidden`
(field 3) stays as-is for backward DIP-15 compat; `relationshipState` is the
richer superset we read first when present.

**Scope boundary (critical):** this spec only **defined** the field + its
encoding. *Whether and how* a `contactInfo` is created to carry it — especially
for a **non-established** declined/blocked sender — was the **privacy question
(R1 from the block review)**, **RESOLVED (2026-06-18): not via `contactInfo` at
all.** Ignore is local-only (Spec 2); cross-device goes through an encrypted
`profile` field later. Kept for context:

> A `contactInfo` *about a non-contact* is a brand-new on-chain document whose
> existence + `$createdAt` can be timing-correlated with the inbound
> `contactRequest` (public `userIdCreatedAt` index) to re-identify *who* you
> blocked — even though `encToUserId` is encrypted, and the ≥2-contacts gate
> (dip-0015.md:697-699) can't cover a non-contact. **Spec 2 resolved this: a
> per-sender `contactInfo` is leaky (above) and even a single owner-scoped list
> on `contactInfo` still signals "an ignore happened", so ignore is kept
> local-only and cross-device is deferred to an encrypted `profile` field whose
> update timing is conflated with ordinary profile edits.** This format spec is agnostic to that
> choice — it just provides the field.

## 5. Padding / 48-byte floor

The contract validates `privateData` at **48–2048 bytes** (dip-0015.md:727).
Our CBOR codec appends a padding element to reach 48; the Dash-message format has
no field for that, and trailing padding would collide with the "ignore data past
the final known field" rule (a future reader could mis-read padding as a higher-
minor field). **Decision needed (Q-pad):**
- (a) Pad the **ciphertext region** only — encode the exact fields, then rely on a
  reserved **`padding` length-prefixed byte field** placed *last in every minor*
  and documented as "ignore"; or
- (b) define the floor purely as an encryption-layer concern (pad plaintext to ≥
  the size that yields 48-byte ciphertext, inside AES-CBC, with the pad length
  recoverable) so the field stream itself carries no padding.
Recommend **(a) an explicit trailing `padding` var-bytes field** that is *always
the final field* and always skipped — it's self-describing (var-int length) so
"ignore trailing" still works, and it's the closest analog to today's behavior.

## 6. Migration / compatibility of *existing* data

contactInfo docs are immutable on-chain. Any docs **we** already wrote (CBOR,
no version field) become unreadable by the new varint decoder.
- DashPay is **pre-release** (not on mainnet); existing CBOR docs are
  testnet/devnet UAT artifacts → **acceptable to abandon** (they'll simply fail
  to decode and be skipped, same as a foreign-root doc today).
- Do **not** build a CBOR↔varint dual-reader unless we find we must preserve
  specific test data. (Open question Q-dual.)
- The **local** SwiftData/SQLite mirror is rebuilt from chain on sync, so no
  local migration is needed beyond the decoder swap.

## 7. Implementation surface

- `packages/rs-platform-wallet/src/wallet/identity/crypto/contact_info.rs`:
  rewrite `encode_private_data` / `decode_private_data` to the Dash-message varint
  format (var-int string/array helpers; `version` first; tolerant decode that
  stops at the known-minor field count and skips trailing). Keep the AES-CBC layer.
- `ContactInfoPrivateData` struct: add `version: u32` (or major/minor accessors),
  `accepted_accounts: Vec<u32>`, and `relationship_state: u8` (minor ≥ 1).
  `displayHidden` stays. (Note: the in-memory struct already flows through
  `set_contact_metadata(ContactInfoPrivateData)` after the recent refactor.)
- No FFI/Swift signature change (privateData is opaque bytes across the boundary);
  only the bytes' internal layout changes.

## 8. Test plan

- **Round-trip:** encode→decode every field incl. empty/None strings, empty and
  non-empty `acceptedAccounts`, `relationshipState` 0/1/2.
- **Forward-compat:** a **minor-0** decoder reading **minor-1** bytes parses
  v0 fields and ignores `relationshipState` (the DIP-15 guarantee) — pin it.
- **Major-incompat:** a decoder reading an unknown **major** discards (returns
  None / skips), not a partial parse.
- **Vector:** if any DIP-15 / reference test vector for privateData exists, match
  it byte-for-byte (none found in dashj/android-dashpay; we may be authoring the
  first — note that).
- **Floor:** encoded output is ≥ 48 bytes after padding (Q-pad), ≤ 2048.

## 9. Open decisions

- **Q-pad** — explicit trailing `padding` field (recommended) vs encryption-layer
  padding.
- **Q-dual** — abandon existing CBOR docs (recommended, pre-release) vs build a
  CBOR/varint dual-reader.
- **Q-state** — single `relationshipState: uInt8` (recommended) vs separate
  `declined`/`blocked` flags.
- **Q-minor-now** — define `relationshipState` (minor 1) in *this* spec/PR, or
  ship the pure DIP-15-v0 alignment first (minor 0) and add the field in Spec 2?
  (Leaning: ship v0 alignment here; add the field in Spec 2 where it's used —
  keeps this PR a clean wire-format fix.)

---

<!-- Folded in verbatim from docs/dashpay/research/07-contactinfo-conventions.md
     when the transient research/ directory was trimmed (headings demoted one
     level; self-reference adjusted). -->

## Appendix A — contactInfo wire conventions (research, 2026-06-12)

Source-verified findings for implementing the DashPay `contactInfo`
document (M3 task 13). Full citations at the bottom.

### Decisive finding: no reference client implements contactInfo

DashSync-iOS (`DSBlockchainIdentity.m` + Identity models), dashj /
android-dpp / kotlin-platform, and dash-shared-core contain **zero**
contactInfo creation or encryption code. DIP-15 + the deployed
dashpay-contract schema are the only authoritative sources, and **this
repo's implementation sets the de-facto wire convention.** There is no
cross-client byte-compatibility constraint — only self-consistency and
schema validity.

### Conventions adopted (CONFIRMED unless marked INFERRED)

#### Key derivation (DIP-15)

The "root encryption key" is the identity's **registered ENCRYPTION
key** (DIP-11 purpose 1); `rootEncryptionKeyIndex` is that key's id on
the identity. Two child keys are derived from its extended form in the
owner's HD tree (hardened CKDpriv):

```text
encToUserId key:  rootEncryptionKey / 65536' / derivationEncryptionKeyIndex'   (2^16)
privateData key:  rootEncryptionKey / 65537' / derivationEncryptionKeyIndex'   (2^16 + 1)
```

The 2^16 offset is DIP-15's explicit "discount other potential
derivations" choice. The AES-256 key is the raw 32-byte child private
key scalar (INFERRED — no hash step is specified anywhere; matches how
contactRequest ECDH consumes key material).

`derivationEncryptionKeyIndex` is sequential per `$ownerId` starting at
0 (one per contactInfo document; the unique index is
`($ownerId, rootEncryptionKeyIndex, derivationEncryptionKeyIndex)`).

#### encToUserId (DIP-15, verbatim justification in the DIP)

`AES-256-ECB(toUserId)` — exactly 32 bytes = two blocks, **no IV, no
padding**. ECB is sound here because the plaintext is itself a SHA-256
output and the key is never reused for other purposes.

#### privateData

> **CORRECTION (2026-06-18): use DIP-15 varint, not CBOR.** The conclusion
> below ("the deployed schema description wins → CBOR") over-weighted an
> advisory note. The contract validates `privateData` by **length only**
> (`byteArray`, 48–2048); its "array in cbor" text is documentation, NOT an
> enforced structural constraint. The encrypted plaintext format is a free
> writer/reader convention, so we follow **DIP-15** (the authoritative protocol
> spec) with `version`/varstr/`acceptedAccounts`. See the spec above.

`IV(16) ‖ AES-256-CBC(plaintext)` — IV prepended (INFERRED from the
`encryptedPublicKey` convention; DIP-15 doesn't state placement for
this field).

Plaintext (~~CBOR~~ → **DIP-15 varint**, per the correction above): the original
analysis adopted a **CBOR array `[aliasName, note, displayHidden]`** per the
deployed schema's field description — positional, with CBOR `null` for absent
strings (INFERRED). DIP-15 prose instead describes Bitcoin-varint "Dash message
data" with `version` + `acceptedAccounts` — and that is what we now use (the
schema enforces length only, so there's no conflict and no contract change).

#### Privacy rule (DIP-15, spec-only — no client enforces it today)

> "A client should not transmit a contact info document for a user to
> the network until that user has at least two established contacts."

Enforced at the publish gate: with <2 established contacts the local
state still updates; the document write is deferred until the rule is
satisfied.

### Discrepancy table (DIP-15 prose vs deployed schema)

| Question | DIP-15 prose | Deployed schema |
|---|---|---|
| Plaintext format | Bitcoin varint stream | CBOR array |
| Fields | version, aliasName, note, displayHidden, acceptedAccounts | aliasName, note, displayHidden |
| version | uInt32 present | absent |
| acceptedAccounts | array of uInt32 | absent |

### Sources

- DIP-0015 (dashpay/dips) — derivation offsets, ECB/CBC modes, privacy rule
- dashpay-contract `schema/v1/dashpay.schema.json` — CBOR-array description,
  unique index, 48–2048B bounds
- DIP-0011 (key purposes), DIP-0013 (identity key paths), DIP-0009
  assignments (15'/16' are incoming-funds / auto-accept — no contactInfo path)
- dashsync-iOS Identity models, android-dpp, kotlin-platform,
  dash-shared-core — checked: no contactInfo implementation anywhere
- rs-dpp `lib.rs` `RootEncryptionKeyIndex` / `DerivationEncryptionKeyIndex`
  type aliases
