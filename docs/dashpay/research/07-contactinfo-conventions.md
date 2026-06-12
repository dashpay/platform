# contactInfo wire conventions (research, 2026-06-12)

Source-verified findings for implementing the DashPay `contactInfo`
document (M3 task 13). Full citations at the bottom.

## Decisive finding: no reference client implements contactInfo

DashSync-iOS (`DSBlockchainIdentity.m` + Identity models), dashj /
android-dpp / kotlin-platform, and dash-shared-core contain **zero**
contactInfo creation or encryption code. DIP-15 + the deployed
dashpay-contract schema are the only authoritative sources, and **this
repo's implementation sets the de-facto wire convention.** There is no
cross-client byte-compatibility constraint — only self-consistency and
schema validity.

## Conventions adopted (CONFIRMED unless marked INFERRED)

### Key derivation (DIP-15)

The "root encryption key" is the identity's **registered ENCRYPTION
key** (DIP-11 purpose 1); `rootEncryptionKeyIndex` is that key's id on
the identity. Two child keys are derived from its extended form in the
owner's HD tree (hardened CKDpriv):

```
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

### encToUserId (DIP-15, verbatim justification in the DIP)

`AES-256-ECB(toUserId)` — exactly 32 bytes = two blocks, **no IV, no
padding**. ECB is sound here because the plaintext is itself a SHA-256
output and the key is never reused for other purposes.

### privateData

`IV(16) ‖ AES-256-CBC(plaintext)` — IV prepended (INFERRED from the
`encryptedPublicKey` convention; DIP-15 doesn't state placement for
this field).

Plaintext: **CBOR array `[aliasName, note, displayHidden]`** per the
deployed schema's field description — positional, with CBOR `null`
for absent strings (INFERRED). NOTE: DIP-15 prose instead describes
Bitcoin-varint "Dash message data" with extra `version` +
`acceptedAccounts` fields; the deployed schema description wins (it is
what any schema-reading client will expect). `version` /
`acceptedAccounts` are NOT included — re-introducing them later means
a versioned-CBOR convention change.

### Privacy rule (DIP-15, spec-only — no client enforces it today)

> "A client should not transmit a contact info document for a user to
> the network until that user has at least two established contacts."

Enforced at the publish gate: with <2 established contacts the local
state still updates; the document write is deferred until the rule is
satisfied.

## Discrepancy table (DIP-15 prose vs deployed schema)

| Question | DIP-15 prose | Deployed schema |
|---|---|---|
| Plaintext format | Bitcoin varint stream | CBOR array |
| Fields | version, aliasName, note, displayHidden, acceptedAccounts | aliasName, note, displayHidden |
| version | uInt32 present | absent |
| acceptedAccounts | array of uInt32 | absent |

## Sources

- DIP-0015 (dashpay/dips) — derivation offsets, ECB/CBC modes, privacy rule
- dashpay-contract `schema/v1/dashpay.schema.json` — CBOR-array description,
  unique index, 48–2048B bounds
- DIP-0011 (key purposes), DIP-0013 (identity key paths), DIP-0009
  assignments (15'/16' are incoming-funds / auto-accept — no contactInfo path)
- dashsync-iOS Identity models, android-dpp, kotlin-platform,
  dash-shared-core — checked: no contactInfo implementation anywhere
- rs-dpp `lib.rs` `RootEncryptionKeyIndex` / `DerivationEncryptionKeyIndex`
  type aliases
