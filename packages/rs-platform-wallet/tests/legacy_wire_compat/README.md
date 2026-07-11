# Legacy txMetadata wire-compat vector generator

`LegacyKeyN.java` is the reproducible JVM generator behind the hard-coded
cross-stack vectors in
`src/wallet/identity/crypto/tx_metadata.rs`
(`legacy_dashj_wire_compat_vector` and
`legacy_dashj_wire_compat_vector_nonzero_identity_index`).

It runs the **actual legacy `org.dashj.platform` / dashj-core stack** — the same
`HDKeyDerivation`, `blockchainIdentityECDSADerivationPath` constants,
`KeyCrypterAESCBC.deriveKey/encrypt`, and `createTxMetadata` blob framing that
dash-sdk-kotlin 4.0.0-RC2 used — so the Rust `derive_tx_metadata_key` /
`seal_tx_metadata` implementations can be pinned against a value the Rust code
did not itself produce. This is what makes the wire-compat guarantee auditable
rather than self-referential (dashpay/platform#4091 review).

## Why a nonzero `identity_index` vector exists

The Rust path is `base / key_type' / identity_index' / key_index' / 32769' /
encryption_key_index'` and `KeyDerivationType::ECDSA == 0`. At
`identity_index = 0` the `key_type'` and `identity_index'` components are both
`0'` and adjacent, so an index-0-only vector cannot distinguish a correctly
placed `identity_index` from one that was dropped or swapped. The
`identity_index = 1` vector (`m/9'/1'/5'/0'/0'/1'/2'/32769'/1'`) derives a
provably different key (`8cda…5196` vs the index-0 `4a2e…84d7`), exercising that
component directly.

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
