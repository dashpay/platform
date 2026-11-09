package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity

/**
 * Port of `PersistentPrivateContactRequestsPosition.swift`: where one of the
 * wallet's identities reads DashPay's private contact request store from
 * next, so a relaunch resumes the private contact request sync instead of
 * reading the store from its start. Written in the same changeset round as
 * the contact requests read before it.
 *
 * Keyed by the network-scoped wallet id; no foreign key, so the wallet
 * deletion purges it explicitly (`deleteWalletDataLocked`), and the round
 * that removes the identity clears it (`clearedIdentities`).
 */
@Entity(
    tableName = "private_contact_requests_positions",
    primaryKeys = ["walletId", "identityId"],
)
data class PrivateContactRequestsPositionEntity(
    val walletId: ByteArray,
    val identityId: ByteArray,
    /** The store position read from next; a `u64` bit pattern. */
    val position: Long,
    /** Fingerprint of the scanning keys [position] holds for; opaque here. */
    val keyFingerprint: ByteArray,
)

/**
 * Port of `PersistentPrivateShieldedAddress.swift`: the shielded address a
 * private contact's tracked request to one of the wallet's identities
 * carries, kept so a relaunch has it without reading the private store again.
 */
@Entity(
    tableName = "private_shielded_addresses",
    primaryKeys = ["walletId", "ownerId", "contactId"],
)
data class PrivateShieldedAddressEntity(
    val walletId: ByteArray,
    /** The wallet-owned identity the request is to. */
    val ownerId: ByteArray,
    /** The contact who sent the request. */
    val contactId: ByteArray,
    /** The raw 43-byte Orchard payment address. */
    val address: ByteArray,
)
