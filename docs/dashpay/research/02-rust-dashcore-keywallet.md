# DashPay support in the rust-dashcore key-wallet

Research date: 2026-06-10
Repo root: `/Users/ivanshumkov/Projects/dashpay/rust-dashcore/`
Crates examined: `key-wallet`, `key-wallet-manager`, `key-wallet-ffi`.

This maps what the key-wallet stack provides for DashPay (contact-based funds):
derivation paths, account types, 256-bit (DIP14/DIP15) derivation, managed
accounts, transaction routing, and the FFI surface — plus what is **missing**
(notably: no ECDH / shared-secret code anywhere in the repo).

---

## 0. TL;DR

- **Derivation paths**: DIP9 `m/9'/coin'/15'` DashPay root constants exist
  (`DASHPAY_ROOT_PATH_MAINNET/TESTNET`). The per-contact path is
  `m/9'/coin'/15'/0'/<user_id>/<friend_id>` where `<user_id>`/`<friend_id>` are
  **non-hardened 256-bit** child numbers (the two identity IDs, 32 bytes each).
- **256-bit derivation (DIP14)**: fully implemented. `ChildNumber` has
  `Normal256 { index: [u8;32] }` / `Hardened256 { index: [u8;32] }`; `ckd_priv`
  and `ckd_pub_tweak` feed the raw 32 bytes into the BIP32 HMAC. Path strings
  parse 256-bit hex (`0x…`). Tested by `test_dashpay_vector_1..4`.
- **Account types**: `AccountType::DashpayReceivingFunds { index, user_identity_id, friend_identity_id }`
  and `AccountType::DashpayExternalAccount { … }` (watch-only, reversed id
  order). Mirrored by `ManagedAccountType`, keyed in collections by
  `DashpayAccountKey { index, user_identity_id, friend_identity_id }`.
- **Managed accounts / addresses**: each DashPay account holds a single
  `AddressPool` with gap limit **20**. The 256-bit derivation is done once at
  account creation to produce the account xpub; address generation then appends
  an ordinary **non-hardened u32 leaf** — standard BIP32 from there on.
- **Transaction checking**: incoming txs are routed to DashPay accounts via
  `AccountTypeToCheck::DashpayReceivingFunds` / `DashpayExternalAccount`, which
  iterate the `dashpay_*_accounts` maps and address-match each pool.
- **FFI**: `wallet_add_dashpay_receiving_account`,
  `wallet_add_dashpay_external_account_with_xpub_bytes`,
  `managed_wallet_get_dashpay_receiving_account`,
  `managed_wallet_get_dashpay_external_account` — all take
  `user_identity_id` + `friend_identity_id` as 32-byte pointers.
- **GAPS**: **No ECDH / shared-secret / xpub-encryption code exists in the whole
  repo.** DashPay accounts are **not auto-created** at wallet init. The helper
  `extended_public_key_for_account_type` returns `None` for DashPay
  ("Currently not retrieved via this helper"). No FFI to compute the per-contact
  derivation path or to derive an xpub from two identity IDs directly.

---

## 1. DIP9 feature paths (`key-wallet/src/dip9.rs`)

### Feature-purpose constants (`dip9.rs:124-140`)

```rust
pub const BIP44_PURPOSE: u32 = 44;
pub const FEATURE_PURPOSE: u32 = 9;
pub const DASH_COIN_TYPE: u32 = 5;
pub const DASH_TESTNET_COIN_TYPE: u32 = 1;
pub const FEATURE_PURPOSE_COINJOIN: u32 = 4;
pub const FEATURE_PURPOSE_IDENTITIES: u32 = 5;
pub const FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_AUTHENTICATION: u32 = 0;
pub const FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_REGISTRATION: u32 = 1;
pub const FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_TOPUP: u32 = 2;
pub const FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_INVITATIONS: u32 = 3;
pub const FEATURE_PURPOSE_ASSET_LOCK_SUBFEATURE_ADDRESS_TOPUP: u32 = 4;
pub const FEATURE_PURPOSE_ASSET_LOCK_SUBFEATURE_SHIELDED_ADDRESS_TOPUP: u32 = 5;
pub const FEATURE_PURPOSE_DASHPAY: u32 = 15;          // <-- DashPay feature index
pub const FEATURE_PURPOSE_PLATFORM_PAYMENT: u32 = 17; // DIP-17
```

So DashPay is feature index **15'** under purpose **9'** (NOT under `purpose 15'`
— note the inline test comment in derivation.rs that says "m/15'/5'/15'" is
wrong; the actual constant builds `m/9'/coin'/15'`).

### DashPay root path constants (`dip9.rs:167-198`)

```rust
// DashPay Root Paths
pub const DASHPAY_ROOT_PATH_MAINNET: IndexConstPath<3> = IndexConstPath {
    indexes: [
        ChildNumber::Hardened { index: FEATURE_PURPOSE },          // 9'
        ChildNumber::Hardened { index: DASH_COIN_TYPE },           // 5'
        ChildNumber::Hardened { index: FEATURE_PURPOSE_DASHPAY },  // 15'
    ],
    reference: DerivationPathReference::ContactBasedFunds,
    path_type: DerivationPathType::CLEAR_FUNDS,
};

pub const DASHPAY_ROOT_PATH_TESTNET: IndexConstPath<3> = IndexConstPath {
    indexes: [
        ChildNumber::Hardened { index: FEATURE_PURPOSE },          // 9'
        ChildNumber::Hardened { index: DASH_TESTNET_COIN_TYPE },   // 1'
        ChildNumber::Hardened { index: FEATURE_PURPOSE_DASHPAY },  // 15'
    ],
    reference: DerivationPathReference::ContactBasedFunds,
    path_type: DerivationPathType::CLEAR_FUNDS,
};
```

So the DashPay root is `m/9'/5'/15'` (mainnet) / `m/9'/1'/15'` (testnet).

### DerivationPathReference enum (`dip9.rs:13-34`)

DashPay-relevant references:

```rust
ContactBasedFunds = 8,          // DashpayReceivingFunds maps here
ContactBasedFundsRoot = 9,
ContactBasedFundsExternal = 10, // DashpayExternalAccount maps here
```

### Per-contact path layout

Built in `AccountType::derivation_path` (`account/account_type.rs:469-514`). The
full per-contact path is:

- **Receiving** (`DashpayReceivingFunds`):
  `m/9'/coin'/15'/0'/<user_identity_id>/<friend_identity_id>`
- **External / watch-only** (`DashpayExternalAccount`):
  `m/9'/coin'/15'/0'/<friend_identity_id>/<user_identity_id>` (ids reversed)

The `0'` is a hardened account-level index. The two trailing components are
**non-hardened `Normal256`** child numbers carrying the raw 32-byte identity IDs:

```rust
Self::DashpayReceivingFunds { user_identity_id, friend_identity_id, .. } => {
    let mut path = /* DASHPAY_ROOT_PATH_{MAINNET,TESTNET} */;
    path.push(ChildNumber::from_hardened_idx(0)?);          // account 0'
    path.push(ChildNumber::Normal256 { index: *user_identity_id });   // non-hardened 256-bit
    path.push(ChildNumber::Normal256 { index: *friend_identity_id });
    Ok(path)
}
// DashpayExternalAccount pushes friend_id THEN user_id (reversed) — account_type.rs:507-512
```

Comment in source: *"Base DashPay root + account 0' + user_id/friend_id
(non-hardened per DIP-14/DIP-15)"* (`account_type.rs:474`).

---

## 2. DIP14 256-bit derivation (`key-wallet/src/bip32.rs`)

### ChildNumber variants (`bip32.rs:575-598`)

```rust
pub enum ChildNumber {
    Normal    { index: u32 },        // [0, 2^31-1]
    Hardened  { index: u32 },        // [0, 2^31-1]
    Normal256 { index: [u8; 32] },   // [0, 2^256-1]  <-- DIP14
    Hardened256 { index: [u8; 32] }, // [0, 2^256-1]  <-- DIP14
}
```

Constructors (`bip32.rs:650-662`):

```rust
pub fn from_normal_idx_256(index: [u8; 32]) -> ChildNumber { ChildNumber::Normal256 { index } }
pub fn from_hardened_idx_256(index: [u8; 32]) -> ChildNumber { ChildNumber::Hardened256 { index } }
```

`is_256_bits()` (`bip32.rs:691`) and `is_hardened()` (`bip32.rs:674`, where
`Normal256 => false`, `Hardened256 => true`) gate the encoding/derivation
branches.

### Child key derivation — private (`bip32.rs:1533-1589`)

The 256-bit branches feed the **raw 32-byte index** into the HMAC-SHA512 engine
(no big-endian-u32 conversion), which is exactly DIP14:

```rust
pub fn ckd_priv<C: secp256k1::Signing>(&self, secp: &Secp256k1<C>, i: ChildNumber)
    -> Result<ExtendedPrivKey, Error>
{
    let mut hmac_engine = HmacEngine::<sha512::Hash>::new(&self.chain_code[..]);
    match i {
        ChildNumber::Normal { index } => {
            hmac_engine.input(&PublicKey::from_secret_key(secp, &self.private_key).serialize()[..]);
            hmac_engine.input(&index.to_be_bytes());
        }
        ChildNumber::Hardened { index } => {
            hmac_engine.input(&[0u8]);
            hmac_engine.input(&self.private_key[..]);
            hmac_engine.input(&(index | (1 << 31)).to_be_bytes());
        }
        ChildNumber::Normal256 { index } => {                 // DIP14 non-hardened
            hmac_engine.input(&PublicKey::from_secret_key(secp, &self.private_key).serialize()[..]);
            hmac_engine.input(&index);                         // raw 32 bytes
        }
        ChildNumber::Hardened256 { index } => {               // DIP14 hardened
            hmac_engine.input(&[0u8]);
            hmac_engine.input(&self.private_key[..]);
            hmac_engine.input(&index);                         // raw 32 bytes
        }
    }
    let hmac_result = Hmac::<sha512::Hash>::from_engine(hmac_engine);
    let sk = SecretKey::from_slice(&hmac_result[..32])?;
    let tweaked = sk.add_tweak(&self.private_key.into())?;
    Ok(ExtendedPrivKey { /* depth+1, child_number: i, private_key: tweaked, chain_code: from_hmac */ })
}
```

`ckd_pub_tweak` (`bip32.rs:1817+`) has the symmetric public-key branches so the
**non-hardened** `Normal256` path can be derived from an xpub alone — important
for DashPay external (watch-only) accounts where you only have the friend's
xpub.

### How identity IDs become derivation indices

There is **no dedicated "identity-id → index" helper**. The identity ID *is* the
index: it's a raw `[u8; 32]` placed directly into `ChildNumber::Normal256`
(see §1). Two paths to construct one:

1. Programmatically: `account_type.derivation_path(network)` builds it (§1).
2. From a string: `DerivationPath::from_str` parses `0x<64 hex>` segments into
   `Normal256`/`Hardened256` (`bip32.rs:855-905`):

```rust
if index_str.starts_with("0x") {
    // decode 32 bytes; trailing ' => Hardened256 else Normal256
}
```

### Binary encoding

Extended keys with 256-bit child numbers serialize to **107 bytes** (vs 78 for
32-bit) — `decode` dispatches on length (`bip32.rs:1602-1604`), and there are
dedicated `encode_256`/`decode_256` paths for both xpriv and xpub
(`bip32.rs:1654-2021`). DIP-14 binary format comment at `bip32.rs:1883`.

### Test vectors (`bip32.rs:2521-2594`)

`test_dashpay_vector_1..4` derive against real DashPay-shaped paths, e.g.:

```
m/9'/5'/15'/0'/0x555d…cfc3a'/0xa137…89b5'/0
```

and assert exact `tprv…`/`tpub…` strings — proving the 256-bit DashPay
derivation is correct and stable.

---

## 3. Account types (`key-wallet/src/account/account_type.rs`)

### The two DashPay variants (`account_type.rs:76-95`)

```rust
/// Incoming DashPay funds account using 256-bit derivation
/// The derivation path used is user_identity_id/friend_identity_id
DashpayReceivingFunds {
    index: u32,                      // account-level selection
    user_identity_id: [u8; 32],      // our identity id
    friend_identity_id: [u8; 32],    // contact's identity id
},
/// DashPay external (watch-only) account using 256-bit derivation
/// The derivation path used is friend_identity_id/user_identity_id
DashpayExternalAccount {
    index: u32,
    user_identity_id: [u8; 32],
    friend_identity_id: [u8; 32],
},
```

- **`DashpayReceivingFunds`** = funds *we* receive from a contact; derived from
  our own key material; path `…/0'/user/friend`. Maps to
  `DerivationPathReference::ContactBasedFunds` (`account_type.rs:300-302`).
- **`DashpayExternalAccount`** = the contact's *external* (watch-only) view of
  where *they* will send; path `…/0'/friend/user` (reversed); typically created
  from the contact's xpub. Maps to `ContactBasedFundsExternal`
  (`account_type.rs:303-305`).

`index()` returns `Some(index)` for both (`account_type.rs:218-225`).

### How a friendship/contact maps to an account

A friendship = the ordered pair `(our identity, contact identity)`. Each
direction is a distinct account:

- We receive from contact → `DashpayReceivingFunds { user=ours, friend=theirs }`.
- We watch where the contact receives (so we know where to pay them) →
  `DashpayExternalAccount`.

In collections (`account/account_collection.rs:19-29, 75-80`) they're stored in
two `BTreeMap`s keyed by:

```rust
pub type DashpayOurUserIdentityId = [u8; 32];
pub type DashpayContactIdentityId = [u8; 32];

pub struct DashpayAccountKey {
    pub index: u32,
    pub user_identity_id: DashpayOurUserIdentityId,
    pub friend_identity_id: DashpayContactIdentityId,
}

// in AccountCollection:
pub dashpay_receival_accounts:  BTreeMap<DashpayAccountKey, Account>,
pub dashpay_external_accounts:  BTreeMap<DashpayAccountKey, Account>,
```

`AccountCollection::insert` (`account_collection.rs:162-184`) routes a built
`Account` into the right map based on `AccountType`.

### Path construction

`derivation_path(network)` for both variants — see §1 (quoted from
`account_type.rs:469-514`).

---

## 4. Managed accounts (`key-wallet/src/managed_account/`)

### ManagedAccountType variants (`managed_account_type.rs:97-118`)

```rust
DashpayReceivingFunds {
    index: u32,
    user_identity_id: DashpayOurUserIdentityId,
    friend_identity_id: DashpayContactIdentityId,
    addresses: AddressPool,        // single pool
},
DashpayExternalAccount {
    index: u32,
    user_identity_id: DashpayOurUserIdentityId,
    friend_identity_id: DashpayContactIdentityId,
    addresses: AddressPool,        // single pool
},
```

Each DashPay account is **single-pool** (one `AddressPool`, no separate
internal/change pool) — `address_pools()` returns `vec![addresses]`
(`managed_account_type.rs:263-274`).

### Construction & gap limit (`managed_account_type.rs:706-749`)

`ManagedAccountType::from_account_type` builds the pool from the account's
256-bit derivation path with **gap limit 20**, pool type `Absent`:

```rust
AccountType::DashpayReceivingFunds { index, user_identity_id, friend_identity_id } => {
    let path = account_type.derivation_path(network)...;      // the 256-bit contact path
    let pool = AddressPool::new(path, AddressPoolType::Absent, 20, network, key_source)?;
    Ok(Self::DashpayReceivingFunds { index, user_identity_id, friend_identity_id, addresses: pool })
}
```

(The literal `20` here is the DashPay gap limit; compare `DIP17_GAP_LIMIT = 20`,
`DEFAULT_SPECIAL_GAP_LIMIT = 5`, `DEFAULT_EXTERNAL_GAP_LIMIT = 30` in
`gap_limit.rs`.)

### Address generation — the 256-bit part is done once

Important architectural detail: the **256-bit DIP14 derivation happens once at
account-creation time** to produce the account-level xpub. From that xpub, the
`AddressPool` generates addresses by appending an **ordinary non-hardened u32
leaf** (`address_pool.rs:427-440`):

```rust
pub(crate) fn generate_address_at_index(&..., index: u32) {
    let mut full_path = /* pool base path */;
    full_path.push(ChildNumber::from_normal_idx(index)?);   // plain 32-bit leaf
    // derive_pub via KeySource::derive_at_path (address_pool.rs:128-142)
}
```

So once the account xpub exists, address generation/gap-limit/balance tracking
for a contact is identical to any other single-pool account. `KeySource`
(`address_pool.rs:128-142`) derives via `xpub.derive_pub` / `xprv.derive_priv`,
which transparently handle 256-bit segments if present.

### Managed collection storage (`managed_account_collection.rs:71-73, 244-268`)

```rust
pub dashpay_receival_accounts: BTreeMap<DashpayAccountKey, ManagedCoreFundsAccount>,
pub dashpay_external_accounts: BTreeMap<DashpayAccountKey, ManagedCoreFundsAccount>,
```

`insert` / `insert_funds_bearing_account` route managed DashPay accounts into
these maps keyed by `DashpayAccountKey`.

### Balance / UTXO tracking

DashPay managed accounts are `ManagedCoreFundsAccount` (funds-bearing), so they
get the full `ManagedAccountTrait` (`managed_account_trait.rs:29+`) — balance,
UTXOs, transaction records, chainlock/instantsend finality — exactly like
standard accounts. Nothing DashPay-specific in the balance machinery.

---

## 5. ECDH / shared secret — **ABSENT**

**There is NO ECDH, shared-secret, Diffie-Hellman, key-agreement, or
extended-public-key-encryption code anywhere in rust-dashcore.** Repo-wide grep
(all crates, excluding `target/`):

```
grep -rinE '\becdh\b|shared_secret|diffie.hellman|SharedSecret|key_agreement' --include='*.rs'
  -> (no matches)
grep -rinE 'encrypt.*extended|extended.*encrypt' --include='*.rs'
  -> (no matches)
```

Implications for the platform wallet:

- The DIP15 "encrypt the contact xpub with an ECDH-derived key, store it in the
  DashPay contact-request document" step is **not** provided here. Key-wallet
  gives you the *derivation* primitives (256-bit account xpub from your seed +
  the two identity IDs) but **not** the ECDH shared key needed to
  encrypt/decrypt that xpub for the on-platform contact request.
- The platform wallet must compute the ECDH shared secret itself (e.g. via
  `secp256k1` ECDH on the two identities' encryption public keys) and do the
  symmetric encryption of the extended public key. key-wallet only consumes the
  *already-decrypted* friend xpub (passed into
  `wallet_add_dashpay_external_account_with_xpub_bytes`).

`secp256k1` *is* a dependency, so the building block (raw ECDH) is reachable —
it's just not wired up in key-wallet.

---

## 6. Transaction checking (`key-wallet/src/transaction_checking/`)

### Routing enum (`transaction_router/mod.rs:183-198`)

```rust
pub enum AccountTypeToCheck {
    // … StandardBIP44, CoinJoin, Identity*, AssetLock*, Provider* …
    DashpayReceivingFunds,
    DashpayExternalAccount,
}
```

`AccountType -> AccountTypeToCheck` conversion (`account_type.rs:190-195`) and
`ManagedAccountType -> AccountTypeToCheck` (`transaction_router/mod.rs:260-265,
325-330`). Note `PlatformPayment` returns `Err(PlatformAccountConversionError)`
because it's Platform-only; DashPay variants convert fine (they're real Core
on-chain funds).

### Matching txs to the right contact account (`account_checker.rs:501-518`)

```rust
AccountTypeToCheck::DashpayReceivingFunds => {
    let mut matches = Vec::new();
    for (key, account) in &self.dashpay_receival_accounts {
        if let Some(m) = account.check_transaction_for_match(tx, Some(key.index)) {
            matches.push(m);
        }
    }
    matches
}
AccountTypeToCheck::DashpayExternalAccount => { /* same over dashpay_external_accounts */ }
```

So an incoming payment is matched by iterating every DashPay account's address
pool(s) and address-matching the tx outputs — the same address-pool matching as
every other account type, just bucketed by `DashpayAccountKey`.

### Match result (`account_checker.rs:144-153`)

```rust
CoreAccountTypeMatch::DashpayReceivingFunds { account_index: u32, involved_addresses: Vec<AddressInfo> },
CoreAccountTypeMatch::DashpayExternalAccount { account_index: u32, involved_addresses: Vec<AddressInfo> },
```

Note: the match carries only `account_index`, **not** the two identity IDs — to
resolve which *contact* matched, the caller maps `account_index` back through
the collection. (The `Display` impl also elides the 32-byte ids for log
readability — `account_type.rs:143-150`.)

### key-wallet-manager events (`key-wallet-manager/src/events.rs`)

The block-processing layer is contact-aware only indirectly: event docs
reference the account type "(which carries any account-level indices like the
Dashpay `user_identity_id` / `friend_identity_id`)" (`events.rs:41-42`). There
is **no DashPay-specific event variant** — DashPay funds surface through the
generic per-account match/transaction events.

---

## 7. FFI surface (`key-wallet-ffi/src/`)

### Create / add DashPay accounts (`wallet.rs`)

```c
// wallet.rs:397 — derive from wallet seed
FFIAccountResult wallet_add_dashpay_receiving_account(
    FFIWallet *wallet, unsigned int account_index,
    const uint8_t *user_identity_id /*32*/, const uint8_t *friend_identity_id /*32*/);

// wallet.rs:451 — watch-only, supply the contact's account xpub bytes
FFIAccountResult wallet_add_dashpay_external_account_with_xpub_bytes(
    FFIWallet *wallet, unsigned int account_index,
    const uint8_t *user_identity_id /*32*/, const uint8_t *friend_identity_id /*32*/,
    const uint8_t *xpub_bytes, size_t xpub_len);
```

`wallet_add_dashpay_receiving_account` builds
`AccountType::DashpayReceivingFunds`, calls `add_account(acct, None)` (derives
from seed). The external one decodes the supplied xpub
(`ExtendedPubKey::decode`, which accepts the 107-byte 256-bit format) and calls
`add_account(acct, Some(xpub))`.

### Fetch managed DashPay accounts (`managed_account.rs`)

```c
// managed_account.rs:436
FFIManagedCoreAccountResult managed_wallet_get_dashpay_receiving_account(
    /* wallet, */ unsigned int account_index,
    const uint8_t *user_identity_id /*32*/, const uint8_t *friend_identity_id /*32*/);

// managed_account.rs:497
FFIManagedCoreAccountResult managed_wallet_get_dashpay_external_account(...same args...);
```

### Generic derivation FFI (usable but not DashPay-aware)

- `derivation_derive_private_key_from_seed(seed, path_str)` —
  `DerivationPath::from_str(path_str)` accepts `0x<64hex>` 256-bit segments, so a
  caller *could* hand-build a DashPay path string and derive
  (`derivation.rs:323`).
- `account_derive_extended_private_key_at` / `account_derive_private_key_at` /
  `account_derive_*_from_{seed,mnemonic}` (`account_derivation.rs:32-377`) —
  generic per-account child derivation.
- `wallet_derive_*` (`keys.rs:122-356`).

There is **no** FFI that takes two identity IDs and returns the per-contact
derivation path or the per-contact account xpub directly (you go through the
add-account calls), and **no** FFI for ECDH/shared-key.

---

## 8. Gaps / TODOs / incomplete

1. **No ECDH / shared-secret / xpub-encryption** anywhere (see §5). This is the
   biggest gap for DashPay: the DIP15 contact-request xpub encryption must be
   built in the platform wallet, not reused from key-wallet.
2. **DashPay accounts are not auto-created at wallet init.** `from_mnemonic` ->
   `create_accounts_from_options` does not include DashPay accounts; you add
   them per-contact after the fact via `add_account` /
   `wallet_add_dashpay_*`. (Confirmed: no DashPay branch in
   `wallet/initialization.rs`; the only mention is a doc comment.)
3. **`extended_public_key_for_account_type` returns `None` for DashPay**
   (`wallet/helper.rs:582-586`): *"Currently not retrieved via this helper"* —
   so you can't pull a DashPay account xpub through that generic accessor; use
   the account stored in the collection instead.
4. **Match results drop the identity IDs.** `CoreAccountTypeMatch::Dashpay*`
   carries only `account_index` (`account_checker.rs:144-153`); resolving which
   contact requires a reverse lookup via `DashpayAccountKey`. With multiple
   contacts sharing the same `account_index` (different ids), matching iterates
   all of them and the caller must disambiguate.
5. **No DashPay-specific event** in key-wallet-manager (§6); contact funds flow
   through generic account events.
6. **External-account xpub must be pre-decrypted.** The FFI takes raw xpub bytes;
   acquiring the friend's xpub from the on-platform DashPay contact-request
   document (decrypting it) is out of scope for key-wallet.

No `todo!()` / `unimplemented!()` were found in DashPay code paths — the gaps are
"feature not present" rather than "stubbed".

---

## Key file:line index

- DashPay path constants: `key-wallet/src/dip9.rs:138, 167-198`; references enum `:13-34`.
- 256-bit ChildNumber + DIP14 derivation: `key-wallet/src/bip32.rs:575-598, 650-662, 1533-1589, 1817+`; 256-hex parse `:855-905`; test vectors `:2521-2594`.
- AccountType DashPay variants + per-contact path: `key-wallet/src/account/account_type.rs:76-95, 218-225, 300-305, 469-514`.
- Collection keys/maps: `key-wallet/src/account/account_collection.rs:19-29, 75-80, 162-184`.
- ManagedAccountType + gap limit 20: `key-wallet/src/managed_account/managed_account_type.rs:97-118, 263-274, 706-749`.
- Address pool leaf derivation: `key-wallet/src/managed_account/address_pool.rs:128-142, 427-440`.
- Managed collection maps: `key-wallet/src/managed_account/managed_account_collection.rs:71-73, 244-268`.
- Tx routing + matching: `key-wallet/src/transaction_checking/transaction_router/mod.rs:183-198, 260-265`; `account_checker.rs:144-153, 501-518`.
- helper gap (xpub None for DashPay): `key-wallet/src/wallet/helper.rs:582-586`.
- add_account flow: `key-wallet/src/wallet/accounts.rs:28-64`.
- FFI: `key-wallet-ffi/src/wallet.rs:397, 451`; `key-wallet-ffi/src/managed_account.rs:436, 497`; generic derive `key-wallet-ffi/src/derivation.rs:323`.
- ECDH absence: repo-wide grep, no matches.
