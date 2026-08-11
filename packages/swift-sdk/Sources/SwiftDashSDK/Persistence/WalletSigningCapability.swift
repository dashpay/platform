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
    public static func probe(walletId: Data) -> Bool? {
        switch WalletStorage().mnemonicAvailability(for: walletId) {
        case .present:
            return true
        case .absent:
            return false
        case .unavailable:
            return nil
        }
    }
}
