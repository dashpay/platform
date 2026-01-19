import Foundation
import DashSDKFFI

/// Platform Wallet for managing identities and DashPay contacts
public class PlatformWallet {
    internal let handle: Handle
    private var identityManagers: [PlatformNetwork: IdentityManager] = [:]

    private init(handle: Handle) {
        self.handle = handle
    }

    /// Internal access to wallet handle for sync operations
    internal var walletHandle: Handle {
        return handle
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

    // MARK: - Serialization

    /// Serialize the wallet to binary data for persistence
    ///
    /// This serializes all wallet state including addresses and identity information.
    /// The serialized data can be used with `restore(from:)` to recreate the wallet
    /// without needing the seed/mnemonic.
    public func serialize() throws -> Data {
        var bytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var len: Int = 0
        var error = PlatformWalletFFIError()

        let result = platform_wallet_info_serialize(
            handle,
            &bytesPtr,
            &len,
            &error
        )

        guard result == Success, let ptr = bytesPtr, len > 0 else {
            throw PlatformWalletError(result: result, error: error)
        }

        // Copy data before freeing
        let data = Data(bytes: ptr, count: len)
        platform_wallet_bytes_free(ptr)

        return data
    }

    /// Restore a wallet from serialized data
    ///
    /// This restores a wallet from binary data created by `serialize()`.
    /// The wallet will have all its previous state including addresses
    /// and can be used for sync operations without needing the PIN.
    public static func restore(from data: Data) throws -> PlatformWallet {
        var handle: Handle = NULL_HANDLE
        var error = PlatformWalletFFIError()

        let result = data.withUnsafeBytes { dataPtr in
            platform_wallet_info_deserialize(
                dataPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                data.count,
                &handle,
                &error
            )
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return PlatformWallet(handle: handle)
    }
}
