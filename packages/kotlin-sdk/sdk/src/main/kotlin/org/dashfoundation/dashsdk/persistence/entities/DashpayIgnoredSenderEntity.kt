package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentDashpayIgnoredSender.swift` — one DashPay **ignored
 * sender** (per-sender mute, = block, reversible, **local-only**), one row
 * per `(network, owner, ignoredSender)` triple.
 *
 * ## Why this row exists
 *
 * Ignore has no on-chain artifact (syncing it would leak who you ignored
 * via the public contact-request indices), and `contactRequest` documents
 * are immutable and never deleted on-chain — so an ignored sender's
 * requests keep returning on every sync sweep. The Rust side suppresses
 * re-ingest via its in-memory `ignored_senders` set; this row is the
 * durable mirror the load path rehydrates that set from (via the
 * `ignoredSenders` array on
 * [org.dashfoundation.dashsdk.ffi.IdentityRestoreData]). Without it the
 * ignored sender resurfaces after every relaunch.
 *
 * ## Keying
 *
 * Suppression is **per-sender** — bare sender id, no `accountReference`:
 * ALL of the sender's requests (including rotated,
 * bumped-`accountReference` ones) are suppressed, matching the Rust set
 * exactly. (This is the deliberate difference from the old
 * per-`(sender, accountReference)` reject this replaces.)
 *
 * [ownerIdentityId] is the FK materialization of the owning identity with
 * CASCADE — losing the owner identity drops its ignored senders (Swift
 * `PersistentIdentity.dashpayIgnoredSenders` declares `.cascade`).
 */
@Entity(
    tableName = "dashpay_ignored_senders",
    primaryKeys = ["networkRaw", "ownerIdentityId", "ignoredSenderId"],
    indices = [Index(value = ["ownerIdentityId"])],
    foreignKeys = [
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["ownerIdentityId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class DashpayIgnoredSenderEntity(
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** Owning (wallet-managed) identity's 32-byte id — the recipient that ignored. */
    val ownerIdentityId: ByteArray,
    /** The ignored sender's 32-byte identity id (the per-sender suppression key). */
    val ignoredSenderId: ByteArray,
    /** Local row bookkeeping; Swift `ignoredAt`. */
    val ignoredAt: Date = Date(),
)
