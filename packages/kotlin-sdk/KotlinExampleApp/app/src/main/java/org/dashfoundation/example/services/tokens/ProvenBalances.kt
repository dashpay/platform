package org.dashfoundation.example.services.tokens

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.first
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.dao.IdentityDao
import org.dashfoundation.dashsdk.persistence.dao.TokenDao
import org.dashfoundation.dashsdk.persistence.UInt64Value
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import java.util.Date

/**
 * Best-effort persist of the proof-verified post-action balances a token
 * transfer / burn broadcast already returned — the Android counterpart of
 * `SDK.persistProvenTokenBalances` (used by
 * `TokenTransferActionView.persistBalancesAfterTransfer` and the burn
 * flows). Balances JSON shape:
 * `{"<identityIdBase58>": "<balanceDecimalString>"}`
 * (rs-platform-wallet-ffi/src/tokens/balances_json.rs).
 *
 * Existing `token_balances` rows (matched on the token relationship key +
 * identity id) are updated in place. When no local row exists yet for an
 * identity, a fresh row is created using the canonical on-chain token id
 * from the now-bridged `calculateTokenId` (mirroring
 * `SDK.persistProvenTokenBalances`, which keys its `PersistentTokenBalance`
 * rows by that same canonical id). If [sdk]/[networkRaw] are not supplied,
 * or the token-id calculation fails, row *creation* is skipped and the
 * Rust-driven periodic balance sync remains the backstop.
 *
 * The row's `identityRef` is only populated when [identityDao] confirms a
 * local `identities` row for that id — recipients picked via DPNS search or
 * typed in by hand need not exist locally, and `identityRef` is a Room
 * foreign key, so setting it unconditionally would throw a constraint
 * violation *after* the transfer/mint already broadcast. `identityId` is
 * always kept as the denormalized scalar so the balance stays queryable
 * regardless.
 *
 * Per-entry DAO failures are swallowed rather than thrown: this call always
 * runs after the on-chain action has already broadcast, so surfacing a
 * cache-write error to the caller would mark an already-executed transfer /
 * mint / burn as failed-and-retryable, risking a duplicate submission. The
 * periodic Rust-driven balance sync remains the backstop for any row this
 * skips.
 */
object ProvenBalances {

    suspend fun persist(
        balancesJson: String?,
        token: TokenEntity,
        dao: TokenDao,
        sdk: Sdk? = null,
        networkRaw: Int? = null,
        identityDao: IdentityDao? = null,
    ) {
        if (balancesJson.isNullOrBlank()) return
        val entries = try {
            LenientJson.parseToJsonElement(balancesJson).jsonObject
        } catch (_: Exception) {
            return
        }
        if (entries.isEmpty()) return

        // Canonical base58 token id — the row key for freshly created rows.
        // Computed once, only when we may need to create rows.
        val canonicalTokenId: String? =
            if (sdk != null && networkRaw != null) {
                runCatching {
                    sdk.tokenQueries.calculateTokenId(
                        Base58.encode(token.contractId), token.position,
                    )
                }.getOrNull()
            } else {
                null
            }

        val now = Date()
        for ((idBase58, amountElement) in entries) {
            val identityId = Base58.decodeIdentifier(idBase58) ?: continue
            // u64 decimal string → unsigned Room value.
            val balance = (amountElement as? JsonPrimitive)?.content
                ?.toULongOrNull()?.let(::UInt64Value) ?: continue
            try {
                val existing = dao.observeBalancesByIdentity(identityId).first()
                    .firstOrNull { it.tokenRef?.contentEquals(token.id) == true }
                if (existing != null) {
                    dao.updateBalance(
                        existing.copy(balance = balance, lastUpdated = now, lastSyncedAt = now),
                    )
                } else if (canonicalTokenId != null && networkRaw != null) {
                    val identityRef = identityDao?.getByIdentityId(identityId)?.let { identityId }
                    dao.insertBalance(
                        TokenBalanceEntity(
                            tokenId = canonicalTokenId,
                            identityId = identityId,
                            balance = balance,
                            createdAt = now,
                            lastUpdated = now,
                            lastSyncedAt = now,
                            tokenName = token.name,
                            tokenDecimals = token.decimals,
                            networkRaw = networkRaw,
                            identityRef = identityRef,
                            tokenRef = token.id,
                        ),
                    )
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                // Best-effort: see the "Per-entry DAO failures" note above.
            }
        }
    }
}
