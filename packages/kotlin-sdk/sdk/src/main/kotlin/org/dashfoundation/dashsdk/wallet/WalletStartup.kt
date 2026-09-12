package org.dashfoundation.dashsdk.wallet

import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Why a wallet bring-up stopped where it did — Kotlin mirror of the Rust
 * `WalletStartupStatus` (and of Swift's `WalletStartupStatus`). Raw values
 * are the `WalletStartupStatusFFI` ABI discriminants: append, never
 * renumber.
 *
 * Every case is a normal result. A host starts Core SPV on all of them —
 * the partial cases just mean the DIP-15 rescan has work left to do.
 */
enum class WalletStartupStatus(val raw: Int) {
    /**
     * Identity resolved, contacts synced, no contact-account builds left
     * queued. Everything a contact payment needs is in place.
     */
    READY(0),

    /**
     * Platform answered that this seed owns no identity. Terminal, and not
     * a failure — there is nothing to sync and nothing to drain.
     */
    NO_IDENTITY(1),

    /**
     * The identity scan never reached Platform inside the budget. The
     * wallet may well own one; we do not know yet, and asking again may
     * answer it.
     */
    PARTIAL_NO_IDENTITY(2),

    /**
     * Identity resolved and synced, but contact-account builds are still
     * queued — the budget ran out, the drain failed on some entries, or no
     * contact-crypto provider was available.
     */
    PARTIAL_ACCOUNTS_PENDING(3),

    /**
     * Discovery failed locally — a wallet or persistence fault, not a
     * reachability problem. Another scan will not answer it: the same
     * fault is still there.
     */
    DISCOVERY_FAILED(4),

    /**
     * The contact-crypto provider does not resolve the seed that owns this
     * wallet, so the contact-account drain was skipped without deriving
     * anything. Not about Platform being slow: the signer handed to the
     * call belongs to a different wallet, and deriving anyway would write
     * contact receiving addresses from the wrong seed that no later
     * correct-seed pass would ever revisit. The queued work is intact; a
     * rerun with the right signer completes it.
     */
    SEED_BINDING_UNVERIFIED(5),

    /**
     * An identity is known and every later step ran, but the gap-limit
     * identity scan is on record as having left indices unanswered. The
     * identity reported is real; it may not be the only one. The verdict
     * stays on record, so the next launch re-scans instead of taking the
     * warm shortcut.
     */
    IDENTITY_SCAN_INCOMPLETE(6),
    ;

    /**
     * Whether another discovery scan could change the answer: true for
     * [PARTIAL_NO_IDENTITY] (Platform was never reached) and
     * [IDENTITY_SCAN_INCOMPLETE] (reached, but not for every index). The
     * others are terminal for this launch.
     */
    val discoveryWorthRetrying: Boolean
        get() = this == PARTIAL_NO_IDENTITY || this == IDENTITY_SCAN_INCOMPLETE

    /**
     * Whether the identity question has an answer. Not the inverse of
     * [discoveryWorthRetrying]: [DISCOVERY_FAILED] leaves the question
     * open AND is not worth retrying, while [IDENTITY_SCAN_INCOMPLETE]
     * has an answer that is merely known to be partial. Use this to
     * decide what to show, and [discoveryWorthRetrying] to decide whether
     * to scan again.
     */
    val identityIsSettled: Boolean
        get() = this != PARTIAL_NO_IDENTITY && this != DISCOVERY_FAILED

    companion object {
        fun fromRaw(raw: Int): WalletStartupStatus =
            entries.firstOrNull { it.raw == raw }
                ?: throw IllegalArgumentException("unknown WalletStartupStatus discriminant $raw")
    }
}

/**
 * What a wallet bring-up did — Kotlin mirror of the Rust
 * `WalletStartupOutcome` (and of Swift's `WalletStartupOutcome`).
 */
data class WalletStartupOutcome(
    val status: WalletStartupStatus,
    /** The wallet's identity, when one is known by the time this returned. */
    val identityId: ByteArray?,
    /**
     * Discovery scans performed; 0 when a local identity was already known
     * and no network scan was needed.
     */
    val discoveryAttempts: Int,
    /**
     * Whether the inline contact-request pass ran TO COMPLETION. False when
     * it was skipped, failed, ran out of budget, or came back degraded — a
     * pass that could not read some identities' contact documents left
     * their account builds unenqueued, so an empty pending count does not
     * mean their addresses are ready.
     */
    val dashPaySyncRan: Boolean,
    /**
     * The drain was skipped because the contact-crypto provider does not
     * resolve this wallet's seed. Nothing was derived and nothing written;
     * the queued work is intact.
     */
    val seedBindingUnverified: Boolean,
    /**
     * The wallet's identity scan is on record as having left indices
     * unanswered and this launch did not close the gap. Carried separately
     * from [status] because a pending contact queue outranks it there.
     */
    val identityScanIncomplete: Boolean,
    /** Contact-crypto entries the drain completed. */
    val contactAccountsDrained: Int,
    /**
     * Contact-account builds still queued on return. Non-zero means the
     * DIP-15 rescan will have to backfill those contacts' payments.
     */
    val contactAccountsPending: Int,
    /** Wall-clock duration of the whole sequence, in milliseconds. */
    val elapsedMs: Long,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is WalletStartupOutcome) return false
        return status == other.status &&
            (identityId?.contentEquals(other.identityId) ?: (other.identityId == null)) &&
            discoveryAttempts == other.discoveryAttempts &&
            dashPaySyncRan == other.dashPaySyncRan &&
            seedBindingUnverified == other.seedBindingUnverified &&
            identityScanIncomplete == other.identityScanIncomplete &&
            contactAccountsDrained == other.contactAccountsDrained &&
            contactAccountsPending == other.contactAccountsPending &&
            elapsedMs == other.elapsedMs
    }

    override fun hashCode(): Int {
        var result = status.hashCode()
        result = 31 * result + (identityId?.contentHashCode() ?: 0)
        result = 31 * result + discoveryAttempts
        result = 31 * result + dashPaySyncRan.hashCode()
        result = 31 * result + seedBindingUnverified.hashCode()
        result = 31 * result + identityScanIncomplete.hashCode()
        result = 31 * result + contactAccountsDrained
        result = 31 * result + contactAccountsPending
        result = 31 * result + elapsedMs.hashCode()
        return result
    }

    companion object {
        /** Size of the JNI outcome blob; must match the Rust serializer. */
        const val BLOB_SIZE: Int = 57

        /**
         * Decode the fixed-layout big-endian blob produced by
         * `WalletManagerNative.startWalletSubsystems` — the layout table
         * lives on both the JNI wrapper and the external declaration, and
         * the three must stay in lockstep.
         */
        fun decode(blob: ByteArray): WalletStartupOutcome {
            require(blob.size == BLOB_SIZE) {
                "startup outcome blob must be $BLOB_SIZE bytes, got ${blob.size}"
            }
            val buf = ByteBuffer.wrap(blob).order(ByteOrder.BIG_ENDIAN)
            val status = WalletStartupStatus.fromRaw(buf.get().toInt() and 0xFF)
            val hasIdentity = buf.get().toInt() != 0
            val identity = ByteArray(32).also { buf.get(it) }
            val discoveryAttempts = buf.int
            val dashPaySyncRan = buf.get().toInt() != 0
            val seedBindingUnverified = buf.get().toInt() != 0
            val identityScanIncomplete = buf.get().toInt() != 0
            val drained = buf.int
            val pending = buf.int
            val elapsedMs = buf.long
            return WalletStartupOutcome(
                status = status,
                identityId = if (hasIdentity) identity else null,
                discoveryAttempts = discoveryAttempts,
                dashPaySyncRan = dashPaySyncRan,
                seedBindingUnverified = seedBindingUnverified,
                identityScanIncomplete = identityScanIncomplete,
                contactAccountsDrained = drained,
                contactAccountsPending = pending,
                elapsedMs = elapsedMs,
            )
        }
    }
}
