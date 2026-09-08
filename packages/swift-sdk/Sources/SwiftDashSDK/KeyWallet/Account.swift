import Foundation
import DashSDKFFI

/// Owns a key-wallet account's FFI handle for as long as the caller holds it.
///
/// Deliberately exposes no account-specific operations: it exists so
/// `Wallet.getAccount(type:)` can report whether an account exists (and create
/// it as a side effect), with the handle freed on deinit.
///
/// Key derivation is NOT done here. It carried a
/// `derivePrivateKeyWIF(wallet:masterPath:index:)` that asked callers for the
/// account root path while the FFI applies the account's own path itself, so
/// the path was applied twice and every derived key came from the wrong
/// branch — silently, since the keys were well-formed. Derive instead through:
///
///   * `ManagedPlatformWallet.providerKeyAtIndex(kind:index:includePrivate:)`
///     for the masternode provider families, which resolves the DIP-3 path
///     Rust-side and cross-checks the seed-derived key against the account
///     xpub before returning it, or
///   * `Wallet.derivePrivateKey(path:)` for an explicit full path, where there
///     is no implicit path to apply twice.
public class Account {
    private let handle: OpaquePointer
    private weak var wallet: Wallet?

    internal init(handle: OpaquePointer, wallet: Wallet) {
        self.handle = handle
        self.wallet = wallet
    }

    deinit {
        account_free(handle)
    }

}
