import Foundation

/// App-level service for shielded (ZK/Orchard) pool operations.
///
/// Placeholder while shielded support is being moved to the Rust
/// `platform-wallet` crate. Tracks display state only — sync, send,
/// and receive paths are no-ops here and will be wired up against
/// the platform-wallet shielded coordinator in a follow-up PR.
@MainActor
class ShieldedService: ObservableObject {
    @Published var shieldedBalance: UInt64 = 0
    @Published var orchardDisplayAddress: String?
    @Published var isSyncing: Bool = false
    @Published var lastError: String?

    /// Placeholder until the platform-wallet shielded sync coordinator
    /// is wired up. Sets `lastError` so the UI can surface the state.
    func manualSync() {
        lastError = "Shielded sync is being rebuilt — see follow-up PR"
    }

    /// Reset state (e.g., on wallet deletion or logout).
    func reset() {
        shieldedBalance = 0
        orchardDisplayAddress = nil
        isSyncing = false
        lastError = nil
    }
}
