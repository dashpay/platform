package org.dashfoundation.dashsdk.wallet

/**
 * Kotlin mirror of platform-wallet-ffi's `FFISpvSyncProgress` — port of
 * Swift's `PlatformSpvSyncProgress` / `PlatformSpvSubProgress`.
 *
 * Populated from the two flat arrays [WalletManagerNative.spvSyncProgress]
 * fills (see [fromNative] for the field order). A missing sub-phase
 * (`has_* == false`) maps to a null [SpvSubProgress].
 */
data class SpvSyncProgressData(
    val overallState: SpvSyncState,
    val overallPercentage: Double,
    val headers: SpvSubProgress?,
    val filterHeaders: SpvSubProgress?,
    val filters: SpvSubProgress?,
    val masternodes: SpvSubProgress?,
) {
    /** Whether any phase is actively syncing / connecting. */
    val isSyncing: Boolean
        get() = overallState == SpvSyncState.SYNCING ||
            overallState == SpvSyncState.WAITING_FOR_CONNECTIONS

    companion object {
        /** The all-zero progress before the first successful poll. */
        val EMPTY: SpvSyncProgressData = SpvSyncProgressData(
            overallState = SpvSyncState.WAIT_FOR_EVENTS,
            overallPercentage = 0.0,
            headers = null,
            filterHeaders = null,
            filters = null,
            masternodes = null,
        )

        /**
         * Reconstruct from the flat arrays filled by
         * [WalletManagerNative.spvSyncProgress].
         *
         * `longs` (`LongArray(17)`): `[overallState, hasHeaders, headersState,
         * headersCurrent, headersTarget, hasFilterHeaders, filterHeadersState,
         * filterHeadersCurrent, filterHeadersTarget, hasFilters, filtersState,
         * filtersCurrent, filtersTarget, hasMasternodes, masternodesState,
         * masternodesCurrent, masternodesTarget]` (bools as 0/1).
         *
         * `percentages` (`DoubleArray(5)`): `[overall, headers, filterHeaders,
         * filters, masternodes]`.
         */
        fun fromNative(longs: LongArray, percentages: DoubleArray): SpvSyncProgressData {
            fun l(i: Int): Long = longs.getOrElse(i) { 0L }
            fun p(i: Int): Double = percentages.getOrElse(i) { 0.0 }
            fun sub(hasIdx: Int, stateIdx: Int, curIdx: Int, tgtIdx: Int, pctIdx: Int): SpvSubProgress? =
                if (l(hasIdx) != 0L) {
                    SpvSubProgress(
                        state = SpvSyncState.fromRaw(l(stateIdx)),
                        currentHeight = l(curIdx),
                        targetHeight = l(tgtIdx),
                        percentage = p(pctIdx),
                    )
                } else {
                    null
                }
            return SpvSyncProgressData(
                overallState = SpvSyncState.fromRaw(l(0)),
                overallPercentage = p(0),
                headers = sub(1, 2, 3, 4, 1),
                filterHeaders = sub(5, 6, 7, 8, 2),
                filters = sub(9, 10, 11, 12, 3),
                masternodes = sub(13, 14, 15, 16, 4),
            )
        }
    }
}

/** One SPV sub-phase's progress. */
data class SpvSubProgress(
    val state: SpvSyncState,
    val currentHeight: Long,
    val targetHeight: Long,
    val percentage: Double,
)

/**
 * SPV sync state — mirror of `dash-spv`'s `SyncState`, whose u32 values the
 * FFI exposes as `SPV_SYNC_STATE_*` constants.
 */
enum class SpvSyncState(val raw: Long) {
    WAIT_FOR_EVENTS(0),
    WAITING_FOR_CONNECTIONS(1),
    SYNCING(2),
    SYNCED(3),
    ERROR(4);

    companion object {
        fun fromRaw(raw: Long): SpvSyncState =
            entries.firstOrNull { it.raw == raw } ?: WAIT_FOR_EVENTS
    }
}
