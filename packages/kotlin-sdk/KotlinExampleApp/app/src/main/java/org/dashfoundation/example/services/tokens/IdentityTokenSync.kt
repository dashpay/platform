package org.dashfoundation.example.services.tokens

import android.util.Log
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58

/**
 * Registers the identity on screen and its watched tokens with the Rust
 * identity-token sync and runs one sync pass, so `onPersistTokenBalanceUpsert`
 * fills the `token_balances` rows the Tokens list and the token action forms
 * read.
 *
 * Port of the persistence half of `IdentityDetailView.reloadTokenBalances`
 * (SwiftExampleApp `IdentityDetailView.swift`): the watched set is every
 * locally saved token on the active network, each token id comes from the
 * native `calculateTokenId` (the protocol formula is not mirrored here), and
 * a failure is non-fatal, so the next reload tries again. Like Swift it syncs
 * only the identity on screen (the Tokens selection, or the acting identity
 * on a token's action screen). Unlike Swift it also registers the wallet's
 * other identities with no tokens, so a pass reads only the identity on
 * screen and a token whose contract was removed stops being watched for every
 * identity.
 *
 * Screens launch it in the application scope, so leaving the screen (for
 * example opening Transfer) does not cancel it before its sync pass. Calls
 * run one at a time in call order, so the latest screen's registration is
 * the one left in place. Removing an identity from the device runs under the
 * same lock ([withWatchLock]), and a queued refresh checks that its identity
 * is still stored before registering it, so a removed identity is not
 * watched again.
 */
object IdentityTokenSync {

    private const val TAG = "IdentityTokenSync"

    /** Serializes registrations and passes, first come first served. */
    private val mutex = Mutex()

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
     * Register [identityId] with the watched ids of [tokens], register each of
     * [otherIdentityIds] with none (stopping their watches), and, when a token
     * is watched, run one sync pass. Does nothing when [isStillStored] says
     * the identity was removed while this call was queued, and leaves out
     * tokens [isTokenStillSaved] says were deleted meanwhile (both checked
     * under the lock contract deletion and identity removal take). Returns
     * true when a sync pass ran; false when there was nothing to sync or the
     * native calls failed. Never throws except on cancellation.
     */
    suspend fun refresh(
        identityId: ByteArray,
        otherIdentityIds: List<ByteArray>,
        tokens: List<TokenEntity>,
        calculateTokenId: suspend (contractIdBase58: String, position: Int) -> String?,
        register: suspend (identityId: ByteArray, tokenIds: List<ByteArray>) -> Unit,
        syncNow: suspend () -> Unit,
        isStillStored: suspend () -> Boolean = { true },
        isTokenStillSaved: suspend (TokenEntity) -> Boolean = { true },
    ): Boolean = mutex.withLock {
        try {
            if (!isStillStored()) return@withLock false
            val tokenIds = watchedTokenIds(tokens.filter { isTokenStillSaved(it) }, calculateTokenId)
            for (other in otherIdentityIds) {
                if (!other.contentEquals(identityId)) register(other, emptyList())
            }
            register(identityId, tokenIds)
            if (tokenIds.isEmpty()) return@withLock false
            syncNow()
            true
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            runCatching { Log.w(TAG, "identity token sync failed", e) }
            false
        }
    }

    /**
     * Register each of [identityIds] with no tokens, without a sync pass, and
     * then, still under the lock, run [then]. Contract deletion deletes the
     * contract in [then]: a pass already reading the contract's tokens cannot
     * store their balances after the delete (platform-wallet stores a result
     * only for tokens still watched), and a refresh queued meanwhile runs
     * after the delete and leaves the deleted tokens out. The next refresh
     * registers the identity on screen again. Returns false, without running
     * [then], when registration failed.
     */
    suspend fun clear(
        identityIds: List<ByteArray>,
        register: suspend (identityId: ByteArray, tokenIds: List<ByteArray>) -> Unit,
        then: suspend () -> Unit = {},
    ): Boolean = mutex.withLock {
        val cleared = try {
            for (identityId in identityIds) register(identityId, emptyList())
            true
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            runCatching { Log.w(TAG, "identity token watch clear failed", e) }
            false
        }
        if (cleared) then()
        cleared
    }

    /**
     * Run [block] under the lock refreshes take, so it is ordered against
     * them: identity removal runs here, and a refresh queued behind it then
     * sees the identity gone.
     */
    suspend fun <T> withWatchLock(block: suspend () -> T): T = mutex.withLock { block() }
}
