package org.dashfoundation.dashsdk.persistence

import androidx.room.withTransaction
import org.dashfoundation.dashsdk.persistence.entities.IdentityIndexStateEntity

/** Transactional persistence seam for app-level identity-slot coordination. */
interface IdentityIndexReservationStore {
    suspend fun nextSafeIndex(walletId: ByteArray): Int
    suspend fun reserveNext(walletId: ByteArray): Int
    suspend fun reserveFreshExact(walletId: ByteArray, identityIndex: Int)
    suspend fun reserveResumeExact(walletId: ByteArray, identityIndex: Int)
}

/**
 * Room-backed monotonic issued-index guard.
 *
 * This does not derive keys or construct paths. Rust remains the sole source
 * of derivation policy; this store only prevents the app from submitting two
 * operations that reuse the same caller-selected DIP-9 slot.
 */
class RoomIdentityIndexReservationStore(
    private val database: DashDatabase,
) : IdentityIndexReservationStore {
    override suspend fun nextSafeIndex(walletId: ByteArray): Int = database.withTransaction {
        nextAfter(currentFloor(walletId))
    }

    override suspend fun reserveNext(walletId: ByteArray): Int = database.withTransaction {
        val next = nextAfter(currentFloor(walletId))
        database.identityIndexStateDao().upsert(IdentityIndexStateEntity(walletId, next))
        next
    }

    override suspend fun reserveFreshExact(walletId: ByteArray, identityIndex: Int) {
        require(identityIndex >= 0) { "identityIndex must be non-negative" }
        database.withTransaction {
            val floor = currentFloor(walletId)
            check(identityIndex > floor) {
                "Identity index $identityIndex was already reserved and cannot be reused. " +
                    "Choose index ${nextAfter(floor)} or later."
            }
            database.identityIndexStateDao().upsert(
                IdentityIndexStateEntity(walletId, identityIndex),
            )
        }
    }

    override suspend fun reserveResumeExact(walletId: ByteArray, identityIndex: Int) {
        require(identityIndex >= 0) { "identityIndex must be non-negative" }
        database.withTransaction {
            check(!database.identityDao().existsByWalletAndIdentityIndex(walletId, identityIndex)) {
                "Identity index $identityIndex already belongs to a registered identity."
            }
            val floor = currentFloor(walletId)
            if (identityIndex > floor) {
                database.identityIndexStateDao().upsert(
                    IdentityIndexStateEntity(walletId, identityIndex),
                )
            }
        }
    }

    private suspend fun currentFloor(walletId: ByteArray): Int = listOfNotNull(
        database.identityIndexStateDao().get(walletId)?.lastIssuedIndex,
        database.identityDao().maxIdentityIndex(walletId),
        database.assetLockDao().maxIdentityRegistrationIndex(walletId),
    ).maxOrNull() ?: -1

    private fun nextAfter(floor: Int): Int {
        check(floor < Int.MAX_VALUE) { "No unused Kotlin identity index remains." }
        return floor + 1
    }
}
