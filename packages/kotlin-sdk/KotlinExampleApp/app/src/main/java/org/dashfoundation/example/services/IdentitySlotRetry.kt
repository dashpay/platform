package org.dashfoundation.example.services

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.identity.IdentityRegistration

/**
 * Runs a registration or an invitation claim at a slot the wallet leased,
 * and moves it to a new slot when the wallet answers
 * [DashSdkError.PlatformWallet.IdentitySlotTakenOnChain]: the keys at that
 * index belong to an identity on Platform (now recorded in the wallet), so
 * nothing was funded and the operation goes to the next slot the wallet
 * leases. The wallet chooses every index; this only follows its answer.
 */
object IdentitySlotRetry {

    /**
     * [attempt] at [first]; on a slot taken on chain, [reacquire] a slot
     * and [attempt] there, at most [limit] times in a row. The last refusal
     * is thrown. [attempt] derives the keys at the slot it gets.
     */
    suspend fun <S, T> run(
        first: S,
        reacquire: suspend () -> S,
        limit: Int = IdentityRegistration.IDENTITY_SLOT_TAKEN_RETRY_LIMIT,
        attempt: suspend (S) -> T,
    ): T {
        var slot = first
        var retries = 0
        while (true) {
            try {
                return attempt(slot)
            } catch (e: DashSdkError.PlatformWallet.IdentitySlotTakenOnChain) {
                if (retries >= limit) throw e
                retries++
                slot = reacquire()
            }
        }
    }
}
