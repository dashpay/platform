import Foundation
import DashSDKFFI

/// Platform Wallet for managing identities and DashPay contacts
public class PlatformWallet {
    private let handle: Handle
    private var identityManagers: [PlatformNetwork: IdentityManager] = [:]

    private init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        platform_wallet_info_destroy(handle)
    }

    /// Create a new Platform Wallet from a 64-byte seed
    public static func fromSeed(_ seed: Data) throws -> PlatformWallet {
        guard seed.count == 64 else {
            throw PlatformWalletError.invalidParameter
        }

        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = seed.withUnsafeBytes { seedPtr in
            platform_wallet_info_create_from_seed(
                seedPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                seed.count,
                &handle,
                &error
            )
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return PlatformWallet(handle: handle)
    }

    /// Create a new Platform Wallet from a BIP39 mnemonic phrase
    public static func fromMnemonic(_ mnemonic: String, passphrase: String? = nil) throws -> PlatformWallet {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let mnemonicCStr = (mnemonic as NSString).utf8String
        let passphraseCStr = passphrase != nil ? (passphrase! as NSString).utf8String : nil

        let result = platform_wallet_info_create_from_mnemonic(
            mnemonicCStr,
            passphraseCStr,
            &handle,
            &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return PlatformWallet(handle: handle)
    }

    /// Get the identity manager for a specific network
    public func getIdentityManager(for network: PlatformNetwork) throws -> IdentityManager {
        // Check if we already have it cached
        if let manager = identityManagers[network] {
            return manager
        }

        var managerHandle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = platform_wallet_info_get_identity_manager(
            handle,
            network.ffiValue,
            &managerHandle,
            &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        let manager = IdentityManager(handle: managerHandle)
        identityManagers[network] = manager
        return manager
    }

    /// Set the identity manager for a specific network
    public func setIdentityManager(_ manager: IdentityManager, for network: PlatformNetwork) throws {
        var error = PlatformWalletFFIError()

        let result = platform_wallet_info_set_identity_manager(
            handle,
            network.ffiValue,
            manager.handle,
            &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        identityManagers[network] = manager
    }
}
