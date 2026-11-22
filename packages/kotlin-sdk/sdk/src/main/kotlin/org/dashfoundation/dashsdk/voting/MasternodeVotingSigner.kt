package org.dashfoundation.dashsdk.voting

import org.dashfoundation.dashsdk.security.KeystoreSigner
import org.dashfoundation.dashsdk.wallet.TeardownGate

/**
 * The signer of one masternode vote: a [KeystoreSigner] of its own that holds only the
 * stored voting key of one masternode or evonode identity, under its voting address.
 * `PlatformWalletManager.withStoredMasternodeVotingKey` creates it for one call and frees it
 * when the call ends; pass it to a signer vote (`VoteCasting.castVote`,
 * `DecisionProposals.castVote`, `MultipleChoiceProposals.castVote`). Port of the per-vote
 * `KeychainSigner` Swift builds in `MasternodeVoteCall.run` (`MasternodeVoterSection.swift`).
 *
 * The wallet manager's own signer never resolves the voting address to the voting key, so a
 * vote cannot change which key a concurrent wallet operation signs with, and two votes never
 * share a signer.
 *
 * Holds no key bytes and exposes no native handle. A vote cast with it after its call ended
 * fails with [IllegalStateException].
 */
class MasternodeVotingSigner internal constructor(
    votingAddress: ByteArray,
    internal val signer: KeystoreSigner,
) {
    private val address = votingAddress.copyOf()

    /** Counts the votes in flight, so the signer is freed only after they return. */
    private val gate = TeardownGate()

    @Volatile
    private var ended = false

    /** The masternode's 20-byte voting address: the hash160 of its voting public key. */
    val votingAddress: ByteArray get() = address.copyOf()

    /**
     * Runs [block] with the dedicated signer's native handle, counted against [end] so the
     * signer outlives the call.
     *
     * @throws IllegalStateException once the voter's call has ended.
     */
    internal suspend fun <T> withSignerHandle(block: suspend (signerHandle: Long) -> T): T {
        check(!ended) { ENDED_MESSAGE }
        return try {
            gate.withOp { block(signer.nativeHandle) }
        } catch (e: IllegalStateException) {
            // The gate closed between the check above and the op: report it as the voter's.
            if (ended && e.message == TeardownGate.CLOSED_MESSAGE) {
                throw IllegalStateException(ENDED_MESSAGE, e)
            }
            throw e
        }
    }

    /** Refuse new votes and wait for those in flight; the owner then frees [signer]. */
    internal suspend fun end() {
        ended = true
        gate.closeAndAwait()
    }

    private companion object {
        const val ENDED_MESSAGE =
            "This masternode voting signer was used after its withStoredMasternodeVotingKey " +
                "call ended."
    }
}
