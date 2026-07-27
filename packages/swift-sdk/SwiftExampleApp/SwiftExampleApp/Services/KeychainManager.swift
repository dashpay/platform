import Foundation
import SwiftDashSDK

/// Shim that used to pin the app to the legacy
/// `com.dash.swiftexampleapp.keys` service name. The app now uses
/// the same unified `org.dashfoundation.wallet` service as
/// `WalletStorage` and every other SDK keychain writer, so this
/// wrapper just forwards to the SDK shared instance — the separate
/// type name is kept so existing call sites (`KeychainManager.shared`
/// inside the app) keep compiling without a global rename.
///
/// The legacy service's contents are wiped on launch by
/// `WalletStorage.cleanupLegacyItems()`; don't expect anything
/// stored under the old name to survive.
@MainActor
enum KeychainManager {
    /// Forwards to `SwiftDashSDK.KeychainManager.shared`, which is
    /// now the single app-wide instance backed by the unified
    /// `org.dashfoundation.wallet` service.
    static var shared: SwiftDashSDK.KeychainManager {
        SwiftDashSDK.KeychainManager.shared
    }
}

// Re-export SpecialKeyType for backwards compatibility
// (KeychainError is not used in the app, so no need to re-export)
typealias SpecialKeyType = SwiftDashSDK.SpecialKeyType
