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
        val wasNewlyCreated = try {
            runBlocking {
                walletStorage.storeIfAbsent(pubkeyHex, ownerWalletId = walletId) {
                    // Single Rust FFI call — the whole derivation,
                    // deadlock-safe — runs OUTSIDE WalletStorage's
                    // private-key lock (storeIfAbsent's own contract).
                    IdentityNative.deriveIdentityPrivateKeyWithResolver(
                        networkOrd = network.ffiValue,
                        walletId = walletId,
                        resolverHandle = mnemonicResolverHandle,
                        identityIndex = identityIndex,
                        keyIndex = keyIndex,
                    ).also { scalar = it }
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

    override fun deleteStored(pubkeyHexes: Collection<String>) {
        // Rollback counterpart of the store above — a failed changeset
        // round deletes the aliases it CREATED (their rows never commit).
        // Same nested-runBlocking rationale as deriveAndStore; the batch
        // delete is one atomic DataStore edit and throws on failure, per
        // the interface contract (the handler keeps the cleanup record
        // alive until a deletion succeeds).
        runBlocking { walletStorage.deletePrivateKeys(pubkeyHexes) }
    }

    private companion object {
        /** Matches `WalletStorage`'s private `PRIVKEY_PREFIX`. */
        const val PRIVKEY_IDENTIFIER_PREFIX = "privkey."
    }
}
