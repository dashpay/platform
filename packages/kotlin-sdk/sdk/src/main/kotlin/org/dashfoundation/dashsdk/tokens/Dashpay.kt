package org.dashfoundation.dashsdk.tokens

import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TokensNative

/**
 * DashPay contact-request lifecycle + payments — the Android analog of the
 * Swift `ManagedPlatformWallet` DashPay calls driving `FriendsView`
 * (`packages/swift-sdk/.../PlatformWallet/ManagedPlatformWallet.swift`).
 *
 * Covers the single-call operations: send / accept / reject a contact
 * request, send a payment, and read / sync the cached DashPay profile —
 * plus the `FriendsView` hydration surface ([syncContactRequests] /
 * [fetchSentContactRequests] + [contacts] / [acceptIncomingRequest]) built
 * over the managed-identity contact-id enumerators.
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
     * Send a contact request to [recipientIdentityId], signing with
     * [signerHandle]. [accountLabel] (encrypted by the SDK) and
     * [autoAcceptProof] are optional. Returns a [ContactRequestRef] wrapping
     * the created native handle — close it (or `use {}`) when done.
     */
    suspend fun sendContactRequest(
        senderIdentityId: ByteArray,
        recipientIdentityId: ByteArray,
        signerHandle: Long,
        accountLabel: String? = null,
        autoAcceptProof: ByteArray? = null,
    ): ContactRequestRef = withContext(Dispatchers.IO) {
        val handle = mapNativeErrors {
            TokensNative.sendContactRequest(
                walletHandle, senderIdentityId, recipientIdentityId,
                accountLabel, autoAcceptProof, signerHandle,
            )
        }
        ContactRequestRef(handle)
    }

    /**
     * Accept the incoming request wrapped by [request], sending the
     * reciprocal request via [signerHandle]. Returns an
     * [EstablishedContactRef] for the newly-established contact.
     */
    suspend fun acceptContactRequest(
        request: ContactRequestRef,
        signerHandle: Long,
    ): EstablishedContactRef = withContext(Dispatchers.IO) {
        val handle = mapNativeErrors {
            TokensNative.acceptContactRequest(walletHandle, request.value, signerHandle)
        }
        EstablishedContactRef(handle)
    }

    /** Reject an incoming contact request from [contactIdentityId] (local drop). */
    suspend fun rejectContactRequest(
        ourIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    ): Unit = withContext(Dispatchers.IO) {
        mapNativeErrors {
            TokensNative.rejectContactRequest(walletHandle, ourIdentityId, contactIdentityId)
        }
    }

    /**
     * Send a Dash payment from [fromIdentityId] to [toContactIdentityId].
     * [amountDuffs] is in duffs (not DASH). Returns the 32-byte transaction
     * id, or null.
     */
    suspend fun sendPayment(
        fromIdentityId: ByteArray,
        toContactIdentityId: ByteArray,
        amountDuffs: Long,
        memo: String? = null,
    ): ByteArray? = withContext(Dispatchers.IO) {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        mapNativeErrors {
            TokensNative.sendDashPayPayment(
                walletHandle, fromIdentityId, toContactIdentityId, amountDuffs, memo,
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
     * sending the reciprocal request via [signerHandle] — port of Swift's
     * `FriendsView.acceptRequest`. Snapshots the managed identity, looks up
     * the incoming request handle by sender, calls the (already-bridged)
     * accept, and frees every transient handle. Returns false when no such
     * incoming request is in local state (sync first).
     */
    suspend fun acceptIncomingRequest(
        ourIdentityId: ByteArray,
        senderId: ByteArray,
        signerHandle: Long,
    ): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors {
            val identityHandle = TokensNative.getManagedIdentity(walletHandle, ourIdentityId)
            if (identityHandle == 0L) return@mapNativeErrors false
            try {
                val requestHandle =
                    TokensNative.getIncomingContactRequest(identityHandle, senderId)
                if (requestHandle == 0L) return@mapNativeErrors false
                try {
                    val established =
                        TokensNative.acceptContactRequest(walletHandle, requestHandle, signerHandle)
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
