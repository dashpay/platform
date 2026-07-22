package org.dashfoundation.dashsdk.security

import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.ffi.IdentityNative
import org.dashfoundation.dashsdk.persistence.DerivedKeyStoreResult
import org.dashfoundation.dashsdk.persistence.PrivateKeyDeriver
import org.dashfoundation.dashsdk.persistence.toHex

/**
 * Production [PrivateKeyDeriver]: derives an identity key's 32-byte
 * private scalar via Rust and encrypts it into [WalletStorage].
 *
 * This is the Android realization of the `packages/kotlin-sdk/CLAUDE.md`
 * "one allowed exception" — Rust performs the whole `mnemonic → seed →
 * path → key` derivation behind a single FFI call; Kotlin only encrypts
 * the returned bytes into Keystore-backed storage and never derives.
 *
 * ## Why the resolver-keyed FFI
 *
 * The identity-key persist callback fires synchronously from inside a
 * platform-wallet operation that holds the wallet-manager **write** lock
 * (`registration.rs` persists the identity changeset under
 * `wallet_manager.write().await`). A wallet-handle-keyed derive would
 * re-`blocking_read` that same RwLock in its capability check and
 * **deadlock**. This deriver uses
 * [IdentityNative.deriveIdentityPrivateKeyWithResolver], a pure derive
 * (`resolver → mnemonic → master → key`) that never touches the
 * wallet-manager registry, so it is safe from the callback. The mnemonic
 * is resolved on the [mnemonicResolverHandle] keyed by `walletId`.
 *
 * Reference: `PlatformWalletPersistenceHandler.swift`'s
 * `deriveAndStoreIdentityKey` (Swift inlines the pipeline; we push it
 * down to Rust, per doctrine).
 *
 * @param network the manager's network (network-locked); its
 *   [Network.ffiValue] is the FFINetwork ordinal the FFI derive expects.
 * @param mnemonicResolverHandle the live `MnemonicResolverHandle`.
 * @param walletStorage where the encrypted scalar lands, keyed by
 *   public-key hex so [KeystoreSigner] can locate it at signing time.
 */
class IdentityKeyPrivateKeyDeriver(
    private val network: Network,
    private val mnemonicResolverHandle: Long,
    private val walletStorage: WalletStorage,
) : PrivateKeyDeriver {

    override fun deriveAndStore(
        walletId: ByteArray,
        publicKeyData: ByteArray,
        identityIndex: Int,
        keyIndex: Int,
        force: Boolean,
    ): DerivedKeyStoreResult {
        val pubkeyHex = publicKeyData.toHex()
        var scalar: ByteArray? = null
        // WalletStorage.storeIfAbsent is suspend; the callback thread
        // already runs everything under runBlocking on the persistence
        // dispatcher, so a nested runBlocking here is consistent with the
        // handler's own pattern (Room DAOs, resolver reads). It checks for
        // an existing (decryptable) entry and records ownerWalletId's
        // ownership atomically with the store, so it derives via Rust
        // (the single FFI call below) ONLY when the alias doesn't already
        // have a usable stored scalar under ANY owner — a re-derive of an
        // existing key would just reproduce the same deterministic bytes.
        //
        // force = true (the repair path, dashpay/platform#4060 finding 6)
        // routes through WalletStorage.replacePrivateKey instead: the
        // usability short-circuit is exactly what must NOT run when a
        // shape+fingerprint-valid but undecryptable blob is being repaired.
        val deriveScalarOnly: suspend () -> ByteArray = {
            // Single Rust FFI call — the whole derivation, deadlock-safe —
            // runs OUTSIDE WalletStorage's private-key lock (the
            // storeIfAbsent/replacePrivateKey contract).
            IdentityNative.deriveIdentityPrivateKeyWithResolver(
                networkOrd = network.ffiValue,
                walletId = walletId,
                resolverHandle = mnemonicResolverHandle,
                identityIndex = identityIndex,
                keyIndex = keyIndex,
            ).also { scalar = it }
        }
        // force = true is the repair path (dashpay/platform#4060 blocker 1):
        // it must PROVE the derived key matches [publicKeyData] before any
        // persistence. The caller only knows an (identityIndex, keyIndex) —
        // a WRONG pair derives a DIFFERENT valid scalar that round-trips
        // through encrypt/decrypt perfectly (probeIdentityKeyRecoverability
        // only proves the blob decrypts, not that it is the RIGHT key), so
        // without this check a mis-indexed repair would clear the pending
        // state and persist an unusable key. Derive the KEYPAIR, compare its
        // public half to [publicKeyData], and THROW before storing on
        // mismatch — replacePrivateKey never runs, so nothing is persisted.
        val deriveVerifiedForRepair: suspend () -> ByteArray = {
            val pair = IdentityNative.deriveIdentityKeyPairWithResolver(
                networkOrd = network.ffiValue,
                walletId = walletId,
                resolverHandle = mnemonicResolverHandle,
                identityIndex = identityIndex,
                keyIndex = keyIndex,
            )
            check(pair.size == 2) { "keypair derive returned ${pair.size} elements" }
            val derivedPrivate = pair[0]
            val derivedPublic = pair[1]
            scalar = derivedPrivate
            if (!derivedPublicKeyMatches(derivedPublic, publicKeyData)) {
                // Scrub the wrong scalar immediately; the finally scrubs
                // again harmlessly (idempotent zero-fill).
                derivedPrivate.fill(0)
                throw IdentityKeyDerivationMismatchException(
                    "identity-key repair derived a key whose public half does " +
                        "not match the requested pubkey $pubkeyHex at slot " +
                        "$identityIndex/$keyIndex — refusing to persist an " +
                        "unusable key (the derivation breadcrumbs are wrong " +
                        "or corrupt); pending state left intact",
                )
            }
            derivedPrivate
        }
        val wasNewlyCreated = try {
            runBlocking {
                if (force) {
                    walletStorage.replacePrivateKey(
                        pubkeyHex,
                        ownerWalletId = walletId,
                        derive = deriveVerifiedForRepair,
                    )
                    true
                } else {
                    walletStorage.storeIfAbsent(
                        pubkeyHex,
                        ownerWalletId = walletId,
                        derive = deriveScalarOnly,
                    )
                }
            }
        } finally {
            // Scrub the only copy that escaped Rust, if a derive happened.
            scalar?.fill(0)
        }
        // Recorded identifier on the persisted row — mirrors the
        // `WalletStorage` privkey storage-key prefix (`"privkey."`) and
        // Swift's keychain account string, so the explorer/signer can
        // reason about it uniformly.
        return DerivedKeyStoreResult(PRIVKEY_IDENTIFIER_PREFIX + pubkeyHex, wasNewlyCreated)
    }

    override fun deleteUnownedStored(
        pubkeyHexes: Collection<String>,
        excludingWalletId: ByteArray,
    ): Set<String> =
        // Rollback counterpart of the store above — a failed changeset
        // round deletes the aliases it CREATED (their rows never commit).
        // Same nested-runBlocking rationale as deriveAndStore; the
        // ownership check and the delete run under WalletStorage's own
        // single lock hold (see deleteUnownedPrivateKeys), so a sibling
        // wallet's concurrent storeIfAbsent can't adopt one of these
        // aliases in a window between a separate check and this delete.
        // Throws on an atomicity failure, per the interface contract (the
        // handler keeps the cleanup record alive until a deletion
        // succeeds).
        runBlocking { walletStorage.deleteUnownedPrivateKeys(pubkeyHexes, excludingWalletId) }

    internal companion object {
        /** Matches `WalletStorage`'s private `PRIVKEY_PREFIX`. */
        const val PRIVKEY_IDENTIFIER_PREFIX = "privkey."

        /**
         * Whether a freshly derived public key [derived] is byte-for-byte the
         * key [expected] a repair was asked to restore. Pure and side-effect
         * free so the repair's before-persistence identity check
         * (dashpay/platform#4060 blocker 1) is unit-testable without the
         * native derive. Both halves are the compressed public-key bytes Rust
         * emits (identical encoding on both derive entry points), so a plain
         * content comparison is the whole check.
         */
        fun derivedPublicKeyMatches(derived: ByteArray, expected: ByteArray): Boolean =
            derived.contentEquals(expected)
    }
}

/**
 * The repair path derived a key whose PUBLIC half does not match the pubkey
 * it was asked to restore — the requested (identityIndex, keyIndex) do not
 * belong to [publicKeyData]. Thrown BEFORE any Keystore write so a
 * mis-indexed repair persists nothing and never clears the pending-repair
 * state (dashpay/platform#4060 blocker 1).
 */
class IdentityKeyDerivationMismatchException(message: String) : RuntimeException(message)
