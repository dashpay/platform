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

    // ── DIP-15 auto-accept QR ─────────────────────────────────────────

    /**
     * Build the owner's DIP-15 auto-accept QR payload
     * (`dash:?du=…&dapk=…`) for [identityId], keying the proof through
     * [coreSignerHandle]. [username] is the owner's DPNS name and is
     * **required** — the FFI rejects a null string; pass `""` for a nameless
     * identity (Rust resolves the name on-chain or errors clearly).
     * ← Swift `ManagedPlatformWallet.buildAutoAcceptQR`.
     */
    external fun buildAutoAcceptQr(
        walletHandle: Long,
        identityId: ByteArray,
        username: String,
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

    // ── DIP-13 invitations ────────────────────────────────────────────
    //
    // The invitation [uri] / returned link IS a bearer credential (the
    // one-time voucher private key rides inside as a WIF). It must never
    // be logged, persisted, or interpolated into an exception message on
    // either side of this boundary.

    /**
     * Rust-enforced cap on an invitation voucher's amount, in duffs.
     * ← Swift `ManagedPlatformWallet.maxInvitationDuffs`.
     */
    external fun invitationMaxDuffs(): Long

    /**
     * Rust-enforced floor on an invitation voucher's amount, in duffs — a
     * smaller voucher can fund neither a claim nor a reclaim.
     * ← Swift `ManagedPlatformWallet.minInvitationDuffs`.
     */
    external fun invitationMinDuffs(): Long

    /**
     * Decode a `dashpay://invite` link into a read-only preview — no
     * wallet, no network, no side effects. Returns a JSON object:
     * `structurallyValid`, `isInstant`, `hasInviter`, `inviterUsername`
     * (string or null), `amountDuffs` / `expiryUnix` (always 0 — the
     * legacy link carries neither; the amount resolves at claim). A
     * malformed link yields `structurallyValid: false`, not an exception.
     * Gate contact features on a non-null `inviterUsername`, not on
     * `hasInviter` (a metadata-only link sets the flag without a name).
     * ← Swift `ManagedPlatformWallet.parseInvitation`.
     */
    external fun parseInvitation(uri: String): String?

    /**
     * Create a DashPay invitation voucher and return the shareable
     * `dashpay://invite` link. Blocking (builds + broadcasts an L1 asset
     * lock and waits for its InstantSend proof). The invitation row lands
     * in Room via [NativePersistenceBridge.onPersistInvitationUpsert]
     * before this returns. [inviterIdentityId] null ⇒ pure funding
     * voucher; non-null (32 bytes) opts into the contact-bootstrap and
     * requires a non-null [inviterUsername].
     * ← Swift `ManagedPlatformWallet.createInvitation`.
     */
    external fun createInvitation(
        walletHandle: Long,
        amountDuffs: Long,
        fundingAccountIndex: Int,
        inviterIdentityId: ByteArray?,
        inviterUsername: String?,
        nowUnix: Long,
        coreSignerHandle: Long,
    ): String?

    /**
     * Claim a `dashpay://invite` link: register a NEW identity for the
     * invitee funded by the imported voucher. Blocking (refetches the
     * funding tx by txid, then waits for the Platform response).
     * [pubkeysBlob] is the same rich key-row layout
     * [IdentityNative.registerIdentityWithFunding] takes; no core signer
     * (the asset-lock signature uses the link's raw voucher key).
     * ← Swift `ManagedPlatformWallet.claimInvitation`.
     */
    external fun claimInvitation(
        walletHandle: Long,
        uri: String,
        identityIndex: Int,
        pubkeysBlob: ByteArray,
        signerHandle: Long,
        nowUnix: Long,
    ): IdentityRegistrationNativeResult

    // ── DashPay private contact requests (protocol version 15) ────────
    //
    // ← Swift `PrivateContactRequests.swift`
    // (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift).

    /**
     * Read DashPay's private contact request store for every identity of
     * the wallet, at most [maxPages] pages, and ingest what it holds for
     * them. The store is opened with keys derived inside
     * [coreSignerHandle] (the manager's `MnemonicResolverHandle`). Ingested
     * requests reach Room through the persister; no handles cross. Returns
     * six longs: requests ingested, pages read, entries refused, identities
     * whose ingest did not reach disk, the store position the pass stopped
     * at, and 1 when it read to the store's last entry (else 0). Blocking.
     * ← Swift `ManagedPlatformWallet.syncPrivateContactRequests(maxPages:)`.
     */
    external fun syncPrivateContactRequests(
        walletHandle: Long,
        coreSignerHandle: Long,
        maxPages: Int,
    ): LongArray?

    /**
     * Send a DashPay private contact request, paid from [account]'s
     * shielded notes on [walletId]. [resolverHandle] supplies the Orchard
     * spend authority and the DashPay keys; [signerHandle] is the identity
     * signer [TokensNative.sendContactRequest] takes. [shieldedAddress]
     * must be 43 raw Orchard bytes when set. Needs the `shielded` native
     * feature. Returns the created `ContactRequest` handle (destroy via
     * [TokensNative.contactRequestDestroy]).
     * ← Swift `PlatformWalletManager.sendPrivateContactRequest`.
     */
    @Suppress("LongParameterList")
    external fun sendPrivateContactRequest(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        senderIdentityId: ByteArray,
        recipientIdentityId: ByteArray,
        accountLabel: String?,
        autoAcceptProof: ByteArray?,
        shieldedAddress: ByteArray?,
        signerHandle: Long,
    ): Long

    /**
     * Accept the incoming contact request [requestHandle] with a private
     * reply, paid from [account]'s shielded notes. Handles as for
     * [sendPrivateContactRequest]. Returns the `EstablishedContact` handle
     * (destroy via [TokensNative.establishedContactDestroy]).
     * ← Swift `PlatformWalletManager.acceptPrivateContactRequest`.
     */
    @Suppress("LongParameterList")
    external fun acceptPrivateContactRequest(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        requestHandle: Long,
        shieldedAddress: ByteArray?,
        signerHandle: Long,
    ): Long

    /**
     * What a managed identity's private contact requests with [contactId]
     * carry: a JSON object (`receivedPrivately`, `sentPrivately`,
     * `shieldedAddress` lower hex or null), or null when neither tracked
     * request between them came or went privately.
     * ← Swift `ManagedIdentity.privateContact(contactId:)`.
     */
    external fun managedIdentityPrivateContact(identityHandle: Long, contactId: ByteArray): String?

    /**
     * Whether a private contact request from the managed identity to
     * [contactId] is being paid for, or was paid for and is not yet read
     * back from the store. A pure read; in memory only on the Rust side.
     * ← Swift `ManagedIdentity.privateReplyPending(contactId:)`.
     */
    external fun managedIdentityPrivateReplyPending(identityHandle: Long, contactId: ByteArray): Boolean

    /**
     * What a contact request from [identityId], one of the wallet's
     * identities, waits for, as the wallet decides it: three flags,
     * `[privatePaymentsRestored, privateStoreRead, unresolvedPrivatePayment]`
     * (`platform_wallet_get_private_contact_readiness`). An unknown wallet
     * handle throws (NotFound), as does an identity the wallet does not
     * hold.
     * ← Swift `ManagedPlatformWallet.privateContactReadiness(identityId:)`.
     */
    external fun privateContactReadiness(walletHandle: Long, identityId: ByteArray): BooleanArray?

    /**
     * Accept the incoming contact request [requestHandle] replying the way
     * it came: privately, paid from [account]'s shielded notes, when it came
     * privately and no reply exists yet; otherwise with a document, or by
     * adopting the reply already sent. The wallet decides. Arguments and
     * result as for [acceptPrivateContactRequest].
     * ← Swift `PlatformWalletManager.acceptContactRequestReplyingInKind`.
     */
    @Suppress("LongParameterList")
    external fun acceptContactRequestReplyingInKind(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        requestHandle: Long,
        shieldedAddress: ByteArray?,
        signerHandle: Long,
    ): Long
}
