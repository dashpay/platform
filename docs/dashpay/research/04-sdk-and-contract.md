# 04 — DashPay Contract & SDK/FFI Surface

Research into (A) the DashPay data-contract schema and (B) how `rs-sdk` / `rs-sdk-ffi` /
`rs-unified-sdk-ffi` expose DashPay operations — especially the `send_contact_request` path that does
ECDH + AES-256-CBC encryption internally (DIP-15). All file:line citations are against the worktree
`/Users/ivanshumkov/Projects/dashpay/platform.worktrees/dev`.

---

## PART A — DashPay Data Contract

### Contract identifiers

`packages/dashpay-contract/src/lib.rs:9-17`

```rust
pub const ID_BYTES: [u8; 32] = [
    162, 161, 180, 172, 111, 239, 34, 234, 42, 26, 104, 232, 18, 54, 68, 179, 87, 135, 95, 107, 65,
    44, 24, 16, 146, 129, 193, 70, 231, 178, 113, 188,
];
pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];
pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
```

- **DashPay contract ID (base58)**: `Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7`
  (hex `a2a1b4ac6fef22ea2a1a68e8123644b357875f6b412c18109281c146e7b271bc`).
- **Owner ID**: all-zero (`[0u8; 32]`) — system contract.
- Only **schema version 1** exists (`platform_version.system_data_contracts.dashpay == 1` →
  `v1::load_documents_schemas()`; any other version is an error). `load_definitions` returns
  `Ok(None)` for v1 (no `$defs`). See `lib.rs:19-38`.
- Rust accessors are minimal — `packages/dashpay-contract/src/v1/mod.rs:4-12` only exposes:
  `document_types::contact_request::NAME = "contactRequest"` and
  `document_types::contact_request::properties::TO_USER_ID = "toUserId"`. There are **no Rust
  constants for the `profile` or `contactInfo` document types** — code refers to them by string
  literal (`"profile"`, `"contactRequest"`, `"contactInfo"`).

> NOTE on a second ID seen in code: the SDK fallback constant
> `DASHPAY_CONTRACT_ID = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"` in
> `packages/rs-sdk/src/platform/dashpay/mod.rs:33` is the **DPNS** contract ID (also used in DPNS
> tests). It is only compiled in the `#[cfg(not(feature = "dashpay-contract"))]` path and is almost
> certainly a copy-paste bug — but in normal builds the `dashpay-contract` feature is on, so
> `SystemDataContract::Dashpay.id()` (= `Bwr4WHCP…`) is used and the wrong fallback is dead code.
> Flagged in "Gaps" below.

### Schema: `packages/dashpay-contract/schema/v1/dashpay.schema.json`

Three document types: `profile`, `contactInfo`, `contactRequest`. Property `position` numbers are
the binary serialization order.

#### `profile` (schema lines 2-74)

System-level: `minProperties: 1`, `additionalProperties: false`, `required: ["$createdAt",
"$updatedAt"]`.

| Property | pos | Type | Constraints |
|---|---|---|---|
| `avatarUrl` | 0 | string (uri) | minLength 1, maxLength 2048 |
| `avatarHash` | 1 | byteArray | exactly 32 bytes (SHA256 of avatar image) |
| `avatarFingerprint` | 2 | byteArray | exactly 8 bytes (dHash of avatar image) |
| `publicMessage` | 3 | string | minLength 1, maxLength 140 |
| `displayName` | 4 | string | minLength 1, maxLength 25 |

`dependentRequired`: any of `avatarUrl` / `avatarHash` / `avatarFingerprint` requires the other two.

**Indices**
- `ownerId` — `[$ownerId asc]`, **unique** (one profile per identity).
- `ownerIdAndUpdatedAt` — `[$ownerId asc, $updatedAt asc]` (non-unique).

#### `contactInfo` (schema lines 75-141)

`additionalProperties: false`; `required: ["$createdAt", "$updatedAt", "encToUserId",
"privateData", "rootEncryptionKeyIndex", "derivationEncryptionKeyIndex"]`.

| Property | pos | Type | Constraints |
|---|---|---|---|
| `encToUserId` | 0 | byteArray | exactly 32 bytes |
| `rootEncryptionKeyIndex` | 1 | integer | minimum 0 |
| `derivationEncryptionKeyIndex` | 2 | integer | minimum 0 |
| `privateData` | 3 | byteArray | 48–2048 bytes (encrypted CBOR of aliasName + note + displayHidden) |

**Indices**
- `ownerIdAndKeys` — `[$ownerId asc, rootEncryptionKeyIndex asc, derivationEncryptionKeyIndex asc]`,
  **unique**.
- `ownerIdAndUpdatedAt` — `[$ownerId asc, $updatedAt asc]` (non-unique).

#### `contactRequest` (schema lines 142-254)

Contract-level flags: `documentsMutable: false`, `canBeDeleted: false`,
`requiresIdentityEncryptionBoundedKey: 2`, `requiresIdentityDecryptionBoundedKey: 2`.
`additionalProperties: false`; `required: ["$createdAt", "$createdAtCoreBlockHeight", "toUserId",
"encryptedPublicKey", "senderKeyIndex", "recipientKeyIndex", "accountReference"]`.
(Note: `$createdAtCoreBlockHeight` is a required system field — pins core block height; there is no
`$updatedAt` because the doc is immutable.)

| Property | pos | Type | Constraints |
|---|---|---|---|
| `toUserId` | 0 | byteArray (identifier) | exactly 32 bytes; `contentMediaType: application/x.dash.dpp.identifier` |
| `encryptedPublicKey` | 1 | byteArray | **exactly 96 bytes** (16-byte IV + 80-byte AES-CBC ciphertext) |
| `senderKeyIndex` | 2 | integer | minimum 0 |
| `recipientKeyIndex` | 3 | integer | minimum 0 |
| `accountReference` | 4 | integer | minimum 0 |
| `encryptedAccountLabel` | 5 | byteArray | 48–80 bytes (16-byte IV + AES-CBC ciphertext), optional |
| `autoAcceptProof` | 6 | byteArray | 38–102 bytes, optional, **not encrypted** |

**Indices**
- `ownerIdUserIdAndAccountRef` — `[$ownerId asc, toUserId asc, accountReference asc]`, **unique**
  (one request per (sender, recipient, account)).
- `ownerIdUserId` — `[$ownerId asc, toUserId asc]` (non-unique).
- `userIdCreatedAt` — `[toUserId asc, $createdAt asc]` (received-requests timeline).
- `ownerIdCreatedAt` — `[$ownerId asc, $createdAt asc]` (sent-requests timeline).

---

## PART B — SDK / FFI DashPay surface

### B.1 — Crypto primitives: `platform-encryption` crate

All ECDH + AES-256-CBC lives in **`packages/rs-platform-encryption/src/lib.rs`** (crate
`platform-encryption`). This is what a prior agent meant by "rs-platform-wallet delegates encryption
to dash-sdk": the wallet calls `dash_sdk::platform::dashpay::send_contact_request`, which in turn
calls these `platform_encryption` functions.

**ECDH shared-secret derivation** (`lib.rs:24-34`) — DIP-15 uses libsecp256k1's ECDH, i.e.
`SHA256((y[31]&0x1|0x2) || x)`:

```rust
pub fn derive_shared_key_ecdh(private_key: &SecretKey, public_key: &PublicKey) -> [u8; 32] {
    use dashcore::secp256k1::ecdh::SharedSecret;
    let shared_secret = SharedSecret::new(public_key, private_key);
    let mut key = [0u8; 32];
    key.copy_from_slice(shared_secret.as_ref());
    key
}
```

**AES-256-CBC core** (`lib.rs:6-11, 45-60`) — `cbc::Encryptor<Aes256>` with PKCS7 padding; 32-byte
key, 16-byte IV:

```rust
type Aes256CbcEnc = cbc::Encryptor<Aes256>;

pub fn encrypt_aes_256_cbc(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let cipher = Aes256CbcEnc::new(key.into(), iv.into());
    // ... PKCS7-padded into buffer ...
    cipher.encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())...
}
```

**Extended-public-key encryption** (`lib.rs:97-105`) — IV is prepended to the ciphertext; for a
78-byte xpub this yields exactly **96 bytes** (16 IV + 80 ciphertext):

```rust
pub fn encrypt_extended_public_key(shared_key: &[u8; 32], iv: &[u8; 16], xpub: &[u8]) -> Vec<u8> {
    let encrypted_data = encrypt_aes_256_cbc(shared_key, iv, xpub);
    let mut result = Vec::with_capacity(16 + encrypted_data.len());
    result.extend_from_slice(iv);            // IV prepended per DIP-15
    result.extend_from_slice(&encrypted_data);
    result
}
```

**Account-label encryption** (`lib.rs:139-147`) — same IV-prepend shape, 48–80 bytes.
Decryption counterparts (`decrypt_extended_public_key` `lib.rs:115-128`, `decrypt_account_label`
`lib.rs:157-171`) split the first 16 bytes back off as IV. `CryptoError` enum at `lib.rs:174-184`.

### B.2 — High-level send flow: `rs-sdk`

Module: `packages/rs-sdk/src/platform/dashpay/` (declared in `packages/rs-sdk/src/platform.rs`).
`mod.rs` exposes the public types and two private helpers on `Sdk`:
`get_dashpay_contract_id()` (`mod.rs:23-42` — uses `SystemDataContract::Dashpay.id()`) and
`fetch_dashpay_contract()` (`mod.rs:45-63` — checks the context provider first, else fetches from
platform).

**Public API surface (re-exported at `mod.rs:9-13`):**
- Types: `ContactRequestInput`, `ContactRequestResult`, `EcdhProvider`, `RecipientIdentity`,
  `SendContactRequestInput`, `SendContactRequestResult`, `ContactRequestDocuments`.
- `Sdk::create_contact_request(...)` — builds the document locally (no broadcast).
- `Sdk::send_contact_request(...)` — builds + signs + broadcasts.
- Queries on `Sdk`: `fetch_sent_contact_requests`, `fetch_received_contact_requests`,
  `fetch_all_contact_requests_for_identity` (in `contact_request_queries.rs`).

**`EcdhProvider` — two modes** (`contact_request.rs:31-54`): `ClientSide { get_shared_secret }`
(hardware wallet supplies the 32-byte shared secret directly, given the recipient pubkey) and
`SdkSide { get_private_key }` (software wallet supplies the sender private key; the SDK does ECDH).

**`create_contact_request` signature** (`contact_request.rs:164-177`):

```rust
pub async fn create_contact_request<F, Fut, G, Gut, H, Hut>(
    &self,
    input: ContactRequestInput,
    ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
    get_extended_public_key: H,   // FnOnce(account_reference: u32) -> Future<Output=Vec<u8>>
) -> Result<ContactRequestResult, Error>
```

`ContactRequestInput` (`contact_request.rs:88-103`) carries `sender_identity: Identity`,
`recipient: RecipientIdentity` (an `Identifier` to be fetched, or a full `Identity`),
`sender_key_index`, `recipient_key_index`, `account_reference`, optional `account_label: String`
(the SDK encrypts it), optional `auto_accept_proof: Vec<u8>`.

**What `create_contact_request` does, in order:**
1. Validates `auto_accept_proof` is 38–102 bytes if present (`:179-186`).
2. Fetches the recipient `Identity` if only an ID was given (`:189-197`).
3. Looks up the **sender's encryption key** at `sender_key_index`, asserts `purpose() ==
   ENCRYPTION` (`:200-216`).
4. Looks up the **recipient's decryption key** at `recipient_key_index`, asserts `purpose() ==
   DECRYPTION`, parses it as a secp256k1 `PublicKey` (`:218-239`).
5. **Derives the shared secret** (`:241-253`):

```rust
let shared_key = match ecdh_provider {
    EcdhProvider::ClientSide { get_shared_secret } => {
        get_shared_secret(&recipient_public_key).await?
    }
    EcdhProvider::SdkSide { get_private_key } => {
        let sender_private_key = get_private_key(sender_key, input.sender_key_index).await?;
        derive_shared_key_ecdh(&sender_private_key, &recipient_public_key)
    }
};
```

6. Fetches the unencrypted extended public key via the caller's `get_extended_public_key` callback
   (`:256`).
7. **Encrypts the xpub** with a fresh random 16-byte IV and asserts the result is exactly 96 bytes
   (`:258-273`):

```rust
let mut rng = StdRng::from_entropy();
let mut xpub_iv = [0u8; 16];
rng.fill_bytes(&mut xpub_iv);
let encrypted_public_key =
    encrypt_extended_public_key(&shared_key, &xpub_iv, &extended_public_key);
if encrypted_public_key.len() != 96 { /* error */ }
```

8. If an `account_label` was given, encrypts it with a second fresh IV and asserts 48–80 bytes
   (`:276-291`).
9. Fetches the DashPay contract, gets the `contactRequest` document type, generates entropy +
   `Document::generate_document_id_v0(contract_id, sender_id, "contactRequest", entropy)`
   (`:293-314`).
10. Builds the property `BTreeMap`: `toUserId` (`Value::Identifier`), `encryptedPublicKey`
    (`Value::Bytes`, 96B), `senderKeyIndex`/`recipientKeyIndex`/`accountReference` (`Value::U32`),
    and optionally `encryptedAccountLabel` / `autoAcceptProof` (`:316-346`).

**`send_contact_request` signature** (`contact_request.rs:378-391`):

```rust
pub async fn send_contact_request<S: Signer<IdentityPublicKey>, F, Fut, G, Gut, H, Hut>(
    &self,
    input: SendContactRequestInput<S>,    // { contact_request, identity_public_key, signer }
    ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
    get_extended_public_key: H,
) -> Result<SendContactRequestResult, Error>
```

It calls `create_contact_request` (encryption happens there), wraps the result in a
`Document::V0 { … }` (`:414-429`), then broadcasts via the `PutDocument` trait
(`packages/rs-sdk/src/platform/transition/put_document.rs:20-47`):

```rust
let platform_document = document
    .put_to_platform_and_wait_for_response(
        self,
        contact_request_document_type.to_owned_document_type(),
        Some(entropy.0),
        input.identity_public_key,
        None,               // token payment info
        &input.signer,
        None,               // settings
    )
    .await?;
```

**Crypto details summarized:** key = 32-byte ECDH shared secret (`SHA256((y_parity)||x)`); cipher =
AES-256-CBC + PKCS7; IV = 16 random bytes generated per field, **prepended** to the ciphertext;
`encryptedPublicKey` is exactly 96 bytes (16 IV + 80 ct for a 78-byte xpub); `encryptedAccountLabel`
is 48–80 bytes. The 3 unit tests at `contact_request.rs:465-543` pin the 96-byte output, the
48–80-byte label range, the 38–102 proof range, and ECDH symmetry.

### B.3 — Wallet integration (rs-platform-wallet, brief)

`rs-platform-wallet` is the platform wallet that the Swift app drives. It **delegates** to the SDK
rather than re-implementing crypto:

- **Send path** — `packages/rs-platform-wallet/src/wallet/identity/network/contact_requests.rs:266`
  calls `self.sdk.send_contact_request(send_input, ecdh_provider, …)`. It builds an
  `EcdhProvider::SdkSide` whose `get_private_key` returns the wallet's ECDH key (with a key-id
  guard, `:240-261`), and signs with a HIGH/CRITICAL `AUTHENTICATION` ECDSA key
  (MASTER is rejected for document writes, `:200-218`).
- **Accept/receive path** — `packages/rs-platform-wallet/src/wallet/identity/network/contacts.rs:434`
  calls `platform_encryption::derive_shared_key_ecdh(&our_private_key, &contact_public_key)` then
  `platform_encryption::decrypt_extended_public_key(...)` directly to recover the contact's xpub.
- `account_labels.rs` also uses `platform_encryption` for label encryption.

### B.4 — `rs-sdk-ffi` exported DashPay functions

Module wiring: `packages/rs-sdk-ffi/src/lib.rs:14` (`mod dashpay;`) + `:46` (`pub use dashpay::*;`).
`dashpay/mod.rs` just re-exports `contact_request::*`. **All DashPay FFI lives in
`packages/rs-sdk-ffi/src/dashpay/contact_request.rs`** and is fully implemented (no stubs):

| `extern "C"` function | file:line | Status |
|---|---|---|
| `dash_sdk_dashpay_create_contact_request(handle, params) -> DashSDKResult` | `contact_request.rs:211` | implemented |
| `dash_sdk_dashpay_send_contact_request(handle, params, identity_public_key, signer) -> DashSDKResult` | `contact_request.rs:454` | implemented |
| `dash_sdk_dashpay_contact_request_result_free(result: *mut DashSDKContactRequestResult)` | `contact_request.rs:690` | implemented |
| `dash_sdk_dashpay_send_contact_request_result_free(result: *mut DashSDKSendContactRequestResult)` | `contact_request.rs:713` | implemented |

**`#[repr(C)]` types Swift uses:**
- `DashSDKEcdhMode` (`:131`) — `ClientSide = 0`, `SdkSide = 1`.
- `DashSDKContactRequestParams` (`:140-173`) — sender identity handle, recipient_id (32B),
  `fetch_recipient` bool, recipient identity handle, sender/recipient key indices,
  `account_reference`, NUL-terminated `account_label`, `auto_accept_proof` + len, `ecdh_mode`,
  `sender_private_key` (32B, SdkSide), `shared_secret` (32B, ClientSide), `extended_public_key` + len.
- `DashSDKContactRequestResult` (`:177`) — `document_id` (base58 C string), `owner_id` (base58),
  `properties_json` (JSON).
- `DashSDKSendContactRequestResult` (`:188`) — `document_json`, `recipient_id` (base58),
  `account_reference: u32`.

The two entry points pick the ECDH mode from `params.ecdh_mode` and route through four internal
async helpers (`create/send_contact_request_with_shared_secret` / `_with_private_key`,
`:19-127`) that use turbofish to fix the SDK's complex generic `EcdhProvider`. The signer is taken
as a **non-owning** `VTableSignerRef` (`:564`).

### B.5 — `rs-unified-sdk-ffi`

`packages/rs-unified-sdk-ffi/src/lib.rs` (entire file):

```rust
pub use dash_network;
pub use key_wallet_ffi;
pub use platform_wallet_ffi;
pub use rs_sdk_ffi;
```

It is a thin aggregator that **re-exports `rs_sdk_ffi` wholesale** (and `platform_wallet_ffi`,
`key_wallet_ffi`, `dash_network`). Because it builds as `staticlib`/`cdylib`
(`rs-unified-sdk-ffi/Cargo.toml:8`), all four `dash_sdk_dashpay_*` `#[no_mangle] extern "C"` symbols
from §B.4 are linked into the unified iOS framework and callable from Swift. There are **no
DashPay-specific functions defined in the unified crate itself.**

---

## Gaps, TODOs, stubs

1. **`send_contact_request` entropy mismatch (real bug)** —
   `packages/rs-sdk/src/platform/dashpay/contact_request.rs:431-435`: the entropy used to build the
   document ID in `create_contact_request` is **not** the entropy passed to
   `put_to_platform_and_wait_for_response`; `send_contact_request` generates *fresh* entropy at
   `:434`. The code comments call this out: *"In a real implementation, we'd need to store the
   entropy used during creation. For now, we'll generate new entropy (this is a simplification)."*
   The document ID and the state-transition entropy therefore diverge, which can cause the platform
   to compute a different document ID than `ContactRequestResult.id`. Worth verifying against
   `PutDocument` behavior (it may overwrite the ID from entropy via `set_id`, in which case the
   returned `result.id` from `create_contact_request` is stale rather than fatal).

2. **Wrong fallback contract ID** — `packages/rs-sdk/src/platform/dashpay/mod.rs:33` hardcodes
   `GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec`, which is the **DPNS** contract ID, not DashPay
   (correct DashPay = `Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7`). Only reachable when the
   `dashpay-contract` feature is **off**; in default builds it is dead code, but it is latent.

3. **No SDK helpers for `profile` or `contactInfo`** — `rs-sdk` only implements `contactRequest`
   create/send/query. Profiles and contactInfo documents must be built/put via the generic
   `Document` + `PutDocument` path; there are no `create_profile` / `put_profile` /
   `create_contact_info` helpers, and no Rust string constants for those type names in
   `dashpay-contract` (only `contactRequest` / `toUserId` are named in `v1/mod.rs`).

4. **No FFI for `profile` / `contactInfo`** — the FFI surface is limited to contact requests. Swift
   has no `dash_sdk_dashpay_*_profile` or `*_contact_info` entry points; those would go through the
   generic document FFI.

5. **No FFI for the receive/decrypt path** — there is no `dash_sdk_dashpay_decrypt_*` extern "C"
   function; decryption (`decrypt_extended_public_key` / `decrypt_account_label`) is only reachable
   from Rust (`rs-platform-wallet` uses it directly). The FFI exposes only the *send* direction of
   the contact-request handshake.
