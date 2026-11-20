package org.dashfoundation.dashsdk.tokens

import org.dashfoundation.dashsdk.wallet.op

import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.DashpayNative
import org.dashfoundation.dashsdk.ffi.NativeCleaner
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
class Dashpay internal constructor(private val walletHandle: Long,
    private val gate: org.dashfoundation.dashsdk.wallet.TeardownGate? = null,
) {

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
    ): ContactRequestRef = gate.op {
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
    ): EstablishedContactRef = gate.op {
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
    ): Unit = gate.op {
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
    ): Unit = gate.op {
        mapNativeErrors {
            TokensNative.unignoreContactSender(walletHandle, ourIdentityId, contactIdentityId)
        }
    }

    /**
     * Send a Dash payment from [fromIdentityId] to [toContactIdentityId],
     * signing the funding inputs through [coreSignerHandle] (the manager's
     * `MnemonicResolverHandle` — the seed never becomes resident).
     * [amountDuffs] is in duffs (not DASH). Returns the 32-byte transaction
     * id plus the exact network fee (Σin − Σout, sub-dust change folded in)
     * — the Swift `sendPayment` `(txid, feeDuffs)` shape — or null.
     */
    suspend fun sendPayment(
        fromIdentityId: ByteArray,
        toContactIdentityId: ByteArray,
        amountDuffs: Long,
        coreSignerHandle: Long,
        memo: String? = null,
    ): SendPaymentResult? = gate.op {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        val packed = mapNativeErrors {
            TokensNative.sendDashPayPayment(
                walletHandle, fromIdentityId, toContactIdentityId, amountDuffs, memo,
                coreSignerHandle,
            )
        } ?: return@op null
        check(packed.size == 40) { "expected 40-byte txid||fee, got ${packed.size}" }
        var fee = 0L
        for (i in 0 until 8) fee = fee or ((packed[32 + i].toLong() and 0xFF) shl (8 * i))
        SendPaymentResult(txid = packed.copyOfRange(0, 32), feeDuffs = fee)
    }

    /**
     * A broadcast DashPay payment: the transaction id and the exact network
     * fee the transaction pays (mirror of Swift's `(txid, feeDuffs)`).
     */
    data class SendPaymentResult(val txid: ByteArray, val feeDuffs: Long) {
        override fun equals(other: Any?): Boolean =
            other is SendPaymentResult && txid.contentEquals(other.txid) &&
                feeDuffs == other.feeDuffs

        override fun hashCode(): Int = 31 * txid.contentHashCode() + feeDuffs.hashCode()
    }

    /** Sync DashPay profiles for every managed identity. Returns the synced count. */
    suspend fun syncProfiles(): Int = gate.op {
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
    suspend fun syncContactRequests(): Unit = gate.op {
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
            val identityHandle = managedIdentityHandleOrZero(identityId)
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
    ): Boolean = gate.op {
        mapNativeErrors {
            val identityHandle = managedIdentityHandleOrZero(ourIdentityId)
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

    // ── DashPay private contact requests (protocol version 15) ────────
    //
    // ← Swift `PrivateContactRequests.swift`
    // (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift).
    // A private request is sealed to one of the recipient's DashPay keys and
    // paid for from the shielded pool, so nothing on chain names either
    // side; it is signed by the same identity signer as a public request.
    // The wallet records it as an ordinary contact request, so the Room rows
    // and [contacts] read it. The send and the private-reply accept are
    // shielded spends and live on `PlatformWalletManager`.

    /**
     * How an identity and one contact exchanged private contact requests.
     * ← Swift `PrivateContactInfo`.
     */
    data class PrivateContactInfo(
        /** The contact's request came privately, so accepting it replies privately. */
        val receivedPrivately: Boolean,
        /** This identity's request to the contact went privately. */
        val sentPrivately: Boolean,
        /** The 43-byte Orchard address the contact asked to be paid at privately. */
        val shieldedAddress: ByteArray?,
    ) {
        override fun equals(other: Any?): Boolean =
            other is PrivateContactInfo && receivedPrivately == other.receivedPrivately &&
                sentPrivately == other.sentPrivately &&
                (shieldedAddress?.contentEquals(other.shieldedAddress) ?: (other.shieldedAddress == null))

        override fun hashCode(): Int =
            (31 * receivedPrivately.hashCode() + sentPrivately.hashCode()) * 31 +
                (shieldedAddress?.contentHashCode() ?: 0)

        internal companion object {
            /** Decode the JSON object `DashpayNative.managedIdentityPrivateContact` returns. */
            fun fromJson(json: String): PrivateContactInfo {
                val obj = org.json.JSONObject(json)
                val address = if (obj.isNull("shieldedAddress")) {
                    null
                } else {
                    obj.getString("shieldedAddress").chunked(2).map { it.toInt(16).toByte() }.toByteArray()
                }
                return PrivateContactInfo(
                    receivedPrivately = obj.getBoolean("receivedPrivately"),
                    sentPrivately = obj.getBoolean("sentPrivately"),
                    shieldedAddress = address,
                )
            }
        }
    }

    /**
     * What a contact request from one of the wallet's identities waits for,
     * as the wallet decides it, so a host shows "restoring private payments"
     * apart from "reading the private store" and from a contact's pending
     * request rather than inferring them from the refusals it got.
     * ← Swift `PrivateContactReadiness`.
     */
    data class PrivateContactReadiness(
        /**
         * Whether the wallet's private payments are restored for the
         * identity. While false, its contact requests and acceptances, public
         * or private, are refused with
         * [org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.PrivateContactPaymentsNotRestored]
         * until the shielded store is configured or the wallet's shielded
         * account bound, or a contact request's own retry of a failed
         * restoration succeeds; no private sync lifts it.
         */
        val privatePaymentsRestored: Boolean,
        /**
         * Whether a private sync read DashPay's private contact request store
         * to its end for the identity with its current keys. Every contact
         * request and acceptance reads it again from the cursor before it is
         * decided, and is refused with
         * [org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.PrivateContactStoreNotRead]
         * while that read cannot finish.
         */
        val privateStoreRead: Boolean,
        /**
         * Whether a private contact request the identity paid for may still
         * be unresolved: the marker its private scan state stores, or a
         * payment the shielded store's journal keeps unresolved. One pair's
         * hold is [privateRepliesPending].
         */
        val unresolvedPrivatePayment: Boolean,
        /**
         * Whether the last restoration of the wallet's private payments
         * failed while [privatePaymentsRestored] is false: the hold then
         * lasts until one succeeds. [privatePaymentsRestoreFailure] reads why.
         */
        val privatePaymentsRestoreFailed: Boolean = false,
    )

    /** What one pass over DashPay's private contact request store did. */
    data class PrivateContactSyncResult(
        /** The requests the pass ingested, received and sent, including ones already held. */
        val requestCount: Long,
        /** The store pages read. */
        val pagesRead: Long,
        /** Entries that opened for one of the wallet's identities but were refused. */
        val rejectedCount: Long,
        /** The store position the pass stopped at. */
        val nextPosition: Long,
        /** Whether the pass read to the store's last entry. When false, sync again to read on. */
        val complete: Boolean,
        /** Identities whose ingest did not reach disk; the next pass reads their entries again. */
        val unpersistedIdentityCount: Long,
    )

    /**
     * Read DashPay's private contact request store for every identity of
     * this wallet, at most [maxPages] pages (the default reads to the end),
     * and ingest what it holds for them: requests to them, each sender's
     * signature checked, and requests they sent, recovered from the store
     * alone. The ingested requests reach Room through the persister.
     *
     * The store is opened with keys derived inside [coreSignerHandle], the
     * manager's `MnemonicResolverHandle`. Where the persister attests
     * `PRIVATE_CONTACT_REQUEST_SCAN_STATE` (the Room handler does), each
     * identity's read position is stored with the requests read before it
     * and restored at load, so a pass after launch reads on from there;
     * otherwise the first pass after launch reads the store from its start.
     * Private requests reach the wallet only through this read.
     * ← Swift `ManagedPlatformWallet.syncPrivateContactRequests(maxPages:)`.
     */
    suspend fun syncPrivateContactRequests(
        coreSignerHandle: Long,
        maxPages: Int = Int.MAX_VALUE,
    ): PrivateContactSyncResult = gate.op {
        val counters = mapNativeErrors {
            DashpayNative.syncPrivateContactRequests(walletHandle, coreSignerHandle, maxPages)
        }
        checkNotNull(counters) { "native syncPrivateContactRequests returned no report" }
        check(counters.size == 6) { "expected 6 sync counters, got ${counters.size}" }
        PrivateContactSyncResult(
            requestCount = counters[0],
            pagesRead = counters[1],
            rejectedCount = counters[2],
            nextPosition = counters[4],
            complete = counters[5] != 0L,
            unpersistedIdentityCount = counters[3],
        )
    }

    /**
     * How [ourIdentityId] and [contactId] exchanged private contact
     * requests: whether the contact's request came privately, whether ours
     * went privately, and the shielded address the contact asked to be paid
     * at (only when its request came privately, and only once a private
     * sync this launch has read it). Null when neither tracked request
     * between them came or went privately, or the identity isn't managed by
     * this wallet. Read from a fresh managed-identity snapshot; local read.
     * ← Swift `ManagedIdentity.privateContact(contactId:)`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift).
     */
    suspend fun privateContact(
        ourIdentityId: ByteArray,
        contactId: ByteArray,
    ): PrivateContactInfo? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            withManagedIdentity(ourIdentityId) { handle ->
                DashpayNative.managedIdentityPrivateContact(handle, contactId)
            }?.let(PrivateContactInfo::fromJson)
        }
    }

    /**
     * What a contact request from [ourIdentityId] waits for, as the wallet
     * decides it ([PrivateContactReadiness]). Not a pure read: a journal
     * record discharged since (a payment proven never to execute) is applied
     * first, briefly taking the wallet's write lock, as a send would see it;
     * then it reads live wallet state. Read it again after the shielded store
     * is configured or the wallet binds its shielded account to see the
     * restore. Throws for an identity the wallet does not hold.
     * ← Swift `ManagedPlatformWallet.privateContactReadiness(identityId:)`.
     */
    suspend fun privateContactReadiness(ourIdentityId: ByteArray): PrivateContactReadiness =
        withContext(Dispatchers.IO) {
            val flags = mapNativeErrors { DashpayNative.privateContactReadiness(walletHandle, ourIdentityId) }
            checkNotNull(flags) { "native privateContactReadiness returned no flags" }
            check(flags.size == 4) { "expected 4 readiness flags, got ${flags.size}" }
            PrivateContactReadiness(
                privatePaymentsRestored = flags[0],
                privateStoreRead = flags[1],
                unresolvedPrivatePayment = flags[2],
                privatePaymentsRestoreFailed = flags[3],
            )
        }

    /**
     * Why the last restoration of the wallet's private payments failed,
     * while they are not restored for [ourIdentityId] and none succeeded
     * since, or null when none failed (restored, or only not restored yet).
     * What [PrivateContactReadiness.privatePaymentsRestoreFailed] flags.
     * Throws for an identity the wallet does not hold.
     * ← Swift `ManagedPlatformWallet.privatePaymentsRestoreFailure(identityId:)`.
     */
    suspend fun privatePaymentsRestoreFailure(ourIdentityId: ByteArray): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors { DashpayNative.privatePaymentsRestoreFailure(walletHandle, ourIdentityId) }
        }

    /**
     * The contacts among [contactIds] whose pair with [ourIdentityId] the
     * wallet holds for a private contact request, as a send decides it
     * (`DashPayView::private_pair_held`, one read per contact): a private
     * request in flight, paid and not yet read back from the store, or kept
     * by the shielded store's journal, also after the identity was removed
     * and added again. While a pair is held, Rust refuses another private
     * send or reply to it
     * ([DashSdkError.PlatformWallet.PrivateContactRequestPending]) and holds
     * a public one, so a host shows it as pending and offers neither. A
     * private sync that reads the entry back frees it, as does the payment
     * proven never to execute (its anchor pruned, which the shielded sync
     * judges); an endpoint's refusal of the payment does not. A journal
     * record discharged since is applied first. Empty when the identity
     * isn't managed.
     * ← Swift `ManagedPlatformWallet.privatePairHeld(senderId:recipientId:)`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift).
     */
    suspend fun privateRepliesPending(
        ourIdentityId: ByteArray,
        contactIds: List<ByteArray>,
    ): List<ByteArray> = withContext(Dispatchers.IO) {
        if (contactIds.isEmpty()) return@withContext emptyList()
        mapNativeErrors {
            // An identity the wallet does not manage holds nothing.
            val managed = withManagedIdentity(ourIdentityId) { true } ?: false
            if (!managed) {
                emptyList()
            } else {
                contactIds.filter { DashpayNative.privatePairHeld(walletHandle, ourIdentityId, it) }
            }
        }
    }

    /**
     * The contacts among [contactIds] to which a public contact request from
     * [ourIdentityId] was broadcast and may have been delivered, with no
     * proof settling its outcome yet (`DashPayView::public_contact_request_unconfirmed`,
     * one read per contact). The send or accept that broadcast it threw
     * [DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed] (20);
     * while it stands, the wallet refuses every contact request or acceptance
     * to the contact, public or private
     * ([DashSdkError.PlatformWallet.PublicContactRequestUnconfirmed], 76), so
     * a host shows the pair as unconfirmed and offers neither. A DashPay sync
     * settles it: a request found is recorded as sent, a nonce proved used
     * frees the pair, and otherwise the same signed bytes are broadcast
     * again only while the wallet's tracked state still allows a public
     * request to the contact; a private send never broadcasts them again.
     * Read it beside [privateRepliesPending]. Empty when the identity
     * isn't managed.
     * ← Swift `ManagedPlatformWallet.publicContactRequestUnconfirmed(senderId:recipientId:)`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift).
     */
    suspend fun publicContactRequestsUnconfirmed(
        ourIdentityId: ByteArray,
        contactIds: List<ByteArray>,
    ): List<ByteArray> = withContext(Dispatchers.IO) {
        if (contactIds.isEmpty()) return@withContext emptyList()
        mapNativeErrors {
            // An identity the wallet does not manage holds nothing.
            val managed = withManagedIdentity(ourIdentityId) { true } ?: false
            if (!managed) {
                emptyList()
            } else {
                contactIds.filter {
                    DashpayNative.publicContactRequestUnconfirmed(walletHandle, ourIdentityId, it)
                }
            }
        }
    }

    /**
     * The incoming contact request from [senderId] to [ourIdentityId] in
     * local state, or null when there is none (or the identity isn't
     * managed by this wallet). Close the returned ref when done. This is
     * the request `PlatformWalletManager.acceptContactRequestReplyingInKind`
     * takes: the accept that replies privately to a request that came
     * privately, and with a document otherwise. Local read; no network I/O.
     * ← Swift `ManagedIdentity.getIncomingContactRequest(senderId:)`.
     */
    suspend fun incomingContactRequest(
        ourIdentityId: ByteArray,
        senderId: ByteArray,
    ): ContactRequestRef? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            withManagedIdentity(ourIdentityId) { identityHandle ->
                val requestHandle =
                    TokensNative.getIncomingContactRequest(identityHandle, senderId)
                if (requestHandle == 0L) null else ContactRequestRef(requestHandle)
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

    // ── DIP-15 auto-accept QR (upstream #3841 parity) ──────────────────

    /**
     * Build the owner's DIP-15 auto-accept QR URI for [identityId]
     * (`dash:?du=…&dapk=…`), keying the proof through [coreSignerHandle].
     * The UI renders it as a QR (ZXing). ← Swift `buildAutoAcceptQR`.
     *
     * [username] is the owner's DPNS name and is **required** (matching
     * Swift's `username: String`): the underlying FFI rejects a null string,
     * so pass `""` for a nameless identity — Rust then resolves the name
     * on-chain (or surfaces a clear "no name registered" error).
     */
    suspend fun buildAutoAcceptQr(
        identityId: ByteArray,
        username: String,
        coreSignerHandle: Long,
    ): String? = gate.op {
        mapNativeErrors {
            DashpayNative.buildAutoAcceptQr(walletHandle, identityId, username, coreSignerHandle)
        }
    }

    /**
     * Scan-to-send: parse an auto-accept QR [uri] and send the contact
     * request it describes from [senderIdentityId]. Blocking network
     * call; runs on IO. Returns the created request wrapped as a
     * [ContactRequestRef] — close it (or `use {}`) when done.
     * ← Swift `sendContactRequestFromQR`.
     */
    suspend fun sendContactRequestFromQr(
        senderIdentityId: ByteArray,
        uri: String,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ContactRequestRef = gate.op {
        val handle = mapNativeErrors {
            DashpayNative.sendContactRequestFromQr(
                walletHandle, senderIdentityId, uri, signerHandle, coreSignerHandle,
            )
        }
        ContactRequestRef(handle)
    }

    // ── DIP-13 invitations ────────────────────────────────────────────
    //
    // The invitation link (parse input / create output) IS a bearer
    // credential — the one-time voucher private key rides inside as a
    // WIF. Never log or persist it; the claim/reclaim flows re-derive
    // everything they need from Rust-side state.

    /**
     * Decode a `dashpay://invite` link into a read-only [InvitationPreview]
     * — no network, no side effects. A malformed link yields
     * `structurallyValid == false`, never an exception, so the claim UI can
     * render a clean "invalid invitation" state.
     * ← Swift `ManagedPlatformWallet.parseInvitation`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift).
     */
    suspend fun parseInvitation(uri: String): InvitationPreview = gate.op {
        val json = mapNativeErrors { DashpayNative.parseInvitation(uri) }
        InvitationPreview.fromJson(json)
    }

    /**
     * Create a DashPay invitation voucher and return the shareable
     * `dashpay://invite` link. Blocking (builds + broadcasts an L1 asset
     * lock at the DIP-13 invitation path and waits for its InstantSend
     * proof); runs on IO. The sent-invitation row lands in Room via the
     * persistence callback before this returns, so the "Sent invitations"
     * `Flow` updates without any extra call.
     * ← Swift `ManagedPlatformWallet.createInvitation`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift).
     *
     * Amount bounds ([minInvitationDuffs], [maxInvitationDuffs]) are
     * enforced in Rust — a voucher below the floor could fund neither a
     * claim nor a reclaim.
     *
     * Pass a non-null [inviterIdentityId] (32 bytes) + [inviterUsername]
     * to opt into the contact-bootstrap (the link then carries the
     * username so the invitee can send a contact request back); null for a
     * pure funding voucher.
     *
     * **The returned link embeds the plaintext one-time voucher key.**
     */
    suspend fun createInvitation(
        amountDuffs: Long,
        fundingAccountIndex: Int,
        inviterIdentityId: ByteArray?,
        inviterUsername: String?,
        coreSignerHandle: Long,
        // Unix seconds as Long: the shared C API takes a u32, so an Int
        // default would go negative in 2038 and be rejected by the JNI
        // guard even though the API itself remains valid.
        nowUnix: Long = System.currentTimeMillis() / 1000L,
    ): String = gate.op {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        require(fundingAccountIndex >= 0) {
            "fundingAccountIndex must be non-negative, got $fundingAccountIndex"
        }
        require(inviterIdentityId == null || inviterIdentityId.size == 32) {
            "inviterIdentityId must be 32 bytes when set"
        }
        require(inviterIdentityId == null || !inviterUsername.isNullOrEmpty()) {
            "inviterUsername is required when inviterIdentityId is set"
        }
        val uri = mapNativeErrors {
            DashpayNative.createInvitation(
                walletHandle, amountDuffs, fundingAccountIndex,
                inviterIdentityId, inviterUsername, nowUnix, coreSignerHandle,
            )
        }
        checkNotNull(uri) { "native createInvitation returned no link" }
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
    ): String? = gate.op {
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
    ): ContactInfoPublishOutcome = gate.op {
        val raw = mapNativeErrors {
            DashpayNative.setContactInfo(
                walletHandle, identityId, contactId, alias, note,
                displayHidden, signerHandle, coreSignerHandle,
            )
        }
        ContactInfoPublishOutcome.fromRaw(raw)
    }

    /**
     * Snapshot the managed identity for [identityId], or 0 when the wallet
     * does not manage it. The native side reports an unmanaged identity as
     * a platform-wallet NotFound error rather than a zero handle, so the
     * "not managed" outcome is translated here — every local-read caller
     * treats it as an absence (null / empty / false), never an exception.
     * An invalid/stale wallet handle is a distinct native error
     * (ErrorInvalidHandle) that is NOT translated, so it propagates instead
     * of masquerading as an unmanaged identity (dashpay/platform#4060).
     */
    private fun managedIdentityHandleOrZero(identityId: ByteArray): Long =
        translateManagedIdentityNotFoundToZero {
            TokensNative.getManagedIdentity(walletHandle, identityId)
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
        val handle = managedIdentityHandleOrZero(identityId)
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

    companion object {
        /**
         * Upper bound, in duffs, on the amount an invitation voucher may
         * lock — the Rust-enforced cap [createInvitation] rejects above.
         * Read it rather than restating it: Rust owns the value, and a
         * client-side copy diverges the moment the constant moves.
         * ← Swift `ManagedPlatformWallet.maxInvitationDuffs`.
         */
        val maxInvitationDuffs: Long get() = DashpayNative.invitationMaxDuffs()

        /**
         * Lower bound, in duffs, on the amount an invitation voucher may
         * lock — the Rust-enforced floor [createInvitation] rejects below
         * (a smaller voucher can fund neither a claim nor a reclaim).
         * ← Swift `ManagedPlatformWallet.minInvitationDuffs`.
         */
        val minInvitationDuffs: Long get() = DashpayNative.invitationMinDuffs()
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
    private val cleanable = NativeCleaner.register(this, HandleCleanup(handleRef))

    internal val value: Long
        get() = handleRef.get().also { check(it != 0L) { "ContactRequestRef has been closed" } }

    /** Idempotent: destroys the handle exactly once, on [close] or the GC backstop. */
    override fun close() {
        cleanable.clean()
    }

    /** Runs on [NativeCleaner] or [close]; destroys the handle exactly once. */
    private class HandleCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val h = handleRef.getAndSet(0)
            if (h != 0L) TokensNative.contactRequestDestroy(h)
        }
    }
}

/** Owned native `EstablishedContact` handle from [Dashpay.acceptContactRequest]. */
class EstablishedContactRef internal constructor(handle: Long) : AutoCloseable {
    private val handleRef = AtomicLong(handle)
    private val cleanable = NativeCleaner.register(this, HandleCleanup(handleRef))

    internal val value: Long
        get() = handleRef.get().also { check(it != 0L) { "EstablishedContactRef has been closed" } }

    /** Idempotent: destroys the handle exactly once, on [close] or the GC backstop. */
    override fun close() {
        cleanable.clean()
    }

    /** Runs on [NativeCleaner] or [close]; destroys the handle exactly once. */
    private class HandleCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val h = handleRef.getAndSet(0)
            if (h != 0L) TokensNative.establishedContactDestroy(h)
        }
    }
}

// ── Free functions (unit-testable, no `this`) ─────────────────────────

/**
 * Run [getHandle] (a `TokensNative.getManagedIdentity` call), translating
 * the platform-wallet NotFound error the native layer raises for an
 * identity the wallet does not manage into a zero handle — the same "not
 * managed" signal the callers already handle by returning null / empty.
 *
 * The FFI's blanket `Option → result` conversion reports the miss as
 * `PlatformWalletFFIResultCode::NotFound` (98, offset into the
 * `DashSDKException` code by [DashSdkError.PLATFORM_WALLET_CODE_OFFSET]),
 * so without this every local read over an unmanaged identity — e.g.
 * [Dashpay.syncState] on a contact's identity — would throw
 * a typed `DashSdkError.PlatformWallet.NotFound("…ManagedIdentity not
 * found")` instead of returning null. Any other error is rethrown
 * untouched.
 */
internal inline fun translateManagedIdentityNotFoundToZero(getHandle: () -> Long): Long =
    try {
        getHandle()
    } catch (e: DashSDKException) {
        val notFound = DashSdkError.PLATFORM_WALLET_CODE_OFFSET +
            DashSdkError.PLATFORM_WALLET_NOT_FOUND_CODE
        if (e.code == notFound) 0L else throw e
    }

/**
 * Read-only preview of a `dashpay://invite` link, decoded off-chain by
 * [Dashpay.parseInvitation] — mirror of Swift
 * `ManagedPlatformWallet.InvitationPreview`.
 *
 * The legacy link carries neither the amount nor an expiry, so
 * [amountDuffs] and [expiryUnix] are always 0 (the claim UI shows "—"; the
 * amount resolves when the claim refetches the funding tx). Gate contact
 * features on a non-null [inviterUsername], not on [hasInviter]: a
 * metadata-only link (display-name/avatar without a `du` username) sets
 * the flag while the username stays null.
 */
data class InvitationPreview(
    /** The link decoded structurally; when false every other field is unset. */
    val structurallyValid: Boolean,
    /** The link carried an `islock` (InstantSend); false ⇒ ChainLock invite. */
    val isInstant: Boolean,
    /** The link carried inviter metadata (username, display name, or avatar). */
    val hasInviter: Boolean,
    /** Inviter DPNS username, or null (metadata-only or pure funding link). */
    val inviterUsername: String?,
    /** Always 0 — not on the wire; resolved at claim time. */
    val amountDuffs: Long = 0,
    /** Always 0 — the legacy link carries no expiry field. */
    val expiryUnix: Int = 0,
) {
    companion object {
        /** An all-unset preview — the malformed-link shape. */
        val INVALID = InvitationPreview(
            structurallyValid = false,
            isInstant = false,
            hasInviter = false,
            inviterUsername = null,
        )

        /** Parse the compact JSON emitted by `DashpayNative.parseInvitation`. */
        internal fun fromJson(json: String?): InvitationPreview {
            if (json.isNullOrEmpty()) return INVALID
            return try {
                val obj = org.json.JSONObject(json)
                InvitationPreview(
                    structurallyValid = obj.optBoolean("structurallyValid", false),
                    isInstant = obj.optBoolean("isInstant", false),
                    hasInviter = obj.optBoolean("hasInviter", false),
                    inviterUsername = if (obj.isNull("inviterUsername")) {
                        null
                    } else {
                        obj.optString("inviterUsername").takeIf { it.isNotEmpty() }
                    },
                    amountDuffs = obj.optLong("amountDuffs", 0),
                    expiryUnix = obj.optInt("expiryUnix", 0),
                )
            } catch (_: org.json.JSONException) {
                INVALID
            }
        }
    }
}
