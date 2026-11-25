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
     * 7 ShieldedSpend, 8 ShieldFromIdentity, 9 IdentityTopUpFromPool,
     * 10 TokenPurchase, 11 TokenShieldedTransfer, 12 TokenUnshield,
     * 13 PrivateDocumentCreate, 14 PrivateContactRequest. Stored as Rust
     * sends it, and a tag the app does not know is shown as 7. A native
     * library that predates a tag reads its rows back as 7 and, when it next
     * emits such a row (a status change), sends 7: the row is then stored as
     * 7, and a later build does not recover the kind.
     *
     * A row is written before its spend is broadcast, so a row of kind 14
     * stands for an attempt: a refused one stays, as Failed, with the
     * recipient's id in [counterparty], although no contact request was sent.
     */
    val kindTag: Int,
    /** 0 In, 1 Out, 2 Self. */
    val direction: Int,
    /** 0 Pending, 1 Confirmed, 2 Failed, 3 Unknown. */
    val status: Int,
    /**
     * Principal amount in credits; Swift `UInt64` → [Long]. In the token's
     * smallest unit for kindTag 11 and 12; the fee for kindTag 13 and 14,
     * which move nothing else (the same number as [fee]).
     */
    val amount: Long,
    /** Fee in credits, meaningful only when [hasFee]; Swift `UInt64` → [Long]. */
    val fee: Long,
    val hasFee: Boolean,
    /** Meaningful only when [hasBlockHeight]; Swift `UInt64` → [Long]. */
    val blockHeight: Long,
    val hasBlockHeight: Boolean,
    /** Record time in Unix millis; Swift `UInt64` → [Long]. */
    val createdAtMs: Long,
    /**
     * Identity id (32 bytes) when the kind carries one: the created
     * identity for kindTag == 6 (IdentityCreate), the funding identity for
     * kindTag == 8 (ShieldFromIdentity), the topped-up identity for
     * kindTag == 9 (IdentityTopUpFromPool); empty otherwise.
     */
    val identityId: ByteArray = ByteArray(0),
    /**
     * Counterparty bytes (43B Orchard / 21B PlatformAddress / Core script),
     * or a 32-byte id: the token for kindTag 10 to 12, the data contract for
     * kindTag 13, the recipient identity for kindTag 14.
     */
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
