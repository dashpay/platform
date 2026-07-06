package org.dashfoundation.dashsdk.tokens

import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DashpayNative
import org.dashfoundation.dashsdk.ffi.TokensNative

/**
 * DashPay contact-request lifecycle + payments — the Android analog of the
 * Swift `ManagedPlatformWallet` DashPay calls driving `FriendsView`
 * (`packages/swift-sdk/.../PlatformWallet/ManagedPlatformWallet.swift`).
 *
 * Covers the single-call operations: send / accept a contact request,
 * ignore / un-ignore a sender (the reversible local mute that replaced the
 * old per-request reject), send a payment, and read / sync the cached
 * DashPay profile — plus the `FriendsView` hydration surface
 * ([syncContactRequests] / [fetchSentContactRequests] + [contacts] /
 * [acceptIncomingRequest]) built over the managed-identity contact-id
 * enumerators.
 *
 * Contact-request and established-contact handles are opaque native handles
 * (`Long`); [ContactRequestRef] / [EstablishedContactRef] wrap them as
 * [AutoCloseable] with the same ownership discipline as `DataContractRef`.
 *
 * [walletHandle] is a live `PlatformWallet` handle from
 * `WalletManagerNative.getWallet`; [signerHandle] is a native `SignerHandle`
 * from `SignerNative.createSigner`.
 */
class Dashpay internal constructor(private val walletHandle: Long) {

    /**
     * Send a contact request to [recipientIdentityId], signing the document
     * state-transition with [signerHandle] and keying the contact crypto
     * (friendship xpub, ECDH, DIP-15 accountReference) through
     * [coreSignerHandle] — the manager's `MnemonicResolverHandle`
     * (`PlatformWalletManager.mnemonicResolverHandle`), matching the Swift
     * wrapper's internally-pinned `MnemonicResolver`. [accountLabel]
     * (encrypted by the SDK) and [autoAcceptProof] are optional. Returns a
     * [ContactRequestRef] wrapping the created native handle — close it
     * (or `use {}`) when done.
     */
    suspend fun sendContactRequest(
        senderIdentityId: ByteArray,
        recipientIdentityId: ByteArray,
        signerHandle: Long,
        coreSignerHandle: Long,
        accountLabel: String? = null,
        autoAcceptProof: ByteArray? = null,
    ): ContactRequestRef = withContext(Dispatchers.IO) {
        val handle = mapNativeErrors {
            TokensNative.sendContactRequest(
                walletHandle, senderIdentityId, recipientIdentityId,
                accountLabel, autoAcceptProof, signerHandle, coreSignerHandle,
            )
        }
        ContactRequestRef(handle)
    }

    /**
     * Accept the incoming request wrapped by [request], sending the
     * reciprocal request via [signerHandle] and keying the reciprocal
     * send + external-account registration through [coreSignerHandle]
     * (the manager's `MnemonicResolverHandle`). Returns an
     * [EstablishedContactRef] for the newly-established contact.
     */
    suspend fun acceptContactRequest(
        request: ContactRequestRef,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): EstablishedContactRef = withContext(Dispatchers.IO) {
        val handle = mapNativeErrors {
            TokensNative.acceptContactRequest(
                walletHandle, request.value, signerHandle, coreSignerHandle,
            )
        }
        EstablishedContactRef(handle)
    }

    /**
     * Ignore the contact sender [contactIdentityId] (per-sender mute, =
     * block, reversible, **local-only** — no on-chain artifact). Drops
     * their pending incoming request and suppresses ALL of their requests
     * (including rotated ones) from future sweeps; persisted through the
     * changeset pipeline so it survives a relaunch. Replaces the removed
     * per-request reject. ← Swift `wallet.ignoreContactSender`.
     */
    suspend fun ignoreContactSender(
        ourIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    ): Unit = withContext(Dispatchers.IO) {
        mapNativeErrors {
            TokensNative.ignoreContactSender(walletHandle, ourIdentityId, contactIdentityId)
        }
    }

    /**
     * Un-ignore the contact sender [contactIdentityId] (reverse
     * [ignoreContactSender]); their on-chain requests re-fetch on the next
     * sweep. A no-op when the sender wasn't ignored. ← Swift
     * `wallet.unignoreContactSender`.
     */
    suspend fun unignoreContactSender(
        ourIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    ): Unit = withContext(Dispatchers.IO) {
        mapNativeErrors {
            TokensNative.unignoreContactSender(walletHandle, ourIdentityId, contactIdentityId)
        }
    }

    /**
     * Send a Dash payment from [fromIdentityId] to [toContactIdentityId],
     * signing the funding inputs through [coreSignerHandle] (the manager's
     * `MnemonicResolverHandle` — the seed never becomes resident).
     * [amountDuffs] is in duffs (not DASH). Returns the 32-byte transaction
     * id, or null.
     */
    suspend fun sendPayment(
        fromIdentityId: ByteArray,
        toContactIdentityId: ByteArray,
        amountDuffs: Long,
        coreSignerHandle: Long,
        memo: String? = null,
    ): ByteArray? = withContext(Dispatchers.IO) {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        mapNativeErrors {
            TokensNative.sendDashPayPayment(
                walletHandle, fromIdentityId, toContactIdentityId, amountDuffs, memo,
                coreSignerHandle,
            )
        }
    }

    /** Sync DashPay profiles for every managed identity. Returns the synced count. */
    suspend fun syncProfiles(): Int = withContext(Dispatchers.IO) {
        mapNativeErrors { TokensNative.syncDashPayProfiles(walletHandle) }
    }

    /**
     * Read the cached DashPay profile for [identityId] (no network round-trip).
     * Returns a JSON object string (displayName, publicMessage, avatarUrl,
     * avatarHash hex, avatarFingerprint hex) or null when there is no cached
     * profile.
     */
    suspend fun getProfile(identityId: ByteArray): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { TokensNative.getDashPayProfile(walletHandle, identityId) }
    }

    // ── Contact-request enumeration (FriendsView hydration) ───────────
    //
    // Ports the Swift `FriendsView.loadFriends()` pipeline: sync incoming
    // from the network, snapshot the managed identity, read three flat id
    // lists (incoming sender ids / sent recipient ids / established contact
    // ids), and accept via the incoming request handle looked up by sender.

    /**
     * Sync incoming contact requests from Platform for every managed
     * identity on the wallet (applies them to in-memory state; a subsequent
     * [contacts] call observes the result). Blocking; runs on IO. ← Swift
     * `wallet.syncContactRequests()`.
     */
    suspend fun syncContactRequests(): Unit = withContext(Dispatchers.IO) {
        mapNativeErrors { TokensNative.syncContactRequests(walletHandle) }
    }

    /**
     * Fetch the contact requests sent by [identityId] from Platform. ← Swift
     * `wallet.fetchSentContactRequests(identityId:)`.
     */
    suspend fun fetchSentContactRequests(identityId: ByteArray): Unit = withContext(Dispatchers.IO) {
        mapNativeErrors { TokensNative.fetchSentContactRequests(walletHandle, identityId) }
    }

    /** The three contact-id lists hydrated from a managed-identity snapshot. */
    data class Contacts(
        /** 32-byte sender ids of incoming (received) contact requests. */
        val incoming: List<ByteArray>,
        /** 32-byte recipient ids of sent (outgoing) contact requests. */
        val outgoing: List<ByteArray>,
        /** 32-byte contact identity ids of established contacts. */
        val established: List<ByteArray>,
    )

    /**
     * Snapshot [identityId]'s managed identity and read its three contact-id
     * lists — the local-read stage of `FriendsView.loadFriends()`. Opens the
     * `ManagedIdentity` handle, reads the incoming / sent / established id
     * blobs, and destroys the handle before returning (ids copied into flat
     * Kotlin lists). Does no network I/O — call [syncContactRequests] /
     * [fetchSentContactRequests] first to refresh from Platform.
     */
    suspend fun contacts(identityId: ByteArray): Contacts = withContext(Dispatchers.IO) {
        mapNativeErrors {
            val identityHandle = TokensNative.getManagedIdentity(walletHandle, identityId)
            if (identityHandle == 0L) {
                return@mapNativeErrors Contacts(emptyList(), emptyList(), emptyList())
            }
            try {
                Contacts(
                    incoming = decodeIdBlob(
                        TokensNative.managedIdentityIncomingContactRequestIds(identityHandle),
                    ),
                    outgoing = decodeIdBlob(
                        TokensNative.managedIdentitySentContactRequestIds(identityHandle),
                    ),
                    established = decodeIdBlob(
                        TokensNative.managedIdentityEstablishedContactIds(identityHandle),
                    ),
                )
            } finally {
                TokensNative.managedIdentityDestroy(identityHandle)
            }
        }
    }

    /**
     * Accept the incoming contact request from [senderId] to [ourIdentityId],
     * sending the reciprocal request via [signerHandle] and keying the
     * contact crypto through [coreSignerHandle] — port of Swift's
     * `FriendsView.acceptRequest`. Snapshots the managed identity, looks up
     * the incoming request handle by sender, calls the (already-bridged)
     * accept, and frees every transient handle. Returns false when no such
     * incoming request is in local state (sync first).
     */
    suspend fun acceptIncomingRequest(
        ourIdentityId: ByteArray,
        senderId: ByteArray,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors {
            val identityHandle = TokensNative.getManagedIdentity(walletHandle, ourIdentityId)
            if (identityHandle == 0L) return@mapNativeErrors false
            try {
                val requestHandle =
                    TokensNative.getIncomingContactRequest(identityHandle, senderId)
                if (requestHandle == 0L) return@mapNativeErrors false
                try {
                    val established = TokensNative.acceptContactRequest(
                        walletHandle, requestHandle, signerHandle, coreSignerHandle,
                    )
                    if (established != 0L) TokensNative.establishedContactDestroy(established)
                    true
                } finally {
                    TokensNative.contactRequestDestroy(requestHandle)
                }
            } finally {
                TokensNative.managedIdentityDestroy(identityHandle)
            }
        }
    }

    // ── DashPay read surface (upstream #3841 parity) ───────────────────
    //
    // JSON-string reads over the managed-identity snapshot, following the
    // [getProfile] precedent (parsing happens at the consumer); see
    // `DashpayNative` for the field shapes.

    /**
     * Read [identityId]'s DashPay payment history as a JSON array string
     * (empty array when none). Local read over a managed-identity
     * snapshot — no network I/O. This getter is the ONLY durable source
     * of payment rows: `PlatformWalletManager.refreshDashPayPayments`
     * upserts its result into Room (the recurring sweep reconciles
     * payments in-memory without persisting). Returns null when the
     * identity isn't managed by this wallet.
     * ← Swift `ManagedIdentity.getDashPayPayments()`.
     */
    suspend fun payments(identityId: ByteArray): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            withManagedIdentity(identityId) { handle ->
                DashpayNative.managedIdentityDashPayPayments(handle)
            }
        }
    }

    /**
     * Read [identityId]'s DashPay sync state (collection counts +
     * high-water cursors) as a JSON object string, or null when the
     * identity isn't managed by this wallet. Local read.
     * ← Swift `ManagedIdentity.getDashPaySyncState()`.
     */
    suspend fun syncState(identityId: ByteArray): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            withManagedIdentity(identityId) { handle ->
                DashpayNative.managedIdentityDashPaySyncState(handle)
            }
        }
    }

    /**
     * Read the cached public profile of [contactIdentityId] as seen by
     * [ownerIdentityId] (the contact-profile cache the requests/contacts
     * UI renders names + avatars from), or null when none is cached.
     * Same JSON shape as [getProfile]. Local read.
     * ← Swift `ManagedPlatformWallet.getContactProfile(owner:contact:)`.
     */
    suspend fun getContactProfile(
        ownerIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            DashpayNative.getContactProfile(walletHandle, ownerIdentityId, contactIdentityId)
        }
    }

    /**
     * Live DPNS prefix search against Platform (wallet-scoped — the call
     * path iOS `AddContactView` drives). Returns a JSON array string of
     * `{"label":…,"identityId":…hex}`; [limit] 0 means no limit.
     * Blocking network call; runs on IO.
     * ← Swift `ManagedPlatformWallet.searchDpnsNames(prefix:limit:)`.
     */
    suspend fun searchDpnsNames(prefix: String, limit: Int = 10): String? =
        withContext(Dispatchers.IO) {
            require(limit >= 0) { "limit must be non-negative, got $limit" }
            mapNativeErrors { DashpayNative.searchDpnsNames(walletHandle, prefix, limit) }
        }

    // ── Profile / contactInfo writes (upstream #3841 parity) ──────────

    /**
     * Create ([doCreate] = true) or update the DashPay profile for
     * [identityId], signing with [signerHandle]. [avatarBytes] is the raw
     * image — Rust computes the SHA-256 hash + perceptual fingerprint.
     * Broadcasts a real document state transition (blocking, network).
     * Returns the resulting profile JSON (same shape as [getProfile]).
     * ← Swift `createDashPayProfile` / `updateDashPayProfile`.
     */
    @Suppress("LongParameterList")
    suspend fun createOrUpdateProfile(
        identityId: ByteArray,
        displayName: String?,
        publicMessage: String?,
        avatarUrl: String?,
        avatarBytes: ByteArray? = null,
        doCreate: Boolean,
        signerHandle: Long,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            DashpayNative.createOrUpdateProfile(
                walletHandle, identityId, displayName, publicMessage,
                avatarUrl, avatarBytes, doCreate, signerHandle,
            )
        }
    }

    /**
     * Set the owner-private contactInfo (alias / note / [displayHidden])
     * for `(identityId, contactId)`. Local state ALWAYS updates; the
     * encrypted on-chain publish is DIP-15-gated — the returned
     * [ContactInfoPublishOutcome] tells the UI whether the change is
     * cross-device yet ([ContactInfoPublishOutcome.DEFERRED_UNTIL_TWO_CONTACTS]
     * means local-only until a second contact establishes; surface that,
     * matching iOS). ← Swift `setDashPayContactInfo`.
     */
    @Suppress("LongParameterList")
    suspend fun setContactInfo(
        identityId: ByteArray,
        contactId: ByteArray,
        alias: String?,
        note: String?,
        displayHidden: Boolean,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ContactInfoPublishOutcome = withContext(Dispatchers.IO) {
        val raw = mapNativeErrors {
            DashpayNative.setContactInfo(
                walletHandle, identityId, contactId, alias, note,
                displayHidden, signerHandle, coreSignerHandle,
            )
        }
        ContactInfoPublishOutcome.fromRaw(raw)
    }

    /**
     * Open the managed-identity handle for [identityId], run [block],
     * and destroy the handle before returning (the [contacts] /
     * [acceptIncomingRequest] discipline). Returns null when the
     * identity isn't managed by this wallet.
     */
    private inline fun <T> withManagedIdentity(
        identityId: ByteArray,
        block: (Long) -> T,
    ): T? {
        val handle = TokensNative.getManagedIdentity(walletHandle, identityId)
        if (handle == 0L) return null
        return try {
            block(handle)
        } finally {
            TokensNative.managedIdentityDestroy(handle)
        }
    }

    private fun decodeIdBlob(blob: ByteArray?): List<ByteArray> {
        if (blob == null || blob.size < 4) return emptyList()
        val buffer = java.nio.ByteBuffer.wrap(blob) // big-endian by default
        val count = buffer.int
        val out = ArrayList<ByteArray>(count.coerceAtLeast(0))
        repeat(count) {
            if (buffer.remaining() < 32) return out
            val id = ByteArray(32)
            buffer.get(id)
            out.add(id)
        }
        return out
    }
}

/**
 * Outcome of a contactInfo write — mirror of the Rust
 * `CONTACT_INFO_*` discriminants (Swift `ContactInfoPublishOutcome`).
 * Local state always updated; this describes the on-chain publish.
 */
enum class ContactInfoPublishOutcome(val raw: Int) {
    /** Encrypted contactInfo document broadcast — cross-device. */
    PUBLISHED(0),

    /**
     * DIP-15 gate: fewer than two established contacts, so the encrypted
     * publish is deferred (local-only until a second contact establishes).
     */
    DEFERRED_UNTIL_TWO_CONTACTS(1),

    /** Watch-only wallet — cannot sign the publish; local-only. */
    SKIPPED_WATCH_ONLY(2),
    ;

    companion object {
        /**
         * Unknown discriminants degrade to [DEFERRED_UNTIL_TWO_CONTACTS]
         * (the "local-only, not yet cross-device" reading — the safe
         * assumption for a newer Rust enum case).
         */
        fun fromRaw(raw: Int): ContactInfoPublishOutcome =
            entries.firstOrNull { it.raw == raw } ?: DEFERRED_UNTIL_TWO_CONTACTS
    }
}

/** Owned native `ContactRequest` handle from [Dashpay.sendContactRequest]. */
class ContactRequestRef internal constructor(handle: Long) : AutoCloseable {
    private val handleRef = AtomicLong(handle)

    internal val value: Long
        get() = handleRef.get().also { check(it != 0L) { "ContactRequestRef has been closed" } }

    /** Idempotent: the [AtomicLong] swap destroys the handle exactly once. */
    override fun close() {
        val h = handleRef.getAndSet(0)
        if (h != 0L) TokensNative.contactRequestDestroy(h)
    }
}

/** Owned native `EstablishedContact` handle from [Dashpay.acceptContactRequest]. */
class EstablishedContactRef internal constructor(handle: Long) : AutoCloseable {
    private val handleRef = AtomicLong(handle)

    internal val value: Long
        get() = handleRef.get().also { check(it != 0L) { "EstablishedContactRef has been closed" } }

    /** Idempotent: the [AtomicLong] swap destroys the handle exactly once. */
    override fun close() {
        val h = handleRef.getAndSet(0)
        if (h != 0L) TokensNative.establishedContactDestroy(h)
    }
}
