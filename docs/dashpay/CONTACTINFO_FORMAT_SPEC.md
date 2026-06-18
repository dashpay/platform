# contactInfo `privateData` — format reconciliation (CBOR vs DIP-15 varint)

Status: **SCRAPPED — the migration premise was wrong. Keep CBOR.** (2026-06-18)
Owner: platform-wallet / platform-encryption

> ## Resolution (the review step caught this before any code)
>
> The premise of this spec — migrate `contactInfo.privateData` from CBOR to the
> DIP-15 Dash-message **varint** format and add `version` + `acceptedAccounts` —
> is **wrong**. The **deployed, registered dashpay contract** is the authority a
> client validates against, and it mandates CBOR:
>
> ```json
> // packages/dashpay-contract/schema/v1/dashpay.schema.json → contactInfo.privateData
> { "type":"array","byteArray":true,"minItems":48,"maxItems":2048,
>   "description":
>     "This is the encrypted values of aliasName + note + displayHidden encoded as an array in cbor" }
> ```
>
> So the schema says **CBOR `[aliasName, note, displayHidden]`** — and explicitly
> NOT `version`/`acceptedAccounts`. DIP-15's *prose* describes varint + those two
> fields, but that does not match the registered contract; any schema-reading
> client codes against the contract, not the DIP prose. Migrating to varint would
> (a) diverge from what every client expects and (b) require a coordinated
> **contract update** (Contract track — DIP/governance, deferred), for no benefit.
>
> **Decision: the current CBOR codec (`crypto/contact_info.rs`) is correct — keep
> it.** This matches `research/07 §C` ("the deployed schema description wins").
> `research/01`'s framing ("CBOR per DIP-0015") reached the right answer (CBOR)
> for the wrong reason (it's CBOR per the *schema*, the DIP prose says varint).
>
> **Consequence for the Ignore feature (Spec 2):** any cross-device ignore signal
> rides the existing CBOR array, NOT a format change — either reuse `displayHidden`
> (already field #3, already the hide/suppress flag) or append a 4th CBOR element
> in place of the current padding (decoders read the first three and ignore the
> rest — the documented forward-compat seam). No wire-format migration is needed.
>
> The original (rejected) migration analysis is preserved below for history.

---

## ~~Original draft (rejected — varint migration)~~

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

## 4. The reject/block fields (define here, populate later)

Appended after `acceptedAccounts`, present from **minor 1**:

| # | Field | Type | Meaning |
|---|-------|------|---------|
| 5 | `relationshipState` | uInt8 | 0 = active, 1 = declined, 2 = blocked (extensible) |

Rationale for a single `relationshipState` byte over two bools: declined/blocked
are mutually-exclusive states of one relationship; one enum is smaller, avoids
the "both set" ambiguity, and extends cleanly (e.g. 3 = muted). `displayHidden`
(field 3) stays as-is for backward DIP-15 compat; `relationshipState` is the
richer superset we read first when present.

**Scope boundary (critical):** this spec only **defines** the field + its
encoding. *Whether and how* a `contactInfo` is created to carry it — especially
for a **non-established** declined/blocked sender — is the **privacy question
(R1 from the block review)** deferred to Spec 2/3:

> A `contactInfo` *about a non-contact* is a brand-new on-chain document whose
> existence + `$createdAt` can be timing-correlated with the inbound
> `contactRequest` (public `userIdCreatedAt` index) to re-identify *who* you
> blocked — even though `encToUserId` is encrypted, and the ≥2-contacts gate
> (dip-0015.md:697-699) can't cover a non-contact. **Spec 2 must resolve whether
> non-established reject/block is carried per-sender (leaky) or in a single
> owner-scoped self-encrypted list (bounded leak), or only for established
> contacts (no leak, partial coverage).** This format spec is agnostic to that
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
