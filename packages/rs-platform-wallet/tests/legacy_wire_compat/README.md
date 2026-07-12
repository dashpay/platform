# Legacy txMetadata wire-compat vector generator

Two checked-in JVM tools back the hard-coded vectors in
`src/wallet/identity/crypto/tx_metadata.rs`
(`legacy_dashj_wire_compat_vector` and
`nonzero_identity_index_derivation_slot_is_internally_consistent`):

- **`LegacyKeyN.java`** — the reproducible key/blob *generator*. It runs
  dashj-core's cryptographic primitives — the same `HDKeyDerivation`,
  `KeyCrypterAESCBC.deriveKey/encrypt`, and `createTxMetadata` blob framing that
  dash-sdk-kotlin 4.0.0-RC2 used — but it **hand-builds the account path** rather
  than calling the real `DerivationPathFactory.blockchainIdentityECDSADerivationPath()`.
- **`LegacyDerivationPathCheck.java`** — the provenance *verifier*. It drives the
  REAL `org.bitcoinj.wallet.DerivationPathFactory` and confirms that
  `LegacyKeyN`'s hand-built account path equals the factory's output at
  identityIndex 0, so the wire-compat anchor is independently reproducible from
  checked-in code — not just asserted in prose (dashpay/platform#4091, findings
  989be307db0f / dd246b5e17d0 / 4c0754158cc6).

## What each vector proves (and what it does NOT)

- **`legacy_dashj_wire_compat_vector` (identity_index 0) — a genuine legacy
  wire-compat anchor.** The index-0 account path
  `m/9'/1'/5'/0'/0'/0'/keyId'/32769'/encryptionKeyIndex'` was independently
  confirmed to equal the output of the REAL dashj `DerivationPathFactory`
  (driven directly, with `32769'` read straight off `TxMetadataDocument`) — so
  the `4a2e…84d7` key is pinned against a path the legacy library itself chose,
  not one this repo constructed. **Run `LegacyDerivationPathCheck` (below) to
  reproduce that equality yourself**: it prints
  `WIRE_COMPAT_ANCHOR_OK = true` when the factory's primary-identity
  (`blockchainIdentityECDSADerivationPath()`, no-arg = `m/9'/1'/5'/0'/0'/0'`)
  path matches `LegacyKeyN`'s hand-built account path at identity_index 0. This
  is the sole point at which legacy wire-compat is defined: the legacy
  `createTxMetadata` flow has NO identity-index component (it always derives
  against the primary identity via the no-arg method), so identity_index 0 is
  the only slot a legacy wallet ever wrote.

- **`nonzero_identity_index_derivation_slot_is_internally_consistent`
  (identity_index 1) — a SELF-REFERENTIAL internal check, NOT a wire-compat
  claim.** `KeyDerivationType::ECDSA == 0` sits immediately before
  `identity_index'` in `base / key_type' / identity_index' / key_index' /
  32769' / encryption_key_index'`, so at index 0 the two adjacent `0'`
  components are indistinguishable. The `identity_index = 1` vector
  (`m/9'/1'/5'/0'/0'/1'/2'/32769'/1'`) derives a provably different key
  (`8cda…5196` vs `4a2e…84d7`), exercising that the component occupies its own
  slot. But because the generator hand-builds this path (the same one Rust's
  `tx_metadata_derivation_path` constructs), the value is a cross-check of
  Rust ⟷ dashj-core HD derivation for a path THIS repo picked — not evidence
  that any legacy platform code selects it. No legacy document is keyed at
  identity_index > 0.

## Reproduce

Classpath jars come from the Gradle module cache
(`~/.gradle/caches/modules-2/files-2.1`):

- `org.dashj/dashj-core/22.0.3/…/dashj-core-22.0.3.jar`
- `org.bouncycastle/bcprov-jdk18on/1.80/…/bcprov-jdk18on-1.80.jar`
- `com.google.guava/guava/30.0-jre/…/guava-30.0-jre.jar`
- `org.slf4j/slf4j-api/1.7.30/…/slf4j-api-1.7.30.jar`
- `de.sfuhrm/saphir-hash-core/3.0.10/…/saphir-hash-core-3.0.10.jar`
  (X11 genesis-block hashing; needed by `LegacyDerivationPathCheck`'s
  `TestNet3Params.get()`, not by `LegacyKeyN`)

```sh
CP="dashj-core-22.0.3.jar:bcprov-jdk18on-1.80.jar:guava-30.0-jre.jar:slf4j-api-1.7.30.jar:saphir-hash-core-3.0.10.jar"

# 1. Verify provenance: the hand-built path IS the real dashj factory path at
#    identity_index 0 (prints WIRE_COMPAT_ANCHOR_OK = true).
javac -cp "$CP" LegacyDerivationPathCheck.java
java -cp ".:$CP" LegacyDerivationPathCheck 0

# 2. Regenerate the key/blob vectors.
javac -cp "$CP" LegacyKeyN.java
# args: <identityIndex> <keyId(=keyIndex)> <encryptionKeyIndex>
java -cp ".:$CP" LegacyKeyN 0 2 1   # -> AES_KEY=4a2e…84d7  (index-0 vector)
java -cp ".:$CP" LegacyKeyN 1 2 1   # -> AES_KEY=8cda…5196  (index-1 vector)
```

`LegacyDerivationPathCheck` also prints the factory's INDEXED overload
`blockchainIdentityECDSADerivationPath(i)` = `m/9'/1'/5'/0'/0'/0'/i'` beside
`LegacyKeyN`'s hand-built nonzero path `m/9'/1'/5'/0'/0'/i'`, making the shape
difference visible: the nonzero `LegacyKeyN` vector is NOT a factory-produced
legacy sample, only the self-referential internal cross-check documented above.

`AES_KEY` is deterministic for a given `(identityIndex, keyId,
encryptionKeyIndex)`; `BLOB` embeds a fresh `SecureRandom` IV per run, so its
bytes differ each invocation while any produced blob still opens under the key
(`open_tx_metadata` reads the IV from the blob). Mnemonic: the BIP-39 test
vector `abandon abandon … about`, empty passphrase, Testnet.
