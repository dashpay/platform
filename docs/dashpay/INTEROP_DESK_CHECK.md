# DashPay cross-client interop desk-check

> Promoted from `docs/dashpay/research/06-interop-desk-check.md` when the
> transient `research/` directory was trimmed — older citations of
> "`research/06`" refer to this file. Kept in the shipped docs because it is
> the evidence base for the consensus-facing wire-format decisions
> (69-byte compact xpub, key-purpose envelope, ASK28 byte order) cited by
> `SPEC.md` and `DIP_CONFORMANCE_GAPS.md`.

Research date: 2026-06-10 (Milestone 1, task 5 — verify-only).
Question: do THIS stack's DashPay wire formats match the reference clients (iOS DashSync,
Android dashj/android-dashpay), per DIP-15? If not, contacts established by our wallet
cannot pay / be paid by mobile-app users.

Reference sources (all read on this date):

| Source | Ref |
|---|---|
| DIP-15 spec | https://raw.githubusercontent.com/dashpay/dips/master/dip-0015.md |
| iOS DashSync (master) | https://github.com/dashpay/dashsync-iOS |
| iOS crypto core (master) | https://github.com/dashpay/dash-shared-core (`dash-spv-masternode-processor`) |
| Android DashPay lib (master) | https://github.com/dashpay/android-dashpay |
| Android dashj (master) | https://github.com/dashpay/dashj |
| key-wallet (our BIP32) | rust-dashcore @ `3d0d5dcd4ad64e2199a726651bca7f8ffac123e6`, `key-wallet/src/bip32.rs` |

## Verdict summary

| # | Item | Verdict |
|---|------|---------|
| 1 | encryptedPublicKey plaintext layout | **FAIL** — ours is a 107-byte DIP-14 serialization; spec + both reference clients use the 69-byte compact (`fingerprint(4) ‖ chainCode(32) ‖ pubKey(33)`). Our current send path cannot even produce a valid document (128-byte ciphertext vs the contract's hard 96). Our receive path rejects reference-client payloads. |
| 2 | ECDH shared-key derivation | **PASS** — all three stacks compute libsecp256k1-style `SHA256((y[31]&0x1\|0x2) ‖ x)`. |
| 3 | accountReference | **PASS for cross-client interop** (recipients disregard it per DIP-15; our hardcoded 0 is harmless to mobile counterparties) — but **our compute helper is wrong on two axes** (HMAC input + ASK28 byte order) and must be fixed before we ever send real values. |
| + | senderKeyIndex / recipientKeyIndex conventions | **Interop hazard (bonus finding)** — mobile clients reference the identity's first ECDSA key (key 0, purpose AUTHENTICATION); our stack requires purpose ENCRYPTION/DECRYPTION on both send and receive, so cross-client requests fail key validation in both directions. |

---

## (1) encryptedPublicKey plaintext — FAIL

### What DIP-15 specifies

DIP-15 "Encrypted Extended Public Key (encryptedPublicKey)":

> The `encryptedPublicKey` property is a binary field that has the following format once it is
> deserialized to bytes:
> * Initialization Vector (16 bytes)
> * Encrypted extended public key with padding (80 bytes) that is encrypted by CBC-AES-256 using a
>   shared ECDH key
>
> The data format of the extended public key differs from what is defined in the Serialization
> Format of BIP32. This is because only the following fields are necessary when constructing
> derivations. The binary format used is as follows:
> * Parent fingerprint (4 bytes)
> * Chain code (32 bytes)
> * Public Key (33 bytes)

So the plaintext is the **compact 69-byte** form. 69 → PKCS7 pad → 80; +16 IV = the 96 bytes
that the deployed contract enforces (`packages/dashpay-contract/schema/v1/dashpay.schema.json:207-212`,
`minItems: 96, maxItems: 96`).

### What OUR stack encrypts

The send path (`packages/rs-platform-wallet/src/wallet/identity/network/contact_requests.rs:124-160`)
derives the full DashPay receiving path and encrypts `ExtendedPubKey::encode()`:

```rust
// contact_requests.rs:131-150
let account_type = AccountType::DashpayReceivingFunds {
    index: account_index,
    user_identity_id: sender_identity_id.to_buffer(),
    friend_identity_id: recipient_identity_id.to_buffer(),
};
let account_path = account_type.derivation_path(self.sdk.network) ...;
let account_xpub = wallet.derive_extended_public_key(&account_path) ...;
let xpub = account_xpub.encode();
```

That path is `m/9'/coin'/15'/0'/(sender_id)₂₅₆/(recipient_id)₂₅₆`
(key-wallet `account_type.rs:469-492` pushes two `ChildNumber::Normal256`), so the derived
key's `child_number.is_256_bits()` is true and `encode()` dispatches to the **DIP-14
107-byte** serialization, not the 78-byte BIP32 one:

```rust
// key-wallet/src/bip32.rs:1884-1890
pub fn encode(&self) -> Vec<u8> {
    if self.child_number.is_256_bits() {
        self.encode_256().to_vec()   // [u8; 107]: version(4)+depth(1)+fingerprint(4)+hardening(1)+child(32)+chaincode(32)+pubkey(33)
    } else {
        self.encode_32().to_vec()    // [u8; 78]: standard BIP32
    }
}
```

The bytes are passed verbatim through the write seam
(`network/sdk_writer.rs:254-257`) into `Sdk::send_contact_request` →
`create_contact_request` (`packages/rs-sdk/src/platform/dashpay/contact_request.rs:256-273`),
which AES-encrypts them via `encrypt_extended_public_key`
(`packages/rs-platform-encryption/src/lib.rs:97-105`, IV prepended) and then asserts:

```rust
// rs-sdk contact_request.rs:267-273
if encrypted_public_key.len() != 96 {
    return Err(Error::Generic(format!(
        "Encrypted public key size mismatch: expected 96 bytes, got {}", ...
```

**Arithmetic:** 107-byte plaintext → PKCS7 → 112 → +16 IV = **128 bytes ≠ 96**. The current
platform-wallet send path therefore errors at runtime on every real send — it doesn't merely
produce an incompatible document, it produces none. (The rs-sdk doc comment at
`contact_request.rs:150` saying "typically 78 bytes" is also wrong per DIP-15: a 78-byte BIP32
xpub would pass the 96-byte check but would still be undecryptable by reference clients.)

Our **receive** path is equally non-interoperable: after decrypting, it requires the plaintext
to be a 78- or 107-byte serialization:

```rust
// packages/rs-platform-wallet/src/wallet/identity/network/contacts.rs:447
let contact_xpub = key_wallet::bip32::ExtendedPubKey::decode(&decrypted_xpub_bytes) ...
// key-wallet/src/bip32.rs:1893-1899
pub fn decode(data: &[u8]) -> Result<ExtendedPubKey, Error> {
    match data.len() {
        78 => Self::decode_32(data),
        107 => Self::decode_256(data),
        _ => Err(Error::WrongExtendedKeyLength(data.len())),
```

A 69-byte compact payload from iOS/Android → `WrongExtendedKeyLength(69)` → treated as a
PERMANENT failure → `mark_contact_channel_broken`
(`network/contact_requests.rs:667-687`). **We can never pay a mobile contact.**

### What iOS DashSync encrypts

`DashSync/shared/Models/Identity/DSPotentialOneWayFriendship.m:114-133`
(https://github.com/dashpay/dashsync-iOS/blob/master/DashSync/shared/Models/Identity/DSPotentialOneWayFriendship.m):

```objc
- (void)encryptExtendedPublicKeyWithCompletion:(void (^)(BOOL success))completion {
    ...
    [self.sourceBlockchainIdentity encryptData:[DSKeyManager extendedPublicKeyData:self.extendedPublicKey]
                                withKeyAtIndex:self.sourceKeyIndex
                               forRecipientKey:recipientKey ...
```

`extendedPublicKeyData` resolves through dash-shared-core
(`dash-spv-masternode-processor/src/keys/ecdsa_key.rs:333-341`,
https://github.com/dashpay/dash-shared-core/blob/master/dash-spv-masternode-processor/src/keys/ecdsa_key.rs):

```rust
fn extended_public_key_data(&self) -> Option<Vec<u8>> {
    self.is_extended.then_some({
        let mut writer = Vec::<u8>::new();
        self.fingerprint.enc(&mut writer);   // 4 bytes
        self.chaincode.enc(&mut writer);     // 32 bytes
        writer.extend(self.public_key_data()); // 33 bytes (compressed)
        writer
    })
}
```

→ **69 bytes**, exactly the DIP-15 compact layout. (The fingerprint `u32` is read from and
written back as the same raw `HASH160[0..4]` bytes, so the wire bytes match dashj's
big-endian `putInt` — both emit the raw fingerprint bytes.)

### What Android encrypts

Send path, `android-dashpay dashpay/src/main/kotlin/org/dashj/platform/dashpay/ContactRequests.kt:22-52`
(https://github.com/dashpay/android-dashpay/blob/master/dashpay/src/main/kotlin/org/dashj/platform/dashpay/ContactRequests.kt):

```kotlin
val contactKeyChain = fromUser.getReceiveFromContactChain(toUser, aesKey)
val contactKey = contactKeyChain.watchingKey
val contactPub = contactKey.serializeContactPub()
...
val (encryptedContactPubKey, encryptedAccountLabel) = fromUser.encryptExtendedPublicKey(contactPub, toUser, toUserPublicKey.id, aesKey)
```

dashj `core/src/main/java/org/bitcoinj/crypto/DeterministicKey.java:584-607`
(https://github.com/dashpay/dashj/blob/master/core/src/main/java/org/bitcoinj/crypto/DeterministicKey.java):

```java
/** serializes a HD Key according to the dashpay encryptedPublicKey specification **/
public byte[] serializeContactPub() {
    ByteBuffer ser = ByteBuffer.allocate(69);
    ser.putInt(getParentFingerprint());
    ser.put(getChainCode());
    ser.put(getPubKey());
    checkState(ser.position() == 69);
    return ser.array();
}
public static DeterministicKey deserializeContactPub(NetworkParameters params, byte [] contactPub) {
    checkArgument(contactPub.length == 69);
    ...
}
```

→ **69 bytes**, and the Android receive path hard-rejects anything else
(`BlockchainIdentity.kt:1816` → `deserializeContactPub` `checkArgument(len == 69)`), so even
a 78-byte BIP32 plaintext from us would fail on Android.

### Required change (precise)

Send side — `send_contact_request_with_external_signer`
(`packages/rs-platform-wallet/src/wallet/identity/network/contact_requests.rs:150`): replace
`account_xpub.encode()` with the 69-byte compact assembly. The components already exist on
`ContactXpubData` (`crypto/dip14.rs:49-58`: `parent_fingerprint: [u8;4]`,
`chain_code: [u8;32]`, `public_key: [u8;33]`); nothing currently assembles them. Layout:
`parent_fingerprint ‖ chain_code ‖ public_key` (raw bytes, no version/depth/child-number).
Same change applies to any other producer feeding `get_extended_public_key` (rs-sdk-ffi
`src/dashpay/contact_request.rs` accepts caller bytes — Swift-side guidance must say
"69-byte compact"); the rs-sdk doc comments at `contact_request.rs:148-150,365-367`
("typically 78 bytes") need correcting.

Receive side — `register_external_contact_account`
(`packages/rs-platform-wallet/src/wallet/identity/network/contacts.rs:446-453`): replace
`ExtendedPubKey::decode(&decrypted)` with a 69-byte compact parser: split into
fingerprint/chaincode/pubkey and reconstruct an `ExtendedPubKey` with synthesized
depth/child-number (dashj does exactly this in `deserializeContactPub`, using depth 7 and
child 0 — only chaincode+pubkey matter for non-hardened child derivation). Reject ≠ 69 bytes.

Account-reference helper — `calculate_account_reference`
(`crypto/dip14.rs:147-172`) HMACs `contact_xpub.encode()` (107 bytes); per DIP-15 §"The data
format of the extended public key is an abbreviated version", the HMAC input must be the same
69-byte compact. (See item 3 for the ASK28 byte-order issue.)

### Blast radius

Effectively **zero for on-chain compatibility**: the current send path always fails the
96-byte assertion (128 ≠ 96) before broadcast, and the same `account_xpub.encode()` source
goes back through the file's history (verified at `c556a86db2~1`), so no contact-request
document with our 107-byte plaintext can exist on devnet/testnet. The contract's
`minItems/maxItems: 96` also makes any nonconforming document impossible to have landed.
Local consequence only: tests that feed synthetic 78-byte xpubs (e.g.
`rs-platform-encryption/src/lib.rs:236`, rs-sdk `contact_request.rs:482`) pin the wrong
plaintext size and will need updating to 69 — that is the "tests harden the wrong format"
risk this desk-check was meant to catch, confirmed.

---

## (2) ECDH derivation — PASS

DIP-15: "This shared key is derived using the libsecp256k1_ecdh method … calculate
`SHA256((y[31]&0x1|0x2) || x)`".

- **Ours** — `packages/rs-platform-encryption/src/lib.rs:24-34`:
  `dashcore::secp256k1::ecdh::SharedSecret::new(public_key, private_key)` — rust-secp256k1's
  `SharedSecret` is exactly libsecp256k1's default ECDH hash (SHA256 of compressed-point
  prefix ‖ x).
- **iOS** — dash-shared-core `ecdsa_key.rs:610-616`:
  ```rust
  impl DHKey for ECDSAKey {
      fn init_with_dh_key_exchange_with_public_key(public_key: &mut Self, private_key: &Self) -> Option<Self> {
          ... Some(Self::with_shared_secret(secp256k1::ecdh::SharedSecret::new(&pubkey, &seckey), false)),
  ```
  (same crate, same function; also used directly in `encrypt_with_secret_key_using_iv`,
  `ecdsa_key.rs:620-632`).
- **Android** — dashj `Secp256k1ECDHAgreement.java:99-105`
  (https://github.com/dashpay/dashj/blob/master/core/src/main/java/org/bitcoinj/crypto/Secp256k1ECDHAgreement.java):
  ```java
  // SHA256((y[31]&0x2|0x1) + x) ... (comment's mask is a typo; code below is &0x01|0x02)
  x32withVersion[0] = (byte)((y32[y32.length - 1] & 0x01) | 0x02);
  System.arraycopy(x32, 0, x32withVersion, 1, 32);
  return new BigInteger(Sha256Hash.hash(x32withVersion));
  ```

Symmetric cipher also matches everywhere: AES-256-CBC, PKCS7, random 16-byte IV, IV prepended
to the ciphertext (ours `rs-platform-encryption/src/lib.rs:45-105`; iOS
`crypto_data.rs:23-25` + IV-prepend in `ecdsa_key.rs:621,627-630`; Android
`KeyCrypterAESCBC.java:73-91` `PaddedBufferedBlockCipher(new CBCBlockCipher(new AESEngine()))`
+ IV-prepend in `BlockchainIdentity.kt:1750-1754`).

---

## (3) accountReference — PASS for interop; our helper is wrong for future use

DIP-15: "accountReference (integer) — … This is encrypted [masked] for the sender. **The
recipient should disregard this field.**"

**What reference clients SEND:** a *computed* masked value, not 0.

- iOS `DSPotentialOneWayFriendship.m:136-139`:
  `return key_create_account_reference(key, self.extendedPublicKey, self.account.accountNumber);`
  → dash-shared-core `bindings/keys.rs:1109-1130`: `HMAC-SHA256(sourceKey, 69-byte compact)`,
  `version = 0`, `version_bits | (ask28 ^ shortened_account_bits)`.
- Android `ContactRequests.kt:45` → `BlockchainIdentity.kt:1898-1916` (`getAccountReference`):
  `HDUtils.hmacSha256(privateKey.privKeyBytes, extendedPublicKey.serializeContactPub())`,
  version 0.

**What reference clients do on RECEIVE:** disregard/secondary. Android
`addContactPaymentKeyChain` (`BlockchainIdentity.kt:1819-1864`) reads it but the actual gate
is comparing the *decrypted xpub* against the locally derived account-0 xpub ("contactRequest
does not match account 0"); iOS likewise derives the friendship path from its own account and
uses the field only for the sender's own bookkeeping. Neither validates/un-masks a
counterparty's value.

**Ours:** the send path hardcodes 0 — `account_reference: account_index` where
`let account_index: u32 = 0;` (`network/contact_requests.rs:124,195`); the receive path
stores the field without interpreting it (`parse_contact_request_doc`,
`network/contact_requests.rs:437-439`). → **Interops fine with mobile clients today** (G3
holds), with two caveats:

1. Same-seed cross-wallet recovery: if a user imports our seed into DashWallet iOS/Android,
   the mobile app will un-mask our literal `0` to a garbage account index. Both mobile
   implementations fall back to xpub-vs-account-0 comparison / own derivation, so account 0
   still works, but account rotation (>0) from our side will not round-trip.
2. The DIP-15 unique index is `($ownerId, toUserId, accountReference)` — with a constant 0 we
   can never broadcast a superseding (rotated) request for the same pair.

**When we do implement real values**, `calculate_account_reference`
(`crypto/dip14.rs:147-172`) needs two fixes:
- HMAC input must be the **69-byte compact**, not `contact_xpub.encode()` (107 bytes).
- ASK28 byte interpretation: ours reads HMAC bytes `[0..4]` big-endian. iOS reads bytes
  `[28..32]` big-endian (`account_secret_key.reversed()` then `u32_le() >> 4`); Android reads
  bytes `[0..4]` little-endian (`Sha256Hash.wrapReversed(...).toBigInteger().toInt() ushr 4`).
  Note the two reference clients **disagree with each other** here, which is survivable only
  because nobody validates the field cross-client; for the same-seed-recovery scenario the
  pragmatic choice is to match whichever client our users co-install (decide at
  implementation time; flag upstream — this is a reference-client divergence worth a DIP
  clarification).

---

## (Bonus) senderKeyIndex / recipientKeyIndex conventions — interop hazard

- **iOS** (`DSBlockchainIdentity.m:3064-3065`): both indices =
  `firstIndexOfKeyOfType:self.currentMainKeyType` (ECDSA) → in practice the identity's key 0
  (a MASTER/AUTHENTICATION key).
- **Android** (`ContactRequests.kt:29-36,50`): `senderKeyIndex =
  getIdentityPublicKeyByPurpose(AUTHENTICATION).id`; `recipientKeyIndex` = first enabled
  `ECDSA_SECP256K1` key with `securityLevel <= MEDIUM` (any purpose) → in practice key 0.
- **Ours**: send requires sender key `Purpose::ENCRYPTION` and recipient key
  `Purpose::DECRYPTION` and errors otherwise (rs-sdk `contact_request.rs:211-234`;
  platform-wallet resolves indices the same way, `network/contact_requests.rs:98-119`);
  receive runs `validate_contact_request` (`crypto/validation.rs:76-150`) which demands
  ENCRYPTION/DECRYPTION purposes and **permanently marks the payment channel broken** on
  mismatch (`network/contact_requests.rs:649-665`).

Consequences: (a) we cannot SEND to a mobile-registered identity that lacks an
ENCRYPTION/DECRYPTION-purpose key (mobile identities typically register only
AUTHENTICATION-purpose keys); (b) requests FROM mobile clients reference AUTHENTICATION-purpose
key 0 and fail our receive-side validation → channel broken. The drive-side data trigger
(`rs-drive-abci/.../triggers/dashpay/v0/mod.rs`) validates only `ownerId != toUserId` and
`toUserId` existence, so this is purely a client-convention mismatch. Needs a re-scope
decision: relax our purpose check to accept ECDSA AUTHENTICATION keys (matching deployed
reference behavior), rather than waiting for the mobile ecosystem to adopt
ENCRYPTION/DECRYPTION-purpose keys.

---

## Re-scope recommendation

Items to fix before Milestone-2 tests harden formats:

1. **Plaintext format (blocker):** switch send to the 69-byte compact; switch receive to a
   compact parser. Touches `network/contact_requests.rs` (send), `network/contacts.rs`
   (receive), `crypto/dip14.rs` (helper + accountReference HMAC input), rs-sdk doc comments,
   and all xpub-size-pinning tests (78→69). No on-chain blast radius (current path cannot
   broadcast).
2. **Key purpose convention (blocker for cross-client):** accept/emit first-ECDSA-key
   (AUTHENTICATION) indices like the reference clients, or gate behind a compatibility mode.
3. **accountReference:** keep sending 0 for now (valid per DIP receiver semantics), but fix
   `calculate_account_reference`'s HMAC input when implementing rotation, and record the
   iOS/Android ASK28 divergence upstream.

---

## G15 testnet verification (2026-06-10)

Empirical check of the key-purpose convention against real testnet data (M1 task 8,
verification half). Data source: pshenmic platform-explorer REST API. The frontend at
`testnet.platform-explorer.com` proxies nothing — the API base URL is baked into the JS
bundle: **`https://testnet.platform-explorer.pshenmic.dev`** (routes in
`pshenmic/platform-explorer` `packages/api/src/routes.js`). Endpoints used:

```text
GET /dataContract/Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7/documents?document_type_name=contactRequest&limit=100&order=desc&page=N
GET /identity/<base58 id>     # includes full publicKeys[] with purpose/securityLevel/contractBounds
```

### Census: all 368 on-chain contactRequest documents (testnet, dash-testnet-51)

| (senderKeyIndex, recipientKeyIndex) | label? | docs | distinct owners | era |
|---|---|---|---|---|
| (2, 2) | yes | 223 | 126 | 2024–2026, dominant; still active 2026-06 |
| (4, 5) | no | 52 | 28 | 2026 only |
| (1, 0) | yes | 30 | 15 | 2024 only |
| (4, 5) | yes | 16 | 15 | 2026 only |
| (0, 0) | no | 16 | 1 | 2026 (single test identity `DcoJJ3W9…`) |
| (1, 1) | yes | 11 | 6 | 2024 |
| (2, 1) | mixed | 12 | 7 | mixed (incl. `DcoJJ3W9…` test traffic 2026-06) |
| other (2,0)/(5,5)/(4,4)/(1,2) | mixed | 8 | — | scattered |

("label?" = presence of optional `encryptedAccountLabel`, an Android-mobile tell.)
All `encryptedPublicKey` payloads observed are exactly **96 bytes**. `accountReference`
is a real non-zero value in almost all docs (one `(4,4)` doc has 0).

### Key purposes actually referenced (per-cohort identity lookups)

**Cohort (2,2)+label — dominant mobile population.** Sample senders
`85KDhzeJYhqirovivDN54V2n4qXPbZCxDYpvPuFbatJP`, `E5mQ8e9UDUgtFe6ScnUiX9UnsQK4waKyTSBsb5qxiBTS`;
sample recipient `EPsCcSgYKkcfrsVGJjHKzTnkscHV85SZ4dhaj1C3zek8`. Identity key layout:
`0=AUTHENTICATION/MASTER, 1=AUTHENTICATION/HIGH, 2=ENCRYPTION/MEDIUM (unbound), [3=TRANSFER]`.
→ **Both indices point at an ENCRYPTION-purpose, MEDIUM, NOT-contract-bound key (id 2).**
These identities have **no DECRYPTION key at all**.

**Cohort (4,5) — newest, 2026-only.** Sample sender
`FBSdgBCNu99mwXf6pxV2vGdsqZaABsLemf1DABMKYZk7`, recipient
`BBUJEMAiPLzu2P62PeY9oGvfonxTvbqUPZhM4WNoUsup`. Key layout:
`0–2=AUTHENTICATION, 3=TRANSFER, 4=ENCRYPTION/MEDIUM, 5=DECRYPTION/MEDIUM`, where keys 4
and 5 carry `contractBounds = {identifier: Bwr4WHCP…NS1C7, documentTypeName: contactRequest}`.
→ **sender=contract-bound ENCRYPTION, recipient=contract-bound DECRYPTION — exactly our
convention and exactly the contract's `requiresIdentityEncryptionBoundedKey: 2` /
`requiresIdentityDecryptionBoundedKey: 2` shape.** (A second (4,5)-shaped pair,
`4DtUte2t…`/`DyNXS4te…`, has the same purposes but unbound.)

**Cohort (1,0)/(1,1) — 2024 legacy.** Sample identities
`wHq4kk4wFk33A8ugtpYnuFoads5qxica3hT9egRQCdA`, `3paQGWPRFGg1iuH4o3Tjj6dek7Ebp7SxYGfaELxfAjzf`:
**only two keys, both AUTHENTICATION (0=MASTER, 1=HIGH)**. These docs reference
AUTHENTICATION keys because the identities had nothing else. No new docs in this shape
since 2024.

**Cohort (0,0)/(2,1) 2026 — test noise.** Owner `DcoJJ3W9…` (1,509 txs, 345 contracts,
dozens of `test-*.dash` aliases — a dev harness identity) created 2026 docs whose indices
point at AUTHENTICATION/MASTER (id 0) and AUTHENTICATION/CRITICAL (id 2) keys — *while the
same identity owns contract-bound ENCRYPTION(18)/DECRYPTION(19) keys it didn't reference*.
Its recipient `EesiqQz3…` has only AUTHENTICATION keys.

### Verdict

**(a) What purposes do real contactRequests reference?** Three populations, none of them
"key 0 by convention": the dominant mobile cohort (223/368 docs, 126 owners, still active
June 2026) references an **unbound ENCRYPTION/MEDIUM key (id 2) for BOTH indices** —
i.e., `recipientKeyIndex` points at the recipient's ENCRYPTION (not DECRYPTION) key. The
newest 2026 cohort (68 docs, ~40 owners) uses **contract-bound ENCRYPTION (sender) /
DECRYPTION (recipient)** — identical to our convention. AUTHENTICATION-key references
exist only in the dead 2024 cohort (whose identities had no other keys) and in one test
identity's 2026 noise.

**(b) Do sender identities have ENCRYPTION/DECRYPTION keys?** Modern mobile identities:
yes for ENCRYPTION (unbound id 2), **no DECRYPTION key at all**. Newest-cohort identities:
both, contract-bound, matching the contract requirement. 2024 identities: neither.

**(c) Does consensus enforce the bounded-key requirement on these fields?** **No.**
`senderKeyIndex`/`recipientKeyIndex` are plain integers; on-chain documents reference
AUTHENTICATION/MASTER keys (2026\!) and unbound ENCRYPTION keys without rejection. The
`requiresIdentity*BoundedKey: 2` contract flags govern bound-key *registration* shape, not
document validation, and the drive data trigger checks only `ownerId \!= toUserId` +
recipient existence. So the desk-check's "key 0 AUTHENTICATION" reading of the mobile
sources is **stale for current testnet**: deployed mobile clients since ~late 2024 *do*
register and reference an ENCRYPTION-purpose key. The real residual mismatch is narrower
than feared: mobile's `recipientKeyIndex` carries ENCRYPTION (not DECRYPTION) purpose and
is unbound, and mobile recipients have no DECRYPTION key for us to select when sending.

### Alignment recommendation (supersedes re-scope item 2 above)

- **Send:** keep current preference — sender ENCRYPTION key, recipient DECRYPTION key
  (live convention of the newest cohort). Add ONE fallback: if the recipient has no
  DECRYPTION-purpose key, select the recipient's **ENCRYPTION**-purpose ECDSA key (covers
  the entire 126-owner mobile population). Accept both bound and unbound keys on both
  sides. Do **not** fall back to AUTHENTICATION keys — no live client population needs it,
  and reusing signing keys for ECDH is poor key separation.
- **Receive/validate:** accept `senderKeyIndex` of purpose ENCRYPTION (bound or unbound);
  accept `recipientKeyIndex` (our key) of purpose ENCRYPTION **or** DECRYPTION. Keep the
  ECDSA_SECP256K1 key-*type* gate (every observed key is that type). On purpose mismatch
  (e.g. legacy 2024 AUTHENTICATION docs), degrade to a warning/skip — do **not**
  permanently mark the payment channel broken, since on-chain history demonstrably
  contains nonconforming-but-honest documents.
