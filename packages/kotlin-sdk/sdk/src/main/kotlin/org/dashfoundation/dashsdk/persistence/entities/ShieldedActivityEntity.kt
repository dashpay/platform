package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentShieldedActivity.swift` — one user-facing shielded
 * activity entry (shield / send / unshield / withdrawal /
 * identity-create / …).
 *
 * Swift `#Unique([\.walletId, \.accountIndex, \.entryId])` → composite
 * primary key (the subwallet scope is required — an intra-wallet transfer
 * legitimately shares one `entryId` across two accounts).
 * Swift `#Index([\.walletId, \.accountIndex])` → index.
 */
@Entity(
    tableName = "shielded_activities",
    primaryKeys = ["walletId", "accountIndex", "entryId"],
    indices = [Index(value = ["walletId", "accountIndex"])],
)
data class ShieldedActivityEntity(
    /** 32-byte wallet id. */
    val walletId: ByteArray,
    /** ZIP-32 account index; Swift `UInt32` → [Int]. */
    val accountIndex: Int,
    /** sha256 of sorted visible output cmxs (32 bytes) — natural key. */
    val entryId: ByteArray,
    /**
     * `ShieldedActivityKind::tag`: 0 Shield, 1 ShieldFromAssetLock,
     * 2 Received, 3 Sent, 4 Unshield, 5 Withdrawal, 6 IdentityCreate,
     * 7 ShieldedSpend.
     */
    val kindTag: Int,
    /** 0 In, 1 Out, 2 Self. */
    val direction: Int,
    /** 0 Pending, 1 Confirmed, 2 Failed. */
    val status: Int,
    /** Principal amount in credits; Swift `UInt64` → [Long]. */
    val amount: Long,
    /** Fee in credits, meaningful only when [hasFee]; Swift `UInt64` → [Long]. */
    val fee: Long,
    val hasFee: Boolean,
    /** Meaningful only when [hasBlockHeight]; Swift `UInt64` → [Long]. */
    val blockHeight: Long,
    val hasBlockHeight: Boolean,
    /** Record time in Unix millis; Swift `UInt64` → [Long]. */
    val createdAtMs: Long,
    /** Created identity id (32 bytes) when kindTag == 6; empty otherwise. */
    val identityId: ByteArray = ByteArray(0),
    /** Counterparty bytes (43B Orchard / 21B PlatformAddress / Core script). */
    val counterparty: ByteArray = ByteArray(0),
    /** 36-byte memo when present; empty otherwise. */
    val memo: ByteArray = ByteArray(0),
    /** Concatenated visible output cmxs (`count` × 32 bytes). */
    val noteCmxs: ByteArray = ByteArray(0),
    /** Concatenated spent nullifiers (`count` × 32 bytes). */
    val spentNullifiers: ByteArray = ByteArray(0),
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
