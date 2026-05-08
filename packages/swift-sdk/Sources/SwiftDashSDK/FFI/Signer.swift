import Foundation

// MARK: - Signer Protocol

/// Legacy Swift-side `Signer` protocol.
///
/// `KeychainSigner` is the production implementation — it is also
/// what every FFI `_with_signer` entry point expects via its
/// `.handle` property. The protocol itself is kept (rather than
/// deleted) so callers that hold a generic `any Signer` reference
/// can keep compiling while migration to `KeychainSigner.handle`
/// proceeds.
///
/// New code should depend on `KeychainSigner` directly and pass
/// `signer.handle` to FFI; this protocol does not (and cannot)
/// participate in the FFI signing path.
public protocol Signer: Sendable {
    /// Sign data using the private key corresponding to the given public key.
    /// - Parameters:
    ///   - identityPublicKey: The public key data identifying which private key to use.
    ///   - data: The data to sign.
    /// - Returns: The signature data, or nil if signing failed.
    func sign(identityPublicKey: Data, data: Data) -> Data?

    /// Check if this signer can sign for the given public key.
    /// - Parameter identityPublicKey: The public key data to check.
    /// - Returns: true if the signer has the corresponding private key.
    func canSign(identityPublicKey: Data) -> Bool
}
