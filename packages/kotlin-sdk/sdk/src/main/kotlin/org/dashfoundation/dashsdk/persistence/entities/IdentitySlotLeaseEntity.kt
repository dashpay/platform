package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity

/**
 * Port of `PersistentIdentitySlotLease.swift`: the identity index an
 * invitation claim holds before its identity lands, keyed by the
 * invitation's asset-lock outpoint. Written by native before any key is
 * derived at that index, so a claim retried after a relaunch returns to the
 * same slot and a registration never takes it. Every field is opaque here
 * and handed back as it came.
 *
 * Keyed by the network-scoped wallet id; no foreign key, so the wallet
 * deletion purges it explicitly (`deleteWalletDataLocked`).
 */
@Entity(
    tableName = "identity_slot_leases",
    primaryKeys = ["walletId", "invitationOutPoint"],
)
data class IdentitySlotLeaseEntity(
    val walletId: ByteArray,
    /** 36 bytes: raw txid, then the little-endian vout. */
    val invitationOutPoint: ByteArray,
    /** The identity index the claim derives its keys at; a `u32` bit pattern. */
    val identityIndex: Int,
    /** The native lease phase code (`IDENTITY_SLOT_LEASE_PHASE_*`), 0..255. */
    val phase: Int,
)
