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
}
