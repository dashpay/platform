package org.dashfoundation.dashsdk.ffi

/**
 * JNI bindings for the DashPay read surface added by the iOS DashPay
 * completion (upstream #3841) — payment history, cached contact
 * profiles, per-identity sync state, wallet-scoped DPNS search and
 * per-account wallet balances. Rust side:
 * `rs-unified-sdk-jni/src/dashpay.rs`. The pre-existing DashPay
 * send/accept/ignore/sync pipeline lives in [TokensNative].
 *
 * Read results are compact JSON strings (the
 * [TokensNative.getDashPayProfile] precedent — parsing happens
 * Kotlin-side); 32-byte ids inside the JSON are lower-hex. Errors throw
 * [DashSDKException].
 */
internal object DashpayNative {

    /**
     * DashPay payment history off a managed-identity handle. Returns a
     * JSON array — per payment: `txid`, `counterpartyId` (hex),
     * `amountDuffs`, `direction` (0 Sent, 1 Received), `status`
     * (0 Pending, 1 Confirmed, 2 Failed), optional `memo`.
     * ← Swift `ManagedIdentity.getDashPayPayments()`.
     */
    external fun managedIdentityDashPayPayments(identityHandle: Long): String?

    /**
     * Cached profile of [contactIdentityId] as seen by
     * [ownerIdentityId], or null when none is cached. Same JSON shape as
     * [TokensNative.getDashPayProfile].
     * ← Swift `ManagedPlatformWallet.getContactProfile(owner:contact:)`.
     */
    external fun getContactProfile(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    ): String?

    /**
     * The managed identity's own cached DashPay profile off its handle,
     * or null. ← Swift `ManagedIdentity.getDashPayProfile()`.
     */
    external fun managedIdentityDashPayProfile(identityHandle: Long): String?

    /**
     * DashPay sync state off a managed-identity handle: JSON object of
     * collection counts (`establishedContacts`, `incomingRequests`,
     * `sentRequests`, `ignoredSenders`, `contactProfiles`,
     * `presentContactProfiles`, `dashpayPayments`, `hasDashpayProfile`)
     * plus the optional `highWaterReceivedMs` / `highWaterSentMs`
     * cursors (keys absent when unset).
     * ← Swift `ManagedIdentity.getDashPaySyncState()`.
     */
    external fun managedIdentityDashPaySyncState(identityHandle: Long): String?

    /**
     * Live DPNS prefix search against Platform, wallet-scoped (the call
     * path iOS `AddContactView` drives — distinct from the
     * SDK-handle-scoped `QueriesNative.dpnsSearch`). Returns a JSON
     * array of `{"label":…,"identityId":…hex}`; `limit` 0 = no limit.
     * Blocking (network). ← Swift
     * `ManagedPlatformWallet.searchDpnsNames(prefix:limit:)`.
     */
    external fun searchDpnsNames(walletHandle: Long, prefix: String, limit: Int): String?

    /**
     * Per-account balance snapshot for [walletId] off the manager
     * handle. Returns a JSON array — per account: `typeTag`,
     * `standardTag`, `index`, `registrationIndex`, `keyClass`,
     * `userIdentityId` / `friendIdentityId` (hex, all-zero when unset),
     * `confirmed`, `unconfirmed`, `immature`, `locked`, `keysUsed`,
     * `keysTotal`. ← Swift `PlatformWalletManager.accountBalances(for:)`.
     */
    external fun walletManagerAccountBalances(managerHandle: Long, walletId: ByteArray): String?

    // ── Recurring DashPay sync service (manager-scoped) ───────────────

    /** Start the recurring DashPay sweep (idempotent Rust-side). */
    external fun dashPaySyncStart(managerHandle: Long)

    /** Stop the recurring sweep; leaves it restartable. */
    external fun dashPaySyncStop(managerHandle: Long)

    /** Whether the sweep loop is running. */
    external fun dashPaySyncIsRunning(managerHandle: Long): Boolean

    /** Whether a sweep pass is executing right now (the 1 Hz poll target). */
    external fun dashPaySyncIsSyncing(managerHandle: Long): Boolean

    /** Unix seconds of the last completed sweep; 0 when never. */
    external fun dashPaySyncLastSyncUnixSeconds(managerHandle: Long): Long

    /** Set the sweep interval in seconds (applies from the next tick). */
    external fun dashPaySyncSetInterval(managerHandle: Long, intervalSeconds: Long)

    /**
     * Run one sweep pass NOW, blocking until it completes. Returns
     * `{"success":…,"errors":…,"syncUnixSeconds":…}` (Swift
     * `DashPaySyncSummary`).
     */
    external fun dashPaySyncNow(managerHandle: Long): String?

    // ── Seedless unlock ───────────────────────────────────────────────

    /**
     * Verify the Keystore-resolved mnemonic reproduces this wallet's key
     * material. A stored-but-foreign seed throws with
     * `ErrorInvalidParameter` — callers disambiguate the seed-mismatch
     * case ONLY by scoping their catch to this call (the Swift contract).
     */
    external fun verifySeedBindsToWallet(walletHandle: Long, coreSignerHandle: Long)

    /** Deferred contact-crypto entries queued on the wallet (in-memory). */
    external fun pendingContactCryptoCount(walletHandle: Long): Int

    /**
     * Drain the deferred contact-crypto queue; returns the drained-entry
     * count. Blocking and potentially slow (network + ECDH per entry) —
     * IO thread only; keep the signer/resolver bridges strongly
     * reachable for the whole call. [signerHandle] may be 0 for
     * resolver-only drains.
     */
    external fun drainPendingContactCrypto(
        walletHandle: Long,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): Int

    // ── Profile / contactInfo writes ──────────────────────────────────

    /**
     * Create ([doCreate]) or update the DashPay profile, signing with
     * [signerHandle]. [avatarBytes] is the raw image — Rust computes the
     * SHA-256 hash + perceptual fingerprint. Broadcasts a real document
     * state transition (blocking, network). Returns the resulting
     * profile JSON. ← Swift `createDashPayProfile` / `updateDashPayProfile`.
     */
    @Suppress("LongParameterList")
    external fun createOrUpdateProfile(
        walletHandle: Long,
        identityId: ByteArray,
        displayName: String?,
        publicMessage: String?,
        avatarUrl: String?,
        avatarBytes: ByteArray?,
        doCreate: Boolean,
        signerHandle: Long,
    ): String?

    /**
     * Set owner-private contactInfo (alias / note / displayHidden) for
     * `(identityId, contactId)`. Local state always updates; the
     * encrypted on-chain publish is DIP-15-gated. Returns the outcome
     * discriminant: 0 published, 1 deferred until two contacts,
     * 2 skipped (watch-only). ← Swift `setDashPayContactInfo`.
     */
    @Suppress("LongParameterList")
    external fun setContactInfo(
        walletHandle: Long,
        identityId: ByteArray,
        contactId: ByteArray,
        alias: String?,
        note: String?,
        displayHidden: Boolean,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): Int

    /**
     * Whether the mnemonic resolver can derive-sign identity keys of
     * [keyType] — the preflight predicate keeping `canSignWith`
     * consistent with the sign path's UNSUPPORTED_KEY_TYPE rejection.
     * ← Swift `KeychainSigner.resolverCanDeriveSign`.
     */
    external fun resolverSupportsKeyType(keyType: Int): Boolean

    // ── DIP-15 auto-accept QR ─────────────────────────────────────────

    /**
     * Build the owner's DIP-15 auto-accept QR payload
     * (`dash:?du=…&dapk=…`) for [identityId], keying the proof through
     * [coreSignerHandle]. [username] is a display hint (nullable).
     * ← Swift `ManagedPlatformWallet.buildAutoAcceptQR`.
     */
    external fun buildAutoAcceptQr(
        walletHandle: Long,
        identityId: ByteArray,
        username: String?,
        coreSignerHandle: Long,
    ): String?

    /**
     * Scan-to-send: parse a DIP-15 auto-accept QR [uri] and send the
     * contact request it describes from [senderIdentityId] (the embedded
     * proof key lets the owner auto-accept). Blocking (network). Returns
     * the created `ContactRequest` handle — destroy via
     * [TokensNative.contactRequestDestroy].
     * ← Swift `ManagedPlatformWallet.sendContactRequestFromQR`.
     */
    external fun sendContactRequestFromQr(
        walletHandle: Long,
        senderIdentityId: ByteArray,
        uri: String,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): Long
}
