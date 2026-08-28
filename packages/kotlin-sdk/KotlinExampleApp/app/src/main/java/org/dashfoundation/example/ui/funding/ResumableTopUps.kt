package org.dashfoundation.example.ui.funding

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.flowOf
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.example.navigation.FundFromAssetLock
import org.dashfoundation.example.navigation.ShieldedFund
import org.dashfoundation.example.util.hexToBytes

/**
 * The two `AssetLockFundingType` discriminants that own the "Pending
 * Platform Top Ups" recovery surface.
 *
 * The full domain is `0 IdentityRegistration, 1 IdentityTopUp,
 * 2 IdentityTopUpNotBound, 3 IdentityInvitation, 4 AssetLockAddressTopUp,
 * 5 AssetLockShieldedAddressTopUp`. Types `0..2` recover on the identity
 * screens (`IdentitiesContentView.crossWalletResumableLocks` and its Kotlin
 * counterpart, both of which admit only `0..2`); `3` is a bearer voucher
 * consumed exclusively by the invitation reclaim flow. That leaves `4` and
 * `5` with no recovery home other than this surface — and `5` had none at
 * all until it was added here.
 */
internal const val FUNDING_TYPE_ADDRESS_TOP_UP = 4
internal const val FUNDING_TYPE_SHIELDED_ADDRESS_TOP_UP = 5

/**
 * Funding types the top-up recovery surface observes, in render order.
 *
 * This list is the whole point of `AssetLockDao.observeResumableTopUpsByFundingType`
 * existing: the older `observeResumableAddressTopUps` hardcodes
 * `fundingTypeRaw = 4`, so a stalled or `RecoveredFromChain` SHIELDED
 * top-up (`5`) was returned by no query any screen ran. Widening the
 * status predicate alone did not fix that — a row invisible on the
 * funding-type axis stays invisible however generous the status axis is.
 */
internal val RESUMABLE_TOP_UP_FUNDING_TYPES = listOf(
    FUNDING_TYPE_ADDRESS_TOP_UP,
    FUNDING_TYPE_SHIELDED_ADDRESS_TOP_UP,
)

/**
 * Cross-wallet, cross-funding-type stream of resumable orphan top-up locks,
 * each tagged with the hex wallet id that owns it (needed for the Resume
 * navigation, which is wallet-scoped while the Identities tab is not).
 *
 * [observe] is the DAO seam — production passes
 * `AssetLockDao::observeResumableTopUpsByFundingType`, which applies the
 * recoverable-status predicate (`[1,3] ∪ {5}`) SQL-side. Taking it as a
 * parameter keeps this function a pure combinator: the funding-type fan-out
 * that the blocker was about is then assertable without a Room database or
 * a Compose runtime.
 *
 * Emissions are ordered by (wallet, funding type) so the rendered list
 * doesn't reshuffle between recompositions.
 */
internal fun resumableTopUpsAcrossWallets(
    walletIdHexes: List<String>,
    observe: (walletId: ByteArray, fundingTypeRaw: Int) -> Flow<List<AssetLockEntity>>,
): Flow<List<Pair<String, AssetLockEntity>>> {
    // `combine` over an empty source list never emits, which would leave the
    // section stuck on its initial value instead of resolving to "nothing to
    // recover". Short-circuit to an explicit empty emission.
    if (walletIdHexes.isEmpty()) return flowOf(emptyList())

    val slots: List<Pair<String, Int>> = walletIdHexes.flatMap { hex ->
        RESUMABLE_TOP_UP_FUNDING_TYPES.map { fundingType -> hex to fundingType }
    }
    val flows = slots.map { (hex, fundingType) -> observe(hex.hexToBytes(), fundingType) }
    return combine(flows) { emissions ->
        slots.zip(emissions.toList()).flatMap { (slot, locks) ->
            val (hex, _) = slot
            locks.map { hex to it }
        }
    }
}

/**
 * The screen a Resume tap on an orphan top-up row must open, or `null` when
 * the row's funding type has no resume flow on this surface.
 *
 * Routing on `fundingTypeRaw` is the second half of the fix: surfacing a
 * type-5 row is useless if its Resume lands on
 * [org.dashfoundation.example.ui.funding.FundFromAssetLockScreen], whose
 * submit calls `resumeFundFromAssetLock` — the platform-ADDRESS resume. A
 * shielded lock has to reach `shieldedResumeFundFromAssetLock` instead,
 * which is what [ShieldedFund] in resume mode does.
 *
 * Fail-closed on anything else: identity-family types (`0..3`) recover on
 * the identity screens, and an unknown discriminant must not be dispatched
 * to a resume flow that would mis-handle it.
 */
internal fun resumeRouteFor(walletIdHex: String, lock: AssetLockEntity): Any? =
    when (lock.fundingTypeRaw) {
        FUNDING_TYPE_ADDRESS_TOP_UP -> FundFromAssetLock(
            walletIdHex = walletIdHex,
            resumeOutPointHex = lock.outPointHex,
        )
        FUNDING_TYPE_SHIELDED_ADDRESS_TOP_UP -> ShieldedFund(
            walletIdHex = walletIdHex,
            resumeOutPointHex = lock.outPointHex,
        )
        else -> null
    }
