import Foundation

// MARK: - Signer Protocol

/// Legacy Swift-side `Signer` protocol.
///
/// `KeychainSigner` is the production implementation — it is also
/// what every FFI `_with_signer` entry point expects via its
/// `.handle` property. Signing itself happens entirely through that
/// handle (the FFI signing path); the protocol only exposes the
/// can-sign capability check.
///
/// New code should depend on `KeychainSigner` directly and pass
/// `signer.handle` to FFI; this protocol does not (and cannot)
/// participate in the FFI signing path.
public protocol Signer: Sendable {
    /// Check if this signer can sign for the given public key.
    /// - Parameter identityPublicKey: The public key data to check.
    /// - Returns: true if the signer has the corresponding private key.
    func canSign(identityPublicKey: Data) -> Bool
}
