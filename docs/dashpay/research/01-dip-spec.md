# DashPay Protocol Specification (from DIPs)

> Protocol-level reference for verifying a Rust + Swift DashPay implementation against the
> official Dash Improvement Proposals. Focuses on the contact / friendship / payment / profile
> flows. Every derivation path, encryption detail, and document field below is sourced from the
> DIPs (and cross-checked against the deployed `dashpay-contract` JSON schema) with citations.

## Source DIPs read

| DIP | Title | URL |
|-----|-------|-----|
| DIP-0009 | Feature Derivation Paths | https://raw.githubusercontent.com/dashpay/dips/master/dip-0009.md |
| DIP-0011 | Identities | https://raw.githubusercontent.com/dashpay/dips/master/dip-0011.md |
| DIP-0013 | Identities in Hierarchical Deterministic Wallets | https://raw.githubusercontent.com/dashpay/dips/master/dip-0013.md |
| DIP-0014 | Extended Key Derivation using 256-Bit Unsigned Integers | https://raw.githubusercontent.com/dashpay/dips/master/dip-0014.md |
| DIP-0015 | **DashPay** (core) | https://raw.githubusercontent.com/dashpay/dips/master/dip-0015.md |
| DIP-0017 | Dash Platform Payment Addresses and HD Derivation | https://raw.githubusercontent.com/dashpay/dips/master/dip-0017.md |

Cross-check source (deployed contract, not a DIP):
- DashPay contract schema: https://raw.githubusercontent.com/dashpay/platform/master/packages/dashpay-contract/schema/v1/dashpay.schema.json

All DIPs were fetched from the `master` branch successfully (no fallback to `main` needed). DIP-0017
turned out to be about *standalone* platform payment addresses (`m/9'/5'/17'/...`) and is **not** the
source of the DashPay friendship derivation — that lives in DIP-0015 + DIP-0014. It is included here
only to disambiguate, so an implementer does not confuse the `17'` feature with the `15'` feature.

> **Verification note.** Where the DIP text and the deployed v1 contract schema disagree on a
> concrete number (e.g. `publicMessage` max length, the `$createdAtCoreBlockHeight` field name), the
> **deployed schema is authoritative for an implementation** and the DIP value is noted as the
> original spec intent. Disagreements are flagged inline with ⚠.

---

## 1. What DashPay is (user model)

DIP-0015 defines DashPay as:

> "an application built on Dash Platform that creates bidirectional direct settlement payment
> channels between Dash Identities."

The product goals (DIP-0015):
- "Payments are easy to perform"
- "A history of payments is readily available"
- "Third parties aren't knowledgeable about the details of the payment"

The design puts "Contacts front and center. All users have a contact list and can easily pay their
friends." Because a payment channel is always tied to two known identities, "the recipient knows and
will always know the sender, hence contact history will always show who made a payment to the user."

Concretely, the user-facing model is:
- **Username** → handled by **DPNS** (DIP-0012), not DashPay. A username is a DPNS name that resolves
  to an **identity** (DIP-0011). DashPay never stores usernames; it references identities by their
  32-byte unique ID.
- **Identity** (DIP-0011) → the cryptographic actor. Holds keys + a credit balance, signs all state
  transitions.
- **Profile** → public presentation (display name, avatar, public message). One `profile` document
  per identity.
- **Contact / Friend** → another identity you have exchanged `contactRequest` documents with.
- **Sending money to a contact** → derive the next unused receive address from the contact's shared
  extended public key (decrypted from their `contactRequest` to you) and pay it on L1 (Dash Core
  chain). DashPay itself is the *coordination/keyshare* layer; the actual value transfer is an
  ordinary Dash L1 transaction to a DashPay-derived address.

The relationship between two mutually-connected identities is called, in the spec, a **"Direct
Settlement Payment Channel (DSPC)"** and the two identities are "friends."

---

## 2. Contact request / friend request lifecycle

### 2.1 The `contactRequest` document

A contact request is a **`contactRequest` document** owned (`$ownerId`) by the **sender** and pointing
(`toUserId`) at the **recipient**. Creating it:

1. `$ownerId` = sender's identity unique ID (set by the platform from the signing identity).
2. `toUserId` = recipient's identity unique ID.
3. Sender derives the **incoming-funds extended public key** for this specific friendship
   (Section 4) — i.e. the xpub that generates the addresses **the sender will watch to receive money
   *from* the recipient**.
4. Sender derives an **ECDH shared secret** with the recipient (Section 3) and **AES-256-CBC encrypts**
   that extended public key into `encryptedPublicKey`.
5. Sender computes `accountReference` (Section 7) and optionally `encryptedAccountLabel` /
   `autoAcceptProof`.
6. Sender publishes it as a signed document-create state transition. The signing identity key must be
   an **encryption/decryption-capable key at the bounded "High" security level** (DIP-0011 places
   contact-request creation at the High security level; the v1 contract requires an encryption key of
   key-type bound level 2).

### 2.2 The bidirectional handshake (what "friendship" means)

A single `contactRequest` is **one-directional**. Friendship is the *pair*:

> "When two users have both sent contact requests to each other, then each is considered a fully
> established contact with the other." — DIP-0015

So:
- **A → B**: A sends `contactRequest{ $ownerId: A, toUserId: B }`. This is a *pending* / *incoming*
  request from B's perspective. A is now watching for payments from B, but B does not yet know how to
  pay A back (B has A's xpub-to-receive-from-B, so B *can* pay A; symmetric below).
- **B → A**: B "accepts" by sending `contactRequest{ $ownerId: B, toUserId: A }`. Now both directions
  exist.
- **Established friendship / DSPC** = both `contactRequest` documents exist (A→B **and** B→A).

Direction semantics to be precise about (this is the part implementations get wrong):
- A's `contactRequest` to B carries the xpub for the address space **A uses to receive funds *from*
  B**. So **B uses A's request** to know where to pay A.
- Symmetrically, B's `contactRequest` to A carries the xpub for the space **B receives from A**, and
  **A uses B's request** to pay B.
- Therefore to *pay someone* you read **their** outgoing contactRequest addressed **to you**
  (`toUserId == yourId`, `$ownerId == theirId`), decrypt its `encryptedPublicKey`, and derive
  addresses. To *receive/track*, you watch the xpub you yourself encrypted.

### 2.3 Immutability

Contact requests are immutable and non-deletable:

> "they can never be deleted. This means they can never be updated or removed from the platform tree."
> — DIP-0015

Rationale: a user must not be able to retroactively alter the extended public key (which would orphan
prior payments / rewrite payment history). To rotate keys, a **new** `contactRequest` is sent with a
new `accountReference` version (Section 7) rather than mutating the old one. The v1 contract enforces
this with `"documentsMutable": false` semantics / no update transition.

### 2.4 Auto-accept (optional)

`autoAcceptProof` (optional, 38–102 bytes) lets a recipient pre-authorize automatic acceptance.
DIP-0015 defines a **separate** derivation path for the auto-accept proof keys:

```text
m / 9' / 5' / 16' / timestamp'
```

i.e. feature `16'` (one above DashPay's `15'`), with an expiration `timestamp'` as the hardened leaf.
This is distinct from the payment-address path and is only used to prove auto-accept eligibility.

---

## 3. Encrypted extended public key sharing (ECDH + AES)

### 3.1 What is encrypted

The plaintext is a **DashPay incoming-funds extended public key** — the xpub at the account level of
the friendship path (Section 4), i.e. the node `m/9'/5'/15'/0'/<sender256>/<recipient256>` whose
non-hardened `index` children are the actual receive addresses. The spec encrypts a **compacted**
serialization of the extended key (NOT the full 78-byte BIP32 xpub):

> "The binary format used is as follows: Parent fingerprint (4 bytes), Chain code (32 bytes),
> Public Key (33 bytes)" — DIP-0015

So the encrypted plaintext is **69 bytes**: `parentFingerprint(4) || chainCode(32) || compressedPubKey(33)`.
(Version/depth/child-number are omitted because both sides already know the path.)

### 3.2 The ECDH shared secret

DIP-0015 uses **libsecp256k1's non-standard ECDH** (NOT plain X-coordinate ECDH):

> "libsecp256k1_ecdh has one extra step to derive the shared key, which is to calculate
> `SHA256((y[31]&0x1|0x2) || x)` where `|` is bitwise or and `||` is concatenation."

Algorithm:
1. Compute the EC point `P = d_self * Q_other` (shared point), where `d_self` is one party's identity
   private key and `Q_other` is the other party's identity public key.
2. Let `x` be the 32-byte big-endian X coordinate of `P`, and `y` its Y coordinate.
3. Prefix byte = `(y[31] & 0x1) | 0x2` — i.e. the standard compressed-point parity prefix (`0x02` if
   Y is even, `0x03` if odd).
4. `sharedKey = SHA256( prefixByte || x )` → 32 bytes → used as the **AES-256 key**.

Both parties compute the identical shared key, per DIP-0015:

> "The private key at `senderKeyIndex` of the sender and the public key at `recipientKeyIndex` of the
> recipient" and conversely "The private key at the `recipientKeyIndex` of the recipient and the
> public key at `senderKeyIndex` of the sender" both derive the same ECDH shared key.

### 3.3 Which identity keys: `senderKeyIndex` / `recipientKeyIndex`

These are **identity public-key `id`s** (DIP-0011: each identity public key has an integer `id`
"unique for the Identity public keys"). They select *which* of the sender's / recipient's identity
keys participate in the ECDH:
- `senderKeyIndex` → the key `id` in the **sender's** identity `publicKeys` array whose private key
  the sender uses (and whose public key the recipient uses).
- `recipientKeyIndex` → the key `id` in the **recipient's** identity `publicKeys` array whose public
  key the sender uses (and whose private key the recipient uses to decrypt).

Both are stored in the `contactRequest` so either party can reconstruct the exact key pair used. They
must reference encryption/decryption-purpose keys (DIP-0011 purposes 1/2/3).

### 3.4 AES-256-CBC layout of `encryptedPublicKey` (96 bytes total)

> "Initialization Vector (16 bytes), Encrypted extended public key with padding (80 bytes) that is
> encrypted by CBC-AES-256." — DIP-0015

```
encryptedPublicKey (96 bytes):
  [ 0 .. 16 )   IV                        (16 bytes, random, unique per request)
  [ 16 .. 96 )  AES-256-CBC ciphertext    (80 bytes)
                  = CBC-AES256(key = sharedKey, iv,
                       plaintext = parentFingerprint(4) || chainCode(32) || pubKey(33) = 69 bytes)
                  69 bytes plaintext → PKCS#7 pad to 80 (next 16-byte multiple) → 80 ciphertext bytes
```

The deployed schema fixes `encryptedPublicKey` at exactly **96** `byteArray` items (16 IV + 80 cipher),
confirming the DIP layout.

### 3.5 `encryptedAccountLabel` (optional, 48–80 bytes)

Same scheme as `encryptedPublicKey` but encrypts a human-readable account label:

> "Initialization Vector (16 bytes), Encrypted account label with padding (32–64 bytes) that is
> encrypted by CBC-AES-256." — DIP-0015

So total = 16 IV + (32..64) ciphertext = **48..80 bytes**, matching the schema's `minItems: 48,
maxItems: 80`.

### 3.6 `contactInfo` private-data encryption (different scheme)

`contactInfo` uses **BIP32-derived symmetric keys**, not ECDH:
- `rootEncryptionKeyIndex` + `derivationEncryptionKeyIndex` select a BIP32 `CKDpriv` child of the
  owner's own key tree (so only the owner can decrypt — this is *self*-encrypted private metadata).
- `encToUserId` (32 bytes): the contact's identity ID, encrypted (AES-256, ECB per the field's fixed
  block size) so the network can't trivially link `contactInfo` to a specific contact.
- `privateData` (48–2048 bytes): AES-256-CBC-encrypted CBOR blob.

Privacy rule (DIP-0015):

> "A client should not transmit a contact info document for a user to the network until that user has
> at least two established contacts."

(prevents trivially correlating a `contactInfo` with the single contactRequest it must refer to.)

---

## 4. Payment-address derivation (DIP-0015 friendship path + DIP-0014 256-bit indices)

### 4.1 The friendship derivation path

DIP-0015 (incoming funds), verbatim:

> "The derivation path therefore has the following paths:
> `m(userA)/9'/5'/15'/0'/(userA's unique id)/(userB's unique id)/index`"

Structured:

```
m / 9' / 5' / 15' / 0' / <userA_256> / <userB_256> / index
  │    │    │     │    │       │              │          └─ non-hardened, 32-bit  : address index (0,1,2,…)
  │    │    │     │    │       │              └──────────── non-hardened, 256-bit : OTHER party's identity id (DIP-14)
  │    │    │     │    │       └─────────────────────────── non-hardened, 256-bit : THIS party's  identity id (DIP-14)
  │    │    │     │    └─────────────────────────────────── hardened : account (0')
  │    │    │     └──────────────────────────────────────── hardened : feature 15' = DashPay incoming funds (DIP-9)
  │    │    └────────────────────────────────────────────── hardened : coin_type 5' = Dash (mainnet; 1' testnet)
  │    └─────────────────────────────────────────────────── hardened : purpose 9'  (DIP-9 feature paths)
  └──────────────────────────────────────────────────────── master from seed
```

Hardening summary (load-bearing — verify exactly):
- `9'`, `5'`, `15'`, `0'` → **hardened**.
- `<userA_256>`, `<userB_256>`, `index` → **non-hardened** (this is the whole point).

DIP-0015 on *why* the last three are non-hardened:

> "Making the last three fields non-hardened allows for an extended public key that covers all address
> spaces of all our contacts for all our identities."

i.e. a wallet can hold the xpub at `m/9'/5'/15'/0'` and, with only public derivation, enumerate the
receive space for every contact of every identity — enabling watch-only accounting and contact-request
construction without touching private keys.

### 4.2 Both parties derive the same chain

For friendship between A and B, the **incoming-funds path A publishes to B** is rooted at
`<userA_256>/<userB_256>` (A's id first = owner, then the counterparty). A keeps the private side; B
receives A's encrypted xpub (Section 3) and derives the **same** public chain
`…/<userA_256>/<userB_256>/index` to compute the addresses to **pay A**. Conversely B's request to A
is rooted `<userB_256>/<userA_256>`. The ordering (owner-first) is what disambiguates the two
directions of the channel.

### 4.3 DIP-0014: 256-bit derivation indices

Standard BIP32 indices are 32-bit (31 bits + hardening bit). A Dash identity ID is **256 bits**, so
DIP-0014 extends CKD to take full 256-bit indices. Key points:

- Hardening is moved out of the index into a **separate boolean `h`** (because the MSB is now a real
  value bit, not a hardening flag). DIP-0014 allows "2^256 normal child keys and 2^256 hardened child
  keys."
- Modified child key derivation (`CKDpriv256`):

  ```
  index < 2^32 (compatibility / BIP32-identical):
    hardened     : I = HMAC-SHA512(key=c_par, data = 0x00 || ser256(k_par) || ser32(i))
    non-hardened : I = HMAC-SHA512(key=c_par, data = serP(point(k_par)) || ser32(i))

  index ≥ 2^32 (256-bit mode):
    hardened     : I = HMAC-SHA512(key=c_par, data = 0x00 || ser256(k_par) || ser256(i))
    non-hardened : I = HMAC-SHA512(key=c_par, data = serP(point(k_par)) || ser256(i))

  then: I_L = I[0..32], I_R = I[32..64]
        k_i = parse256(I_L) + k_par  (mod n)
        c_i = I_R
  ```

  Public derivation `CKDpub256` (non-hardened only; rejects `h=true`):
  `K_i = point(parse256(I_L)) + K_par`, `c_i = I_R`.

- For DashPay the identity IDs are used as the **256-bit non-hardened** indices. DIP-0014 explicitly
  ties this to the security requirement: "any form that reduces the entropy of this relationship to 31
  bits per child would be attackable." The two 256-bit levels are the user IDs; reducing them to 31
  bits (e.g. by hashing down to a BIP32 index) would be insecure.

- 256-bit extended keys use **new serialization version bytes** (e.g. `0x0EECEFC5` mainnet-public) and
  replace the 4-byte child-number field with `ser256(i)` (32 bytes) plus a separate hardening byte.
  (Relevant if the implementation serializes/transports these xpubs.)

---

## 5. Profile

One `profile` document per identity. Fields (deployed v1 schema; DIP value noted where it differs):

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| `$ownerId` | byteArray(32) | implicit | profile owner identity ID |
| `displayName` | string | 1–25 chars | user-friendly, **not unique** |
| `publicMessage` | string | 1–**140** chars (schema) ⚠ DIP says 0–250 | bio/status |
| `avatarUrl` | string (URI) | 1–2048 chars | public image URL |
| `avatarHash` | byteArray(32) | SHA-256 of avatar image | integrity |
| `avatarFingerprint` | byteArray(8) | dHash perceptual hash | dedup / fuzzy match |
| `$createdAt` | integer | required | ms timestamp |
| `$updatedAt` | integer | required | ms timestamp |

Constraints:
- DIP-0015: "At least one field between the `avatarUrl`, `publicMessage` and `displayName` must be
  set." (The v1 schema additionally makes the three avatar fields **mutually dependent** — if any
  avatar field is present, the related ones must be too.)
- Profiles **are** mutable (unlike contactRequest): updating `displayName` etc. bumps `$updatedAt`.

Indices (v1 schema):
- `profile` unique index on `$ownerId` (one profile per identity).
- `ownerIdAndUpdatedAt`: `$ownerId` asc, `$updatedAt` asc (for sync / "what changed").

---

## 6. DashPay data contract — document types & indices

The DashPay contract defines exactly **three** document types: `profile`, `contactRequest`,
`contactInfo`. Below merges DIP-0015 field semantics with the **deployed v1 schema**
(`packages/dashpay-contract/schema/v1/dashpay.schema.json`), which is authoritative for byte
sizes/indices.

### 6.1 `contactRequest`

| Field | Type | Constraints | Required | Meaning |
|-------|------|-------------|:--------:|---------|
| `toUserId` | byteArray(32) | contentMediaType `application/x.dash.dpp.identifier` | ✔ | recipient identity ID |
| `encryptedPublicKey` | byteArray(96) | exactly 96 | ✔ | IV(16) + AES-CBC xpub(80) — Section 3.4 |
| `senderKeyIndex` | integer | ≥0 | ✔ | sender identity key `id` for ECDH |
| `recipientKeyIndex` | integer | ≥0 | ✔ | recipient identity key `id` for ECDH |
| `accountReference` | integer | ≥0 | ✔ | masked account ref (Section 7) |
| `encryptedAccountLabel` | byteArray(48–80) | optional | | IV(16) + AES-CBC label(32–64) |
| `autoAcceptProof` | byteArray(38–102) | optional | | optional auto-accept proof (path `m/9'/5'/16'/timestamp'`) |
| `$createdAt` | integer | | ✔ | ms timestamp |
| `$createdAtCoreBlockHeight` | integer | | ✔ | Dash L1 chain height at creation (DIP calls it `$coreHeightCreatedAt`) ⚠ |

Indices (v1 schema):
- `ownerIdUserIdAndAccountRef` — **unique** on (`$ownerId`, `toUserId`, `accountReference`). This is
  the key uniqueness invariant: one request per (sender, recipient, accountReference). A *new*
  accountReference version lets the same pair create a fresh request (key rotation).
- `ownerIdUserId` — (`$ownerId`, `toUserId`).
- `userIdCreatedAt` — (`toUserId`, `$createdAt`): **incoming** requests for a user, time-ordered.
- `ownerIdCreatedAt` — (`$ownerId`, `$createdAt`): **outgoing** requests, time-ordered.

Constraints: immutable, non-deletable; creation requires an identity encryption/decryption key at the
bound security level (High).

### 6.2 `profile`

See Section 5.

### 6.3 `contactInfo`

| Field | Type | Constraints | Required | Meaning |
|-------|------|-------------|:--------:|---------|
| `encToUserId` | byteArray(32) | | ✔ | contact's identity ID, AES-encrypted |
| `rootEncryptionKeyIndex` | integer | ≥0 | ✔ | owner BIP32 root key index for self-encryption |
| `derivationEncryptionKeyIndex` | integer | ≥0 | ✔ | owner BIP32 derivation index |
| `privateData` | byteArray(48–2048) | encrypted CBOR | ✔ | self-encrypted contact metadata |
| `$createdAt` | integer | | ✔ | |
| `$updatedAt` | integer | | ✔ | |

`privateData` plaintext (CBOR, per DIP-0015), decrypted only by the owner:
- `version` (uInt32)
- `aliasName` (String) — user-chosen nickname for the contact
- `note` (String) — free-form notes
- `displayHidden` (uInt8) — hidden/ignored flag
- `acceptedAccounts` (array of uInt32) — which account-reference versions have been accepted

Indices (v1 schema):
- `ownerIdAndKeys` — **unique** on (`$ownerId`, `rootEncryptionKeyIndex`, `derivationEncryptionKeyIndex`).
- `ownerIdAndUpdatedAt` — (`$ownerId`, `$updatedAt`).

Privacy publication rule: do not publish a `contactInfo` until the owner has ≥2 established contacts
(Section 3.6).

---

## 7. `accountReference` — masked derivation hash

### 7.1 Purpose

The friendship path uses account `0'` by default, but a user may use higher account numbers. The
`accountReference` integer tells the recipient **which account** the sender's published xpub belongs to,
**without revealing the raw account number** (which would leak wallet structure). It also carries a
**version** so key rotations are detectable.

### 7.2 Exact computation (DIP-0015, verbatim)

```
ASK                  = HMAC-SHA256(senderSecretKey, extendedPublicKey)
ASK28                = 28 most significant bits of ASK
ShortenedAccountBits = Account & 0x0FFFFFFF          # low 28 bits of the account number
VersionBits          = Version << 28                 # top 4 bits = version
AccountRef           = VersionBits | (ASK28 xor ShortenedAccountBits)
```

Where:
- `senderSecretKey` is the sender's secret used as the HMAC key (the private key associated with the
  published extended public key / friendship root).
- `extendedPublicKey` is the (compact, 69-byte) extended public key being shared.
- `ASK28` "can be considered the result of a pseudorandom function derived from the account" — it masks
  the account so an observer cannot read the raw account number.
- Result layout: **bits 31..28 = version (4 bits)**, **bits 27..0 = masked account (28 bits)**.

The recipient, knowing the decrypted `extendedPublicKey` and the sender's relevant public key, can
recompute `ASK28` and **un-mask** the account number, and read the version.

DIP-0015 notes uniqueness is not required: "Using only 28 bits means collision probability of 2^28,
but uniqueness is not a requirement of this system."

### 7.3 Version semantics (key rotation)

> "if receiving any number other than zero, and while also having a contact request with the previous
> version, clients should notify the recipient user … that the sender has updated their payment
> addresses." — DIP-0015

So a non-zero (or incremented) version on a *new* `contactRequest` for an existing pair signals the
sender rotated their payment xpub; the recipient should start using the new addresses. The
`(ownerId, toUserId, accountReference)` unique index is exactly what allows multiple successive
requests for the same pair (one per version).

---

## 8. Cross-cutting derivation reference (for the implementation)

```
# DashPay incoming-funds receive addresses (the money path) — DIP-15 + DIP-14
m / 9' / 5' / 15' / 0' / <ownerIdentity256> / <counterpartyIdentity256> / index
        └ hardened ┘        └─ non-hardened 256-bit (DIP-14) ─┘          └ non-hardened 32-bit

# DashPay auto-accept proof — DIP-15
m / 9' / 5' / 16' / timestamp'

# Identity authentication / registration keys — DIP-13 (NOT DashPay payments, but used as the
# ECDH identity keys referenced by senderKeyIndex/recipientKeyIndex)
m / 9' / 5' / 5' / <subfeature'> / <keyType'> / <identityIndex'> / <keyIndex'>
#   first identity, first ECDSA auth key = m/9'/5'/5'/0'/0'/0'/0'
#   subfeature 1' = registration funding, 2' = top-up, 3' = invitation

# Standalone platform payment addresses — DIP-17 (separate feature; do NOT confuse with 15')
m / 9' / 5' / 17' / account' / key_class' / index     (mainnet; coin_type 1' on testnet)
```

Coin type is `5'` on mainnet, `1'` on testnets (DIP-0009 / SLIP-0044).

---

## 9. Implementation verification checklist (Rust + Swift)

1. **256-bit CKD** (DIP-14): identity IDs are used as full 256-bit **non-hardened** indices. Verify the
   HMAC data is `serP(point(k_par)) || ser256(i)` (non-hardened, 256-bit mode), **not** a truncated
   32-bit index. Reducing to 31 bits is a security bug per DIP-14.
2. **Friendship path ordering**: owner identity first, counterparty second
   (`…/<owner256>/<counterparty256>/index`). The two directions of a channel differ only by this order.
3. **Compact xpub plaintext** = `parentFingerprint(4) || chainCode(32) || pubKey(33)` = 69 bytes — NOT
   a 78-byte BIP32 xpub. Pad to 80 with PKCS#7 before AES.
4. **ECDH** is libsecp256k1-style: `SHA256( ((y&1)|2) || x )`, not raw X-coord, not standard
   SHA-512-based ECDH.
5. **AES**: `encryptedPublicKey` = IV(16) ‖ CBC-AES-256(80) = 96 bytes fixed. Same scheme for
   `encryptedAccountLabel` (48–80). `contactInfo.privateData` uses BIP32-derived keys, `encToUserId`
   uses ECB.
6. **accountReference**: top 4 bits = version, low 28 = `HMAC-SHA256(secret, xpub)[28 msb] XOR
   (account & 0x0FFFFFFF)`.
7. **Friendship = both contactRequests exist.** Sending one is "pending"; the reverse one is "accept."
8. **Immutability**: never update/delete a `contactRequest`; rotate via a new one with bumped version.
9. **Indices** must match the unique `(ownerId, toUserId, accountReference)` invariant and the
   incoming/outgoing `(toUserId,$createdAt)` / `($ownerId,$createdAt)` query indices.
10. **Field-name/size discrepancies** between DIP-0015 prose and the deployed v1 schema (⚠ above):
    use the schema — `publicMessage` 1–140 (not 0–250), core-height field is `$createdAtCoreBlockHeight`.
    Confirm against the exact contract version your network runs.
