package org.dashfoundation.example.services.tokens

import android.util.Log
import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58

/**
 * Registers an identity's watched tokens with the Rust identity-token sync
 * and runs one sync pass, so `onPersistTokenBalanceUpsert` fills the
 * `token_balances` rows the Tokens list and the token action forms read.
 *
 * Port of the persistence half of `IdentityDetailView.reloadTokenBalances`
 * (SwiftExampleApp `IdentityDetailView.swift`): the watched set is every
 * locally saved token on the active network, each token id comes from the
 * native `calculateTokenId` (the protocol formula is not mirrored here), and
 * a failure is non-fatal, so the next reload tries again. As in Swift, an
 * empty local token list registers nothing.
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
     * Register [identityId] with the watched ids of [tokens] and run one sync
     * pass. Returns false when nothing was registered or the native calls
     * failed; never throws except on cancellation.
     */
    suspend fun refresh(
        identityId: ByteArray,
        tokens: List<TokenEntity>,
        calculateTokenId: suspend (contractIdBase58: String, position: Int) -> String?,
        register: suspend (identityId: ByteArray, tokenIds: List<ByteArray>) -> Unit,
        syncNow: suspend () -> Unit,
    ): Boolean {
        if (tokens.isEmpty()) return false
        val tokenIds = watchedTokenIds(tokens, calculateTokenId)
        if (tokenIds.isEmpty()) return false
        return try {
            register(identityId, tokenIds)
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
