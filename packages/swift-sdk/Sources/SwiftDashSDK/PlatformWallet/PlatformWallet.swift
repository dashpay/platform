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
        platform_wallet_info_destroy(handle).discard()
    }

    /// Create a new Platform Wallet from a 64-byte seed
    public static func fromSeed(_ seed: Data, network: PlatformNetwork = .testnet) throws -> PlatformWallet {
        guard seed.count == 64 else {
            throw PlatformWalletError.invalidParameter(
                "seed must be 64 bytes, got \(seed.count)"
            )
        }

        var handle: Handle = NULL_HANDLE
        try seed.withUnsafeBytes { seedPtr in
            try platform_wallet_info_create_from_seed(
                network.ffiValue,
                seedPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(seed.count),
                &handle
            ).check()
        }
        return PlatformWallet(handle: handle)
    }

    /// Create a new Platform Wallet from a BIP39 mnemonic phrase
    public static func fromMnemonic(
        _ mnemonic: String,
        passphrase: String? = nil,
        network: PlatformNetwork = .testnet
    ) throws -> PlatformWallet {
        var handle: Handle = NULL_HANDLE

        let mnemonicCStr = (mnemonic as NSString).utf8String
        let passphraseCStr = passphrase != nil ? (passphrase! as NSString).utf8String : nil

        try platform_wallet_info_create_from_mnemonic(
            network.ffiValue,
            mnemonicCStr,
            passphraseCStr,
            &handle
        ).check()

        return PlatformWallet(handle: handle)
    }

    /// Get the identity manager for a specific network
    public func getIdentityManager(for network: PlatformNetwork) throws -> IdentityManager {
        // Check if we already have it cached
        if let manager = identityManagers[network] {
            return manager
        }

        var managerHandle: Handle = NULL_HANDLE
        try platform_wallet_info_get_identity_manager(handle, &managerHandle).check()

        let manager = IdentityManager(handle: managerHandle)
        identityManagers[network] = manager
        return manager
    }

    /// Set the identity manager for a specific network
    public func setIdentityManager(_ manager: IdentityManager, for network: PlatformNetwork) throws {
        try platform_wallet_info_set_identity_manager(handle, manager.handle).check()
        identityManagers[network] = manager
    }
}
