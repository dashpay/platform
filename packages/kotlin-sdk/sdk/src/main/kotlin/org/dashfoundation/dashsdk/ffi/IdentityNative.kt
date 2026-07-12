package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for identity registration, discovery, key preview, and
 * DPNS name registration — mirrors `rs-unified-sdk-jni/src/identity.rs`.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.identity.IdentityRegistration]. Handles are
 * raw Rust pointers as [Long] (wallet handle, `SignerHandle`,
 * `MnemonicResolverHandle`); passing a stale or foreign value is undefined
 * behavior, so ownership is confined to the SDK wrapper classes. Errors
 * throw [DashSDKException].
 *
 * Each function is a thin marshaler over a SINGLE `platform-wallet-ffi`
 * entry point — no orchestration crosses this boundary (see
 * `packages/kotlin-sdk/CLAUDE.md`).
 */
internal object IdentityNative {

    /**
     * Derive the first [count] identity-authentication keypairs the wallet
     * would probe during a discovery scan, starting at [startIndex]. A
     * pure-compute view — no Platform RPCs.
     *
     * Returns a flat BLOB decoded by
     * [org.dashfoundation.dashsdk.identity.IdentityKeyPreview.decodeAll]:
     * `u32 rowCount` then per row `u32 identityIndex, u16 pathLen,
     * pathUtf8, u8[33] pubkey, u8[32] privkey` (all big-endian). The Rust
     * preview buffer (incl. private material) is zeroized + freed before
     * this returns. [count] < 0 uses the Rust gap-limit default.
     *
     * @param resolverHandle `MnemonicResolverHandle`, needed for
     *   watch-only / external-signable wallets; ignored for resident-key
     *   wallets.
     */
    external fun previewRegistrationKeys(
        walletHandle: Long,
        resolverHandle: Long,
        startIndex: Int,
        count: Int,
    ): ByteArray

    /**
     * Derive the full identity-registration key **set** for a single
     * identity: keyId 0..[count] at the fixed [identityIndex]. Unlike
     * [previewRegistrationKeys] (which fixes the MASTER key slot and walks
     * the *identity* index for the discovery preview), this fixes the
     * identity index and walks the *key* index — so it returns every
     * keypair a freshly-created identity is built from.
     *
     * [count] < 0 derives the canonical default set (4 keys: MASTER auth,
     * CRITICAL auth, HIGH auth, TRANSFER/CRITICAL). Returns the same flat
     * BLOB layout as [previewRegistrationKeys], decoded by
     * [org.dashfoundation.dashsdk.identity.IdentityKeyPreview.decodeAll].
     * The per-key DPP role is applied positionally by keyId at
     * registration time — the row carries only the derived ECDSA keypair.
     *
     * @param resolverHandle `MnemonicResolverHandle`, needed for
     *   watch-only / external-signable wallets; ignored for resident-key
     *   wallets.
     */
    external fun previewRegistrationKeySet(
        walletHandle: Long,
        resolverHandle: Long,
        identityIndex: Int,
        count: Int,
    ): ByteArray

    /**
     * Derive the ready-to-persist 32-byte ECDSA private-key scalar for the
     * identity key at ([identityIndex], [keyIndex]) on the wallet behind
     * [walletHandle].
     *
     * The whole `mnemonic → seed → path → key` derivation runs on the Rust
     * side (the `packages/kotlin-sdk/CLAUDE.md` "one allowed exception"
     * shape): Kotlin receives bytes ready to encrypt into Keystore-backed
     * storage and never derives. The Rust-owned buffer is zeroized + freed
     * before this returns; the returned array is the only copy that
     * escapes, so callers should scrub it after storing.
     *
     * The derivation source (resident wallet vs. resolver-provided
     * mnemonic) is chosen by the wallet's capability. [resolverHandle] is
     * consulted only for watch-only / external-signable wallets and may be
     * `0` (null) otherwise. Network + path shape come from the wallet
     * handle — Kotlin decides nothing.
     *
     * @return the 32-byte private-key scalar.
     */
    external fun deriveIdentityPrivateKey(
        walletHandle: Long,
        resolverHandle: Long,
        identityIndex: Int,
        keyIndex: Int,
    ): ByteArray

    /**
     * Resolver-keyed sibling of [deriveIdentityPrivateKey] for the
     * **persistence-callback** path.
     *
     * The identity-key persistence callback fires synchronously from
     * inside a platform-wallet operation that holds the wallet-manager
     * write lock, so a derive that re-locks the manager (including
     * [deriveIdentityPrivateKey], whose capability check reads the
     * registry) would deadlock. This variant routes through a **pure**
     * Rust derive (`resolver → mnemonic → master → key`) that never
     * touches the wallet-manager registry, so it is safe to call from the
     * callback.
     *
     * The network + [walletId] are passed explicitly because the callback
     * has no wallet handle; the resolver resolves the mnemonic keyed by
     * [walletId].
     *
     * @param networkOrd FFINetwork ordinal: 0=Mainnet, 1=Testnet,
     *   2=Devnet, 3=Regtest.
     * @return the 32-byte private-key scalar.
     */
    external fun deriveIdentityPrivateKeyWithResolver(
        networkOrd: Int,
        walletId: ByteArray,
        resolverHandle: Long,
        identityIndex: Int,
        keyIndex: Int,
    ): ByteArray

    /**
     * Keypair variant of the resolver-keyed slot derive:
     * `[privateKey(32), publicKey]`. The public half is required by
     * IdentityUpdateTransition add-key rows.
     */
    external fun deriveIdentityKeyPairWithResolver(
        networkOrd: Int,
        walletId: ByteArray,
        resolverHandle: Long,
        identityIndex: Int,
        keyIndex: Int,
    ): Array<ByteArray>

    /**
     * Register a new identity funded from the wallet's Core balance. The
     * single FFI entry point the registration coordinator's body invokes.
     *
     * @param pubkeysBlob the keys to register, big-endian: `u32 rowCount`
     *   then per row `u32 keyId, u16 pubkeyLen, pubkey`. Built by
     *   [org.dashfoundation.dashsdk.identity.IdentityKeyPreview.encodeForRegistration].
     * @param signerHandle identity-key `SignerHandle`.
     * @param coreSignerHandle `MnemonicResolverHandle` for the asset-lock
     *   credit-spend signature.
     * @return the 32-byte identity id.
     */
    external fun registerIdentityWithFunding(
        walletHandle: Long,
        amountDuffs: Long,
        accountIndex: Int,
        identityIndex: Int,
        pubkeysBlob: ByteArray,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ByteArray

    /**
     * Register a new identity funded by the wallet's already-committed
     * Platform-payment (DIP-17) address balances — the ID-08 create path,
     * distinct from [registerIdentityWithFunding] (ID-01) which builds a
     * new Core asset lock.
     *
     * @param pubkeysBlob the keys to register (same layout as
     *   [registerIdentityWithFunding]). Built by
     *   [org.dashfoundation.dashsdk.identity.IdentityKeyPreview.encodeForRegistration].
     * @param signerHandle used for **both** the identity-key and the
     *   platform-address signing roles (the native `VTableSigner`
     *   dispatches by key-type byte).
     * @param inputsBlob the funding addresses, big-endian: `u32 rowCount`
     *   then per row `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash,
     *   u64 credits`. Built by
     *   [org.dashfoundation.dashsdk.credits.FundingInput.encode]. Nonces
     *   are auto-fetched Rust-side.
     * @return the 32-byte identity id.
     */
    external fun registerIdentityFromAddresses(
        walletHandle: Long,
        identityIndex: Int,
        pubkeysBlob: ByteArray,
        signerHandle: Long,
        inputsBlob: ByteArray,
    ): ByteArray

    /**
     * Scan the wallet's identity-authentication tree for registered
     * identities (gap-limit walk, Rust-side). Returns the concatenated
     * 32-byte ids (length is a multiple of 32). [startIndex] < 0 uses the
     * Rust default start.
     */
    external fun discoverIdentities(
        walletHandle: Long,
        resolverHandle: Long,
        startIndex: Int,
        gapLimit: Int,
    ): ByteArray

    /**
     * Register a DPNS name for [identityId] (32 bytes), signed via
     * [signerHandle]. Returns the full domain name (e.g. `"alice.dash"`).
     */
    external fun registerDpnsName(
        walletHandle: Long,
        identityId: ByteArray,
        label: String,
        signerHandle: Long,
    ): String

    /**
     * Create + broadcast a new data contract owned by [ownerIdentityId],
     * signed via [signerHandle]. Thin marshaler over
     * `platform_wallet_create_data_contract_with_signer` — the whole
     * build/validate/broadcast pipeline is in platform-wallet.
     *
     * [documentsSchemaJson] is required; the rest are optional (`null` or
     * empty ⇒ the section is omitted). Returns the 32-byte contract id.
     */
    external fun createDataContract(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        documentsSchemaJson: String,
        tokensSchemaJson: String?,
        groupsSchemaJson: String?,
        keywordsJson: String?,
        description: String?,
        configJson: String?,
        signerHandle: Long,
    ): ByteArray

    /**
     * Update + broadcast the existing data contract [contractId] (32 bytes)
     * owned by [ownerIdentityId], signed via [signerHandle]. Thin marshaler
     * over `platform_wallet_update_data_contract_with_signer` — the wallet
     * fetches the live contract, bumps its version, and *merges* the
     * supplied sections additively (omitted keys keep their on-chain
     * definition). Unlike the document ops there is **no** `signingKeyId` —
     * the wallet selects the key internally.
     *
     * [documentsSchemaJson] is required; the rest are optional (`null` or
     * empty ⇒ the section is omitted). Returns the 32-byte updated contract
     * id.
     */
    external fun updateDataContract(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        contractId: ByteArray,
        documentsSchemaJson: String,
        tokensSchemaJson: String?,
        groupsSchemaJson: String?,
        keywordsJson: String?,
        description: String?,
        configJson: String?,
        signerHandle: Long,
    ): ByteArray
}
