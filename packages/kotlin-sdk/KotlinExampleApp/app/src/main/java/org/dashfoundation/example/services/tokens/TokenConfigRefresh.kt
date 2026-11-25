package org.dashfoundation.example.services.tokens

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay

/**
 * Brings the stored contract up to date after a token configuration update
 * (max supply and the other `TokenConfigurationChangeItem`s) took effect.
 *
 * Platform applies the change to the contract and raises its version by
 * one, but the update's result carries only the token history document, not
 * the contract. So the stored contract, and the token rows materialized from
 * it, keep the old configuration until the contract is fetched again. This
 * fetches it through the caller's [refresh] (`ContractDownloader
 * .refreshPersistedContract`), which verifies the proof and refuses a
 * version below the floor it is given.
 *
 * The floor is one above the stored version, the version the update
 * produced when the stored contract was current. A node that has not seen
 * the update's block yet returns the older version and is refused, so the
 * refresh is tried again a few times before giving up.
 */
object TokenConfigRefresh {

    const val ATTEMPTS = 3
    const val RETRY_DELAY_MS = 2_000L

    /**
     * Refresh after a confirmed update, given the version stored before it.
     * Returns null once the refresh succeeded, or the last failure after
     * [attempts] tries. Does nothing (returns null) without a stored version:
     * a token row exists only under a stored contract.
     */
    suspend fun afterConfirmedUpdate(
        storedVersion: Int?,
        attempts: Int = ATTEMPTS,
        retryDelayMs: Long = RETRY_DELAY_MS,
        pause: suspend (Long) -> Unit = { delay(it) },
        refresh: suspend (minimumVersion: Long) -> Unit,
    ): Exception? {
        val floor = (storedVersion ?: return null).toLong() + 1
        var failure: Exception? = null
        repeat(attempts) { attempt ->
            if (attempt > 0) pause(retryDelayMs)
            try {
                refresh(floor)
                return null
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                failure = e
            }
        }
        return failure
    }
}
