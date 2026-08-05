package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentInvitation.swift` — one **created** DashPay invitation
 * (DIP-13), one row per `(walletId, outpoint)`. Upserted by the
 * `on_persist_invitations_fn` persister callback whenever `create_invitation`
 * flushes an `InvitationChangeSet`; the "Sent invitations" list reads it via
 * a Room `Flow`.
 *
 * Room is the UI source of truth — there is **no** Rust→Kotlin rehydrate path
 * (the `fundingIndexRaw` re-derives the voucher key inside Rust), so a store
 * wipe loses only list *visibility*, never funds. **No secret column:** the
 * one-time voucher key is never stored.
 *
 * Swift `@Attribute(.unique)` on `outPointHex` → primary key
 * (`<txid display hex>:<vout>`, same encode as [AssetLockEntity]).
 * Swift `#Index([\.walletId])` → index. No FK to `wallets`: rows are deleted
 * by wallet teardown (`deleteWalletData`), mirroring the Swift model.
 */
@Entity(
    tableName = "invitations",
    indices = [Index(value = ["walletId"])],
)
data class InvitationEntity(
    @PrimaryKey val outPointHex: String,
    /**
     * Raw 36-byte outpoint (`txid_le ‖ vout_le`), stored alongside
     * [outPointHex] so the reclaim flow can rebuild an `OutPointFFI` directly
     * without a reverse-decode of the display string. The persistence
     * callback already has these bytes in hand from `InvitationEntryFFI`.
     */
    val rawOutPoint: ByteArray,
    /** 32-byte wallet id that created this invitation. */
    val walletId: ByteArray,
    /**
     * DIP-13 invitation funding index (`m/9'/coin'/5'/3'/<index>'`) — the
     * handle that re-derives the voucher key and drives recovery/reclaim.
     */
    val fundingIndexRaw: Int,
    /** Amount locked in the voucher (duffs). */
    val amountDuffs: Long,
    /** Advisory expiry (unix seconds); display-only, not consensus-enforced. */
    val expiryUnix: Long,
    /** Creation time (unix seconds), from the Rust changeset. */
    val createdAtSecs: Long,
    /** Whether the link carried inviter info (contact-bootstrap opted in). */
    val hasInviter: Boolean,
    /**
     * Invitation status discriminant: 0 = Created, 1 = Claimed,
     * 2 = Reclaimed (pinned Rust-side by `status_to_u8`). Rust emits only
     * `Created`; the Claimed/Reclaimed transitions are written locally by
     * the app's reclaim-outcome classifier — the persist callback preserves
     * this column on conflict so a Rust re-emit can never reset it.
     */
    val statusRaw: Int,
    /**
     * Set true just before this wallet's own reclaim consume is submitted,
     * cleared once the terminal status is saved. It survives a crash between
     * the on-chain consume and the `statusRaw = 2` save, so a retry that hits
     * "already consumed" can tell *our own* reclaim (in-flight → Reclaimed)
     * from a voucher someone else claimed (never in-flight → Claimed).
     * Crash forensics only — never a concurrency guard.
     */
    val reclaimInFlight: Boolean = false,
    val createdAt: Date = Date(),
    val updatedAt: Date = Date(),
)
