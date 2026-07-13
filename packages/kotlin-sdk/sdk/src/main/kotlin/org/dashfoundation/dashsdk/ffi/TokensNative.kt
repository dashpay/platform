package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the platform-wallet token / DashPay / group-action
 * families — mirrors `rs-unified-sdk-jni/src/tokens.rs`.
 *
 * These bind the `platform_wallet_token_*`, `platform_wallet_*contact*`,
 * `platform_wallet_send_dashpay_payment`, `platform_wallet_*dashpay_profile*`
 * and group-query entry points of `platform-wallet-ffi` (the surface the iOS
 * `ManagedPlatformWallet` drives). Every action takes a **wallet handle**
 * (`platform_wallet_ffi::handle::Handle`, obtained from
 * [WalletManagerNative.getWallet]) plus a native `SignerHandle`
 * (from [SignerNative.createSigner]), both crossing as `Long`.
 *
 * 32-byte identifiers cross as `ByteArray`; group-action authorization is a
 * flattened `(kind, position, actionId?, actionIsProposer)` tuple. JSON
 * results are raw strings decoded by the public wrappers. Errors throw
 * [DashSDKException]; the public wrappers map them via `mapNativeErrors`.
 */
internal object TokensNative {

    // ── Token actions (12 forms) ─────────────────────────────────────

    /**
     * Mint [amount] of the token. [issuedToIdentityId] may be null (mint to
     * the transition owner). Group-gatable — pass the group tuple.
     * Returns post-mint balances JSON (keyed by the recipient) or null.
     */
    external fun tokenMint(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        issuedToIdentityId: ByteArray?,
        amount: Long,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    ): String?

    /** Burn [amount] from the caller. Returns post-burn balances JSON or null. */
    external fun tokenBurn(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: Long,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    ): String?

    /** Transfer [amount] to [recipientId]. Single-signer. Returns balances JSON or null. */
    external fun tokenTransfer(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        recipientId: ByteArray,
        amount: Long,
        publicNote: String?,
        signingKeyId: Int,
        signerHandle: Long,
    ): String?

    /** Freeze [frozenIdentityId]'s entire balance. Group-gatable. */
    external fun tokenFreeze(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        frozenIdentityId: ByteArray,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /** Unfreeze [frozenIdentityId]'s frozen balance. Group-gatable. */
    external fun tokenUnfreeze(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        frozenIdentityId: ByteArray,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /** Destroy [frozenIdentityId]'s frozen balance. Group-gatable. */
    external fun tokenDestroyFrozenFunds(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        frozenIdentityId: ByteArray,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /** Pause all token operations. Group-gatable. */
    external fun tokenPause(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /** Resume all token operations. Group-gatable. */
    external fun tokenResume(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /** Set the direct-purchase price ([pricePerToken] == 0 disables). Group-gatable. */
    external fun tokenSetPrice(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        pricePerToken: Long,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /** Purchase [amount] at the set price. Single-signer (never group-gated). */
    external fun tokenPurchase(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: Long,
        expectedTotalCost: Long,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /** Claim a distribution. [distributionType]: 0 = pre-programmed, 1 = perpetual. Single-signer. */
    external fun tokenClaim(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        distributionType: Int,
        publicNote: String?,
        signingKeyId: Int,
        signerHandle: Long,
    )

    /**
     * Update token configuration. [changeItemTag] selects the variant
     * (0 = MaxSupply, the only tag wired in this release);
     * [changeItemPayloadJson] is the per-tag JSON payload. Group-gatable.
     */
    external fun tokenUpdateConfig(
        walletHandle: Long,
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        changeItemTag: Int,
        changeItemPayloadJson: String,
        publicNote: String?,
        groupInfoKind: Int,
        groupInfoPosition: Int,
        groupInfoActionId: ByteArray?,
        groupInfoActionIsProposer: Boolean,
        signingKeyId: Int,
        signerHandle: Long,
    )

    // ── Group-action queries ─────────────────────────────────────────

    /**
     * List group-action proposals. [status]: 0 = active/pending, 1 = closed.
     * [startAtActionId] may be null; [limit] == 0 uses the FFI default.
     * Returns a JSON array string, or null.
     */
    external fun pendingGroupActions(
        walletHandle: Long,
        tokenContractId: ByteArray,
        groupContractPosition: Int,
        status: Int,
        startAtActionId: ByteArray?,
        limit: Int,
    ): String?

    /** List signers of a specific proposal. Returns a JSON array string, or null. */
    external fun groupActionSigners(
        walletHandle: Long,
        tokenContractId: ByteArray,
        groupContractPosition: Int,
        status: Int,
        actionId: ByteArray,
    ): String?

    // ── DashPay single calls ─────────────────────────────────────────

    /**
     * Send a contact request. [accountLabel] / [autoAcceptProof] may be null.
     * [coreSignerHandle] is the manager's `MnemonicResolverHandle` — the
     * friendship xpub / ECDH / DIP-15 accountReference derivations all
     * route through it Rust-side. Returns a native `ContactRequest` handle
     * (release via [contactRequestDestroy]).
     */
    @Suppress("LongParameterList")
    external fun sendContactRequest(
        walletHandle: Long,
        senderIdentityId: ByteArray,
        recipientIdentityId: ByteArray,
        accountLabel: String?,
        autoAcceptProof: ByteArray?,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): Long

    /**
     * Accept an incoming contact request (by its handle), keying the
     * reciprocal send + external-account registration through
     * [coreSignerHandle]. Returns a native `EstablishedContact` handle
     * (release via [establishedContactDestroy]).
     */
    external fun acceptContactRequest(
        walletHandle: Long,
        requestHandle: Long,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): Long

    /**
     * Ignore a contact sender (per-sender mute, = block, reversible,
     * local-only — replaces the removed per-request reject). Reverse via
     * [unignoreContactSender].
     */
    external fun ignoreContactSender(
        walletHandle: Long,
        ourIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    )

    /**
     * Un-ignore a contact sender (reverse [ignoreContactSender]); rewinds
     * the received cursor so their requests re-fetch on the next sweep.
     */
    external fun unignoreContactSender(
        walletHandle: Long,
        ourIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    )

    /** Release a `ContactRequest` handle from [sendContactRequest]. Safe on 0. */
    external fun contactRequestDestroy(handle: Long)

    /** Release an `EstablishedContact` handle from [acceptContactRequest]. Safe on 0. */
    external fun establishedContactDestroy(handle: Long)

    /**
     * Send a Dash payment ([amountDuffs] in duffs), signing the funding
     * inputs through [coreSignerHandle] (the manager's
     * `MnemonicResolverHandle`). Returns a 40-byte packed array —
     * `txid[32] || feeDuffs(u64 LE)`, the exact network fee (Σin − Σout) —
     * or null. `Dashpay.sendPayment` splits it into a typed result.
     */
    external fun sendDashPayPayment(
        walletHandle: Long,
        fromIdentityId: ByteArray,
        toContactIdentityId: ByteArray,
        amountDuffs: Long,
        memo: String?,
        coreSignerHandle: Long,
    ): ByteArray?

    /** Sync DashPay profiles for every managed identity. Returns the synced count. */
    external fun syncDashPayProfiles(walletHandle: Long): Int

    /**
     * Read the cached DashPay profile for [identityId]. Returns a JSON object
     * string (displayName, publicMessage, avatarUrl, avatarHash hex,
     * avatarFingerprint hex) or null when there is no cached profile.
     */
    external fun getDashPayProfile(walletHandle: Long, identityId: ByteArray): String?

    // ── DashPay contact-request enumeration (FriendsView hydration) ───

    /**
     * Sync incoming contact requests from Platform for every managed identity
     * on the wallet (applies them to in-memory state). Blocking.
     */
    external fun syncContactRequests(walletHandle: Long)

    /** Fetch the contact requests sent by [identityId] from Platform. Blocking. */
    external fun fetchSentContactRequests(walletHandle: Long, identityId: ByteArray)

    /**
     * Take a fresh `ManagedIdentity` snapshot handle for [identityId]. Release
     * via [managedIdentityDestroy]. Snapshot doesn't track later mutations —
     * re-fetch after a sync.
     */
    external fun getManagedIdentity(walletHandle: Long, identityId: ByteArray): Long

    /** Release a `ManagedIdentity` handle from [getManagedIdentity]. Safe on 0. */
    external fun managedIdentityDestroy(handle: Long)

    /**
     * The 32-byte sender ids of the managed identity's incoming contact
     * requests, as a flat blob (`u32 count` big-endian + `count × 32`).
     */
    external fun managedIdentityIncomingContactRequestIds(identityHandle: Long): ByteArray?

    /** The 32-byte recipient ids of the managed identity's sent requests, flat blob. */
    external fun managedIdentitySentContactRequestIds(identityHandle: Long): ByteArray?

    /** The 32-byte contact ids of the managed identity's established contacts, flat blob. */
    external fun managedIdentityEstablishedContactIds(identityHandle: Long): ByteArray?

    /**
     * The incoming `ContactRequest` handle for [senderId] (fed into
     * [acceptContactRequest]), or 0 when not in local state. Release via
     * [contactRequestDestroy].
     */
    external fun getIncomingContactRequest(identityHandle: Long, senderId: ByteArray): Long
}
