package org.dashfoundation.example.services.tokens

import android.util.Log
import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58

/**
 * Registers the wallet's identities and their watched tokens with the Rust
 * identity-token sync and runs one sync pass, so `onPersistTokenBalanceUpsert`
 * fills the `token_balances` rows the Tokens list and the token action forms
 * read.
 *
 * Port of the persistence half of `IdentityDetailView.reloadTokenBalances`
 * (SwiftExampleApp `IdentityDetailView.swift`): the watched set is every
 * locally saved token on the active network, each token id comes from the
 * native `calculateTokenId` (the protocol formula is not mirrored here), and
 * a failure is non-fatal, so the next reload tries again. Swift registers the
 * one identity on screen; this registers every wallet identity on the
 * network with the same set, so a sender picked on a token's action screen is
 * synced too and a removed contract's token stops being watched for every
 * identity, not only the one selected. An empty saved set clears the watches.
 */
object IdentityTokenSync {

    private const val TAG = "IdentityTokenSync"

    /**
     * Canonical 32-byte token ids for [tokens], deduplicated in first-seen
     * order. Tokens whose position is outside the u16 range, or whose id the
     * native call cannot compute, are skipped, as in Swift.
     */
    suspend fun watchedTokenIds(
        tokens: List<TokenEntity>,
        calculateTokenId: suspend (contractIdBase58: String, position: Int) -> String?,
    ): List<ByteArray> {
        val seen = LinkedHashMap<String, ByteArray>()
        for (token in tokens) {
            if (token.position !in 0..UShort.MAX_VALUE.toInt()) continue
            val tokenIdBase58 = try {
                calculateTokenId(Base58.encode(token.contractId), token.position)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                null
            } ?: continue
            val tokenId = Base58.decodeIdentifier(tokenIdBase58) ?: continue
            seen.putIfAbsent(tokenIdBase58, tokenId)
        }
        return seen.values.toList()
    }

    /**
     * Register each of [identityIds] with the watched ids of [tokens] and,
     * when any token is watched, run one sync pass. With no saved token the
     * identities are registered with none, which stops their watches.
     * Returns true when a sync pass ran; false when there was nothing to sync
     * or the native calls failed. Never throws except on cancellation.
     */
    suspend fun refresh(
        identityIds: List<ByteArray>,
        tokens: List<TokenEntity>,
        calculateTokenId: suspend (contractIdBase58: String, position: Int) -> String?,
        register: suspend (identityId: ByteArray, tokenIds: List<ByteArray>) -> Unit,
        syncNow: suspend () -> Unit,
    ): Boolean {
        if (identityIds.isEmpty()) return false
        val tokenIds = watchedTokenIds(tokens, calculateTokenId)
        return try {
            for (identityId in identityIds) register(identityId, tokenIds)
            if (tokenIds.isEmpty()) return false
            syncNow()
            true
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            runCatching { Log.w(TAG, "identity token sync failed", e) }
            false
        }
    }
}
