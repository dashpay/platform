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
        try ownedString("platform_wallet_identity_authentication_path") { out in
            try platform_wallet_identity_authentication_path(
                network.ffiValue, identityIndex, keyIndex, &out
            ).check()
        }
    }

    /// Throws when `value` contains a NUL. The C string would end there, so
    /// the FFI would answer for the prefix.
    static func requireNoNUL(_ value: String, _ name: String) throws {
        guard !value.utf8.contains(0) else {
            throw PlatformWalletError.invalidParameter("\(name) must not contain NUL")
        }
    }

    /// Runs `call` and returns the string it wrote to its out pointer, freed
    /// with `free`.
    static func ownedString(
        _ function: String,
        free: (UnsafeMutablePointer<CChar>?) -> Void = platform_wallet_free_string,
        _ call: (inout UnsafeMutablePointer<CChar>?) throws -> Void
    ) throws -> String {
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        try call(&outPtr)
        guard let ptr = outPtr else {
            throw PlatformWalletError.nullPointer("\(function) returned a NULL string pointer")
        }
        defer { free(ptr) }
        return String(cString: ptr)
    }
}
