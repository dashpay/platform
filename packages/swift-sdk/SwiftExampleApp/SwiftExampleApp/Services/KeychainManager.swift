import Foundation
import SwiftDashSDK

/// App-specific KeychainManager that uses the legacy service name for data continuity.
/// This ensures existing keys stored under "com.dash.swiftexampleapp.keys" remain accessible.
///
/// New apps should use `SwiftDashSDK.KeychainManager` directly with their own service name.
@MainActor
final class KeychainManager {
    /// Shared instance using the app's legacy service name
    static let shared = SwiftDashSDK.KeychainManager(
        serviceName: "com.dash.swiftexampleapp.keys"
    )
}

// Re-export SpecialKeyType for backwards compatibility
// (KeychainError is not used in the app, so no need to re-export)
typealias SpecialKeyType = SwiftDashSDK.SpecialKeyType
