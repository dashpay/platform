# Platform Addresses

Dash Platform has its own address system, independent of the legacy Base58Check addresses used on the Core chain. Platform addresses use **Bech32m** encoding (BIP-350), following the specification in **DIP-0018**. There are three address types: two transparent and one shielded.

If you are coming from Bitcoin or Dash Core, the key mental shift is this: platform addresses are not derived from a single private key via a single algorithm. They are typed containers for a hash or a public key, unified under a single encoding scheme with a shared human-readable prefix.

## The Three Address Types

| Type | Inner Data | Bech32m Type Byte | Prefix (mainnet) | Prefix (testnet) |
|------|-----------|-------------------|-------------------|-------------------|
| P2PKH | 20-byte pubkey hash | `0xb0` | `dash1k...` | `tdash1k...` |
| P2SH | 20-byte script hash | `0x80` | `dash1s...` | `tdash1s...` |
| Orchard | 43-byte shielded address | `0x10` | `dash1z...` | `tdash1z...` |

The type byte is the first byte of the Bech32m data payload. It determines how the rest of the payload is interpreted. The type bytes were chosen so that the first character after `dash1` (or `tdash1`) is a memorable letter: **k** for keys (P2PKH), **s** for scripts (P2SH), **z** for zero-knowledge (Orchard).

## PlatformAddress

The transparent address type is defined in `packages/rs-dpp/src/address_funds/platform_address.rs`:

```rust
pub enum PlatformAddress {
    P2pkh([u8; 20]),
    P2sh([u8; 20]),
}
```

Two variants, both wrapping a 20-byte hash. A `P2pkh` address contains a `Hash160(compressed_pubkey)`, just like on Core. A `P2sh` address contains a `Hash160(redeem_script)`, supporting standard multisig scripts.

### Bech32m Encoding

The human-readable part (HRP) depends on the network:

```rust
const PLATFORM_HRP_MAINNET: &str = "dash";
const PLATFORM_HRP_TESTNET: &str = "tdash";  // also devnet, regtest
```

Encoding produces addresses like:
- `dash1krma5z3ttj75la4m93xcndna9ullamq9y5e9n5rs` (P2PKH, mainnet)
- `tdash1sppl5xpu70aka8nacc4kj2htflydspzkc8jtru5` (P2SH, testnet)

The wire format is `type_byte || hash` -- 21 bytes total, then Bech32m-encoded with the appropriate HRP.

### Storage vs. Bech32m: Two Byte Schemes

There is an important distinction between how addresses are encoded for users and how they are serialized for storage. The Bech32m type bytes (`0xb0`, `0x80`) are *not* the same as the bincode variant indices (`0x00`, `0x01`) used in GroveDB keys:

| Context | P2PKH byte | P2SH byte |
|---------|-----------|-----------|
| Bech32m (user-facing) | `0xb0` | `0x80` |
| Bincode (storage/wire) | `0x00` | `0x01` |

This matters when you encounter raw bytes. If the leading byte is `0xb0` or `0x80`, you are looking at a Bech32m payload. If it is `0x00` or `0x01`, it is bincode-serialized. The `to_bytes()` method produces bincode format; `to_bech32m_string()` produces the user-facing string.

### Conversion to Core Addresses

`PlatformAddress` can be converted to and from `dashcore::Address`, the type used by the Core chain RPC client. This allows the platform layer to interoperate with Core's address format when needed -- for example, when processing withdrawals that ultimately create Core chain transactions.

## OrchardAddress

The shielded address type is defined in `packages/rs-dpp/src/address_funds/orchard_address.rs`:

```rust
pub struct OrchardAddress(grovedb_commitment_tree::PaymentAddress);
```

It wraps the Orchard protocol's native `PaymentAddress`, which consists of two components:

| Component | Size | Purpose |
|-----------|------|---------|
| Diversifier | 11 bytes | Entropy for deriving unlinkable addresses from a single spending key |
| pk_d | 32 bytes | Diversified transmission key (Pallas curve point) |

Total: **43 bytes**. The Bech32m payload is `0x10 || diversifier || pk_d` -- 44 bytes.

### Key Constants

```rust
const ORCHARD_DIVERSIFIER_SIZE: usize = 11;
const ORCHARD_PKD_SIZE: usize = 32;
const ORCHARD_ADDRESS_SIZE: usize = 43;
const ORCHARD_TYPE: u8 = 0x10;
```

### Diversifiers and Unlinkability

A single Orchard spending key can derive an unlimited number of addresses by varying the diversifier. Each address looks completely unrelated to any other address derived from the same key. Only the holder of the corresponding **Incoming Viewing Key (IVK)** can link them.

This is the fundamental privacy property: a merchant can give every customer a unique address, and no observer can determine that all those addresses belong to the same wallet. The diversifier is the mechanism that makes this possible.

### Differences from Zcash Encoding

Dash's OrchardAddress uses the same 43-byte raw format as Zcash Orchard addresses (identical diversifier + pk_d structure). But the Bech32m encoding is Dash-specific:

- **No F4Jumble**: Zcash applies an F4Jumble permutation before encoding; Dash does not.
- **No Unified Address wrapper**: Zcash wraps Orchard addresses in a Unified Address (UA) envelope with typecodes and length prefixes; Dash uses a simple type-byte scheme.
- **Different HRP**: Zcash uses `u1`; Dash uses `dash`/`tdash`.

The result is simpler, shorter addresses that are not interoperable with Zcash wallets.

### Converting to Orchard's Native Type

The builder functions that construct shielded transactions need the Orchard library's native `PaymentAddress`, not our wrapper. Conversion is straightforward:

```rust
impl From<&OrchardAddress> for PaymentAddress {
    fn from(address: &OrchardAddress) -> Self {
        *address.inner()
    }
}
```

This is used in the shielded builder module (`packages/rs-dpp/src/shielded/builder/`) when adding outputs to an Orchard bundle.

## Address Witnesses

When a transparent address is used as an input (for example, funding a shield transition), the sender must prove ownership. This is done through an `AddressWitness`, defined in `packages/rs-dpp/src/address_funds/witness.rs`:

```rust
pub enum AddressWitness {
    P2pkh {
        signature: BinaryData,
    },
    P2sh {
        signatures: Vec<BinaryData>,
        redeem_script: BinaryData,
    },
}
```

### P2PKH Witnesses

A P2PKH witness contains a single 65-byte recoverable ECDSA signature. The public key is *recovered* from the signature rather than transmitted alongside it -- saving 33 bytes per witness. Verification recovers the public key, hashes it with `Hash160`, and checks the result against the address hash.

### P2SH Witnesses

A P2SH witness contains the signatures and the redeem script. Only standard bare multisig scripts are supported:

```
OP_M <pubkey1> <pubkey2> ... <pubkeyN> OP_N OP_CHECKMULTISIG
```

Constraints:
- Maximum 17 signature entries (`MAX_P2SH_SIGNATURES`) -- 16 keys plus 1 dummy for the CHECKMULTISIG bug
- Only compressed public keys (33 bytes)
- No timelocks, hash puzzles, or custom scripts
- The redeem script must hash to the address: `Hash160(script) == address_hash`

### Verification Cost Tracking

Every witness verification has a measurable cost. The `AddressWitnessVerificationOperations` struct tracks the work:

```rust
pub struct AddressWitnessVerificationOperations {
    pub ecdsa_signature_verifications: u16,
    pub message_hash_count: u16,
    pub pubkey_hash_verifications: u16,
    pub script_hash_verifications: u16,
    pub signable_bytes_len: usize,
}
```

These counts feed into the fee system. A P2PKH witness costs one ECDSA verification. An M-of-N multisig costs N verifications (all public keys are checked against all signatures, per Bitcoin's CHECKMULTISIG semantics).

## How Addresses Are Used in Shielded Transitions

Each of the five shielded transition types uses addresses differently:

### Shield (Transparent to Shielded)

The sender provides one or more `PlatformAddress` inputs, each with a nonce and a maximum contribution amount. An `AddressWitness` proves ownership of each input. The funds enter the shielded pool and are received by an `OrchardAddress` embedded in the Orchard bundle's encrypted outputs.

```rust
pub struct ShieldTransitionV0 {
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    pub input_witnesses: Vec<AddressWitness>,
    pub actions: Vec<SerializedAction>,  // contains encrypted OrchardAddress recipient
    pub amount: u64,
    // ...
}
```

### Shielded Transfer (Shielded to Shielded)

No transparent addresses are involved. The sender spends notes from the shielded pool and creates new notes for the recipient's `OrchardAddress` and a change `OrchardAddress`. Both addresses are hidden inside the Orchard bundle -- only the respective recipients can decrypt them.

```rust
pub struct ShieldedTransferTransitionV0 {
    pub actions: Vec<SerializedAction>,  // spend + output pairs
    pub value_balance: u64,             // fee extracted from pool
    // ...
}
```

### Unshield (Shielded to Transparent)

The sender spends shielded notes and sends the funds to a `PlatformAddress` visible on-chain. The transparent output address is explicitly included in the transition and bound into the Orchard sighash to prevent substitution.

```rust
pub struct UnshieldTransitionV0 {
    pub output_address: PlatformAddress,  // visible transparent recipient
    pub unshielding_amount: u64,
    pub actions: Vec<SerializedAction>,   // spend + change output
    // ...
}
```

### Shielded Withdrawal (Shielded to Core Chain)

Similar to unshield, but the destination is a Core chain script rather than a platform address. The `CoreScript` holds a raw P2PKH or P2SH script that will be used in a Core chain withdrawal transaction.

```rust
pub struct ShieldedWithdrawalTransitionV0 {
    pub output_script: CoreScript,  // Core chain P2PKH or P2SH
    pub core_fee_per_byte: u32,
    pub pooling: Pooling,
    // ...
}
```

### Shield From Asset Lock (Core Chain to Shielded)

An asset lock proof from the Core chain funds the shielded pool directly. The recipient is an `OrchardAddress` inside the Orchard bundle. No `PlatformAddress` inputs are needed -- the asset lock proof substitutes for them.

The transition also carries an optional `surplus_output: Option<PlatformAddress>`. When the consumed asset lock exceeds `shield_amount + pool_fee`, the leftover *surplus* is credited to this platform address; if it is unset, the surplus folds into the fee pools (bounded by `shielded_implicit_fee_cap`). See [Entry-Transition Fees](../fees/shielded-fees.md#entry-transition-fees-shield-and-shieldfromassetlock). Unlike the transparent recipients of `Unshield`/`ShieldedWithdrawal` (which are bound through the Orchard sighash `extra_data` above), `surplus_output` is bound through the state transition's **own** `platform_signable` signature -- it sits before the `signature` field, so it is part of the signed payload and cannot be substituted or truncated after signing. The Orchard `extra_data` therefore remains empty for this transition.

## The Platform Sighash

When transparent fields need to be bound to an Orchard bundle's proof, the platform uses a custom sighash computation defined in `packages/rs-dpp/src/shielded/mod.rs`:

```rust
const SIGHASH_DOMAIN: &[u8] = b"DashPlatformSighash";

pub fn compute_platform_sighash(
    bundle_commitment: &[u8; 32],
    extra_data: &[u8],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SIGHASH_DOMAIN);
    hasher.update(bundle_commitment);
    hasher.update(extra_data);
    hasher.finalize().into()
}
```

The `bundle_commitment` is a BLAKE2b-256 hash of the Orchard bundle (per ZIP-244). The `extra_data` binds transparent fields:

| Transition | extra_data |
|-----------|------------|
| Shield | empty |
| Shielded Transfer | empty |
| Unshield | `output_address.to_bytes() \|\| amount.to_le_bytes()` |
| Shielded Withdrawal | `output_script.as_bytes()` |
| Shield From Asset Lock | empty |

This binding is critical for security. Without it, an attacker who intercepts an unshield transition could substitute the `output_address` while reusing the valid Orchard proof and signatures. The sighash ensures the Orchard bundle's spend authorization signatures commit to the specific transparent recipient.

The same `compute_platform_sighash` function is used on both sides: the client uses it when signing the bundle, and the platform uses it when verifying.

## Trial Decryption and Address Privacy

A shielded recipient does not appear anywhere in cleartext on-chain. To discover incoming payments, a wallet must attempt **trial decryption** of every new note using its Incoming Viewing Key (IVK).

The process works as follows:

1. The wallet retrieves new note entries from the shielded pool (each entry contains a `nullifier`, `cmx`, and 216-byte `encrypted_note`).
2. For each entry, the wallet constructs a `CompactAction` from the nullifier, commitment, ephemeral public key, and encrypted ciphertext.
3. The wallet attempts trial decryption using `try_note_decryption` with its IVK.
4. If decryption succeeds, the note was addressed to one of the wallet's `OrchardAddress` instances.

The nullifier stored alongside each note is essential -- it provides the `Rho` value needed for decryption. Without it, the Orchard protocol's forward secrecy mechanism would prevent the recipient from recovering the note.

Because a single spending key can generate unlimited `OrchardAddress` instances (via different diversifiers), the IVK-based scan catches all of them in a single pass. The diversifier that was used becomes apparent only after successful decryption.

## Note Encryption Structure

Each `SerializedAction` contains an `encrypted_note` field of 216 bytes:

| Component | Size | Purpose |
|-----------|------|---------|
| epk | 32 bytes | Ephemeral public key for Diffie-Hellman key agreement |
| enc_ciphertext | 104 bytes | Note plaintext encrypted to recipient (ChaCha20-Poly1305) |
| out_ciphertext | 80 bytes | Note encrypted to sender for wallet recovery |

The enc_ciphertext contains the note plaintext (52 bytes), a Dash-specific memo (36 bytes), and the AEAD authentication tag (16 bytes). The memo is smaller than Zcash's 512-byte memos -- Dash uses the `DashMemo` type (36 bytes) rather than `ZcashMemo` (512 bytes), keeping encrypted notes compact.

## Storage in GroveDB

Notes are stored in a BulkAppendTree within the shielded credit pool:

```
AddressBalances / "s" (shielded_credit_pool) /
    [1] notes       -- CommitmentTree (BulkAppendTree)
    [2] nullifiers   -- Tree (spent note markers)
    [5] total_balance -- SumItem
    [6] anchors      -- Tree (block_height -> anchor)
```

Each note entry stores:
```
cmx (32 bytes) || nullifier (32 bytes) || encrypted_note (216 bytes) = 280 bytes
```

The nullifier is stored alongside the note (rather than separately) specifically to support trial decryption. A scanning client needs both the encrypted ciphertext and the nullifier to attempt decryption.

## Rules and Guidelines

**Do:**
- Use `to_bech32m_string()` for user-facing address display. Always include the network parameter.
- Use `to_bytes()` (bincode format) for storage keys and wire serialization.
- Generate a fresh diversifier for each new Orchard payment address to maximize unlinkability.
- Always bind transparent fields into the platform sighash. Forgetting to include the output address in an unshield transition would be a critical vulnerability.

**Do not:**
- Mix up Bech32m type bytes (`0xb0`, `0x80`, `0x10`) with bincode variant bytes (`0x00`, `0x01`). They serve different purposes and are not interchangeable.
- Assume Orchard addresses are interoperable with Zcash. The encoding differs (no F4Jumble, no Unified Address wrapper).
- Use non-standard scripts in P2SH addresses. Only bare multisig (`OP_M ... OP_N OP_CHECKMULTISIG`) is supported.
- Store or transmit the raw spending key. Use the Incoming Viewing Key for scanning and the Full Viewing Key for read-only wallet recovery.
- Attempt trial decryption without the nullifier. The Orchard protocol requires the `Rho` derived from the nullifier to reconstruct the note.
