import Foundation

/// The single wallet signing-material probe behind every `isLocal`
/// promotion gate (persister upsert, startup heal, key-removal
/// recompute). Tri-state on purpose:
/// - `true`  — the wallet's mnemonic item is present; its identity
///   keys are signable via the resolver (derive-sign-destroy).
/// - `false` — the Keychain answered definitively that no mnemonic
///   exists: a genuine watch-only wallet. Ownership may still be
///   recorded, but identities must not present as "Local".
/// - `nil`   — the Keychain could not answer (device locked, daemon
///   unavailable). Callers MUST preserve their current state and let
///   a later pass decide — persisting a conclusion drawn from a
///   transient failure is how false Observed/Local classifications
///   are minted.
///
/// Deliberately an approximation of per-identity capability: imported
/// scalar keys are covered by the key-material arms of `isLocal`, not
/// this wallet arm, and full seed-binding verification
/// (`verifySeedBinding`) is async wallet-manager machinery that runs
/// at unlock and gates actual signing — this probe only gates the
/// persisted classification.
public enum WalletSigningCapability {
    /// `verifiedBindingMarker` is the wallet row's
    /// `seedBindingVerifiedMarker` — persisted only after
    /// `verifySeedBinding` ran a FULL verification that bound the
    /// mnemonic item to this wallet's account-0 xpub. Mere item
    /// presence is not capability: a mnemonic can exist without
    /// binding to this wallet (restored backups, replaced items), and
    /// promoting on presence alone would expose mutation controls
    /// that actual signing rejects. A present-but-never-verified
    /// mnemonic therefore DEFERS (`nil`) — the first successful
    /// unlock writes the marker and promotion follows; a binding
    /// MISMATCH clears the marker and demotes via
    /// `handleSeedBindingMismatch`.
    public static func probe(
        walletId: Data,
        verifiedBindingMarker: String?
    ) -> Bool? {
        switch WalletStorage().mnemonicAvailability(for: walletId) {
        case .present:
            return verifiedBindingMarker != nil ? true : nil
        case .absent:
            return false
        case .unavailable:
            return nil
        }
    }
}
