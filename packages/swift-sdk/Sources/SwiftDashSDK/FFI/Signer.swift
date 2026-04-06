import Foundation

// MARK: - Signer Protocol

/// Protocol for signing operations
/// Implementations should securely store and retrieve private keys
public protocol Signer: Sendable {
    /// Sign data using the private key corresponding to the given public key
    /// - Parameters:
    ///   - identityPublicKey: The public key data identifying which private key to use
    ///   - data: The data to sign
    /// - Returns: The signature data, or nil if signing failed
    func sign(identityPublicKey: Data, data: Data) -> Data?

    /// Check if this signer can sign for the given public key
    /// - Parameter identityPublicKey: The public key data to check
    /// - Returns: true if the signer has the corresponding private key
    func canSign(identityPublicKey: Data) -> Bool
}
