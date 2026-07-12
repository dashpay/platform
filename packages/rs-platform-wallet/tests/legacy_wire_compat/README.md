# Legacy txMetadata wire-compat vector generator

`LegacyKeyN.java` is the reproducible JVM generator behind the hard-coded
vectors in
`src/wallet/identity/crypto/tx_metadata.rs`
(`legacy_dashj_wire_compat_vector` and
`nonzero_identity_index_derivation_slot_is_internally_consistent`).

It runs dashj-core's cryptographic primitives — the same `HDKeyDerivation`,
`KeyCrypterAESCBC.deriveKey/encrypt`, and `createTxMetadata` blob framing that
dash-sdk-kotlin 4.0.0-RC2 used — but it **hand-builds the account path** rather
than calling the real `DerivationPathFactory.blockchainIdentityECDSADerivationPath()`.

## What each vector proves (and what it does NOT)

- **`legacy_dashj_wire_compat_vector` (identity_index 0) — a genuine legacy
  wire-compat anchor.** The index-0 account path
  `m/9'/1'/5'/0'/0'/0'/keyId'/32769'/encryptionKeyIndex'` was independently
  confirmed to equal the output of the REAL dashj `DerivationPathFactory`
  (driven directly, with `32769'` read straight off
  `TxMetadataDocument`) — so the `4a2e…84d7` key is pinned against a path the
  legacy library itself chose, not one this repo constructed. This is the sole
  point at which legacy wire-compat is defined: the legacy `createTxMetadata`
  flow has NO identity-index component (it always derives against the primary
  identity), so identity_index 0 is the only slot a legacy wallet ever wrote.

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

```sh
CP="dashj-core-22.0.3.jar:bcprov-jdk18on-1.80.jar:guava-30.0-jre.jar:slf4j-api-1.7.30.jar"
javac -cp "$CP" LegacyKeyN.java

# args: <identityIndex> <keyId(=keyIndex)> <encryptionKeyIndex>
java -cp ".:$CP" LegacyKeyN 0 2 1   # -> AES_KEY=4a2e…84d7  (index-0 vector)
java -cp ".:$CP" LegacyKeyN 1 2 1   # -> AES_KEY=8cda…5196  (index-1 vector)
```

`AES_KEY` is deterministic for a given `(identityIndex, keyId,
encryptionKeyIndex)`; `BLOB` embeds a fresh `SecureRandom` IV per run, so its
bytes differ each invocation while any produced blob still opens under the key
(`open_tx_metadata` reads the IV from the blob). Mnemonic: the BIP-39 test
vector `abandon abandon … about`, empty passphrase, Testnet.
