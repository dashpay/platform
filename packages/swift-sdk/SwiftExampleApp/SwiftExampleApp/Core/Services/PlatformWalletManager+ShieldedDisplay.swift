// PlatformWalletManager+ShieldedDisplay.swift
// SwiftExampleApp
//
// App-side display convenience over the per-wallet shielded engine
// lookup. Kept in the app (not the SDK) because it composes two SDK
// calls for UI display; the SDK stays persist/load/bridge-only.

import Foundation
import SwiftDashSDK

extension PlatformWalletManager {
    /// Bech32m-encoded default Orchard payment address for `walletId`
    /// (`account`), or `nil` when the wallet has no engine-bound
    /// shielded sub-wallet yet (or the wallet is unknown to this
    /// manager). Single home for the resolve-then-encode pattern the
    /// per-wallet shielded surfaces share.
    func shieldedDisplayAddress(
        walletId: Data,
        account: UInt32 = 0,
        network: Network
    ) -> String? {
        guard let raw = try? shieldedDefaultAddress(
            walletId: walletId,
            account: account
        ) else { return nil }
        return DashAddress.encodeOrchard(rawBytes: raw, network: network)
    }
}
