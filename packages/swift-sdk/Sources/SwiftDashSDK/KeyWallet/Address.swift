import Foundation
import DashSDKFFI

/// Address utilities
public class Address {

    /// Validate an address
    /// - Parameters:
    ///   - address: The address to validate
    ///   - network: The network type
    /// - Returns: True if the address is valid
    public static func validate(_ address: String, network: Network = .mainnet) -> Bool {
        guard !address.utf8.contains(0) else { return false }
        let result = address.withCString { addressCStr in
            platform_wallet_address_validate(addressCStr, network.ffiValue)
        }
        return PlatformWalletResult(result).isSuccess
    }
}
