package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentDashpayPayment.swift` — one DashPay payment-history
 * entry, a mirror of one entry in the Rust-side `dashpay_payments` map on
 * a `ManagedIdentity`, one row per `(network, owner, txid)` triple.
 *
 * Unlike the contact-request rows this entity is **not** populated by the
 * persister callback — the Rust persister doesn't project payment
 * history. Rows are refreshed on demand from the
 * `managed_identity_get_dashpay_payments` FFI getter via
 * `PlatformWalletManager.refreshDashPayPayments`, which upserts here so
 * the UI can observe payments reactively. That refresh is the ONLY path
 * by which payment rows become durable; the recurring DashPay sweep
 * reconciles payments in-memory without persisting them.
 *
 * The source `PaymentEntry` carries no timestamp field (history is keyed
 * by txid, no wall-clock time recorded), so none is persisted —
 * [createdAt] / [lastUpdated] are local row bookkeeping, not payment
 * dates.
 *
 * [ownerIdentityId] materializes the NON-optional `owner` relationship
 * with CASCADE (Swift `PersistentIdentity.dashpayPayments` declares
 * `.cascade`).
 */
@Entity(
    tableName = "dashpay_payments",
    primaryKeys = ["networkRaw", "ownerIdentityId", "txid"],
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
data class DashpayPaymentEntity(
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** Owning (wallet-managed) identity's 32-byte id. */
    val ownerIdentityId: ByteArray,
    /**
     * The other identity in this payment
     * (`DashpayPaymentFFI::counterparty_id`). Whether they are the sender
     * or the receiver is encoded in [directionRaw].
     */
    val counterpartyIdentityId: ByteArray,
    /** Amount in duffs; always positive, [directionRaw] carries the sign. Swift `UInt64` → [Long]. */
    val amountDuffs: Long,
    /** Raw `DashPayPaymentDirection` value (0 = sent, 1 = received); Swift `UInt8` → [Int]. */
    val directionRaw: Int,
    /** Raw `DashPayPaymentStatus` value (0 = pending, 1 = confirmed, 2 = failed); Swift `UInt8` → [Int]. */
    val statusRaw: Int,
    /** Transaction id (hex), the Rust `dashpay_payments` map key. */
    val txid: String,
    /** Sender memo, when present; null mirrors the source `Option` being `None`. */
    val memo: String? = null,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
