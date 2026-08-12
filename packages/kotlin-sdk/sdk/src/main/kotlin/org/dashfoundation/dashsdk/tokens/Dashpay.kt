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
    ): CreatedInvitation = gate.op {
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
        val blob = mapNativeErrors {
            DashpayNative.createInvitation(
                walletHandle, amountDuffs, fundingAccountIndex,
                inviterIdentityId, inviterUsername, nowUnix, coreSignerHandle,
            )
        }
        checkNotNull(blob) { "native createInvitation returned no data" }
        require(blob.size > 36) { "createInvitation blob too short (${blob.size} bytes)" }
        // Blob layout (fixed by the JNI): outpoint[36] (txid[32] || vout_le[4])
        // then the UTF-8 URI. The 36-byte outpoint is the key the host's own
        // invite-history row and the funding-tx "Invitation" label ride on.
        CreatedInvitation(
            outPoint = blob.copyOfRange(0, 36),
            uri = String(blob, 36, blob.size - 36, Charsets.UTF_8),
        )
    }

    /**
     * A freshly created invitation: the shareable `dashpay://invite` [uri]
     * (embeds the plaintext one-time voucher key — never log it) and the
     * 36-byte funding [outPoint] (`txid[32] || vout_le[4]`) the funding
     * asset-lock landed on. Hosts key their invite-history tracking and the
     * funding-tx classification on the outpoint.
     */
    data class CreatedInvitation(val outPoint: ByteArray, val uri: String) {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is CreatedInvitation) return false
            return outPoint.contentEquals(other.outPoint) && uri == other.uri
        }

        override fun hashCode(): Int = 31 * outPoint.contentHashCode() + uri.hashCode()

        /** Redacts the bearer URI so an accidental log/toString never leaks the voucher key. */
        override fun toString(): String =
            "CreatedInvitation(outPoint=${outPoint.size}B, uri=<redacted>)"
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
