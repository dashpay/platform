// ShieldedEngineBindPlan.swift
// SwiftExampleApp
//
// Pure iteration seam for the multi-wallet shielded engine-bind that
// `SwiftExampleAppApp.rebindWalletScopedServices()` performs. Extracted
// so the "engine-bind every OTHER loaded wallet" contract can be
// unit-tested without a configured `PlatformWalletManager` (whose
// `bindEngine` path calls into FFI and needs a live handle).

import Foundation

/// Invoke `bindEngine` once for every wallet id in `allWalletIds`
/// EXCEPT `mirrorWalletId` (the app-level `firstWallet`, which is
/// engine-bound separately via `ShieldedService.bind(...)` when the UI
/// mirror is attached).
///
/// The per-id closure is best-effort and independent: it must not
/// throw (each `ShieldedService.bindEngine` already swallows its own
/// errors), but even if a caller passes a throwing closure — as the
/// tests do to simulate one wallet's missing mnemonic — a failure for
/// one id must NOT stop the remaining ids from being bound. Each thrown
/// error is caught and dropped so the loop always visits every
/// non-mirror wallet.
///
/// Order is not significant to the coordinator (it iterates
/// registrations each sync tick), so the caller may pass ids in any
/// order.
///
/// `@MainActor`-isolated because the only caller
/// (`rebindWalletScopedServices`) is, and the `bindEngine` closure it
/// passes touches `@MainActor` `ShieldedService` state — keeping the
/// helper on the main actor lets that call be synchronous with no
/// actor hop under Swift 6 strict concurrency.
@MainActor
func engineBindOtherWallets(
    allWalletIds: some Sequence<Data>,
    mirrorWalletId: Data,
    bindEngine: (Data) throws -> Void
) {
    for walletId in allWalletIds where walletId != mirrorWalletId {
        // Independent + best-effort: one id's failure can't block the
        // rest. `ShieldedService.bindEngine` never throws in production;
        // the `try?`-style catch here is belt-and-braces for the test
        // seam and any future throwing binder.
        try? bindEngine(walletId)
    }
}
