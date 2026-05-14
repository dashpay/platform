import Foundation
import DashSDKFFI

/// Key derivation utilities
public class KeyDerivation {

    /// Get identity authentication path
    /// - Parameters:
    ///   - network: The network type
    ///   - identityIndex: The identity index
    ///   - keyIndex: The key index
    /// - Returns: The derivation path string
    public static func getIdentityAuthenticationPath(network: Network = .mainnet,
                                                    identityIndex: UInt32,
                                                    keyIndex: UInt32) throws -> String {
        var pathBuffer: UnsafeMutablePointer<CChar>? = nil
        let rc = platform_wallet_derive_identity_authentication_path(
            network.ffiValue, identityIndex, keyIndex, &pathBuffer)

        guard rc == 0, let pathBuffer else {
            throw PlatformWalletError.walletOperation(
                "platform_wallet_derive_identity_authentication_path returned \(rc)")
        }
        defer { platform_wallet_string_free(pathBuffer) }

        return String(cString: pathBuffer)
    }
}
