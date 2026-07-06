package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentDashpayContactRequest.swift` — one directional
 * DashPay `contactRequest` document, one row per
 * `(network, owner, contact, isOutgoing)` quad.
 *
 * Swift `#Unique([\.networkRaw, \.ownerIdentityId, \.contactIdentityId,
 * \.isOutgoing])` → composite primary key.
 *
 * [ownerIdentityId] doubles as the FK materialization of the NON-optional
 * `owner` relationship (Swift keeps it equal to `owner.identityId` by
 * construction) with CASCADE (Swift `PersistentIdentity.contactRequests`
 * declares `.cascade`).
 */
@Entity(
    tableName = "dashpay_contact_requests",
    primaryKeys = ["networkRaw", "ownerIdentityId", "contactIdentityId", "isOutgoing"],
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
data class DashpayContactRequestEntity(
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** Owning (wallet-managed) identity's 32-byte id. */
    val ownerIdentityId: ByteArray,
    /** Other party's 32-byte identity id (recipient if outgoing, sender if incoming). */
    val contactIdentityId: ByteArray,
    /** true ⇒ owner sent this request; false ⇒ owner received it. */
    val isOutgoing: Boolean,
    /** `ContactRequest::sender_key_index`; Swift `UInt32` → [Int]. */
    val senderKeyIndex: Int,
    /** `ContactRequest::recipient_key_index`; Swift `UInt32` → [Int]. */
    val recipientKeyIndex: Int,
    /** `ContactRequest::account_reference`; Swift `UInt32` → [Int]. */
    val accountReference: Int,
    /** ECDH-sealed key bytes — opaque passthrough. Always non-empty. */
    val encryptedPublicKey: ByteArray,
    val encryptedAccountLabel: ByteArray? = null,
    val autoAcceptProof: ByteArray? = null,
    /** Core block height the request landed at; Swift `UInt32` → [Int]. */
    val coreHeightCreatedAt: Int,
    /** Document `created_at` in Unix millis; Swift `UInt64` → [Long]. */
    val createdAtMillis: Long,
    /**
     * Whether the established relationship's payment channel is
     * permanently broken (disables "Send Dash" in the UI). Only stamped
     * on established rows; always false while pending. Swift
     * `paymentChannelBroken`.
     */
    @ColumnInfo(defaultValue = "0")
    val paymentChannelBroken: Boolean = false,
    /** Owner-private alias for the contact (contactInfo). Swift `contactAlias`. */
    val contactAlias: String? = null,
    /** Owner-private note (contactInfo). Swift `contactNote`. */
    val contactNote: String? = null,
    /** `contactInfo.displayHidden`. Swift `contactHidden`. */
    @ColumnInfo(defaultValue = "0")
    val contactHidden: Boolean = false,
    /**
     * The contact's decrypted DIP-15 account label — direction-specific
     * (incoming established row only). Swift `contactAccountLabel`.
     */
    val contactAccountLabel: String? = null,
    /**
     * DIP-15 accepted-account acceptances, big-endian `u32`s packed 4
     * bytes each (see `encodeAcceptedAccounts`). Null when empty. Swift
     * `contactAcceptedAccounts` (`[UInt32]`).
     */
    val contactAcceptedAccounts: ByteArray? = null,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
