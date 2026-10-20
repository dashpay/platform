package org.dashfoundation.dashsdk.services

/**
 * Pure iteration seam for the multi-wallet shielded engine-bind that
 * `AppContainer.rebindWalletScopedServices()` (and
 * [ShieldedService.clearLocalState]'s recovery pass) perform — port of
 * `ShieldedEngineBindPlan.swift`. Extracted so the "engine-bind every
 * OTHER loaded wallet" contract can be unit-tested without a configured
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager] (whose
 * [ShieldedService.bindEngine] path calls into JNI and needs a live
 * handle).
 *
 * Invokes [bindEngine] once for every wallet id in [allWalletIds] EXCEPT
 * [mirrorWalletId] (the app-level first wallet, which is engine-bound
 * separately via [ShieldedService.bind] when the UI mirror is attached).
 *
 * The per-id action is best-effort and independent: it must not throw
 * (each [ShieldedService.bindEngine] already swallows its own errors),
 * but even if a caller passes a throwing action — as the tests do to
 * simulate one wallet's missing mnemonic — a failure for one id must NOT
 * stop the remaining ids from being bound. Each thrown error is caught
 * and dropped so the loop always visits every non-mirror wallet.
 *
 * Order is not significant to the coordinator (it iterates registrations
 * each sync tick), so the caller may pass ids in any order. Ids are
 * compared with `==` — callers pass the wallet map's hex-string keys,
 * where string equality is byte equality.
 */
suspend fun <K> engineBindOtherWallets(
    allWalletIds: Collection<K>,
    mirrorWalletId: K,
    bindEngine: suspend (K) -> Unit,
) {
    for (walletId in allWalletIds) {
        if (walletId == mirrorWalletId) continue
        // Independent + best-effort: one id's failure can't block the rest.
        // ShieldedService.bindEngine never throws in production; the catch
        // here is belt-and-braces for the test seam and any future throwing
        // binder.
        try {
            bindEngine(walletId)
        } catch (_: Exception) {
            // Dropped by contract — the binder logs its own failures.
        }
    }
}
