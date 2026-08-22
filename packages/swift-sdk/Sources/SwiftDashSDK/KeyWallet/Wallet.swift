import Foundation
import DashSDKFFI

/// Swift wrapper for a Dash wallet with HD key derivation
public class Wallet {
    internal let handle: OpaquePointer
    private let ownsHandle: Bool

    // MARK: - Static Methods

    /// Initialize the key wallet library (call once at app startup)
    public static func initialize() -> Bool {
        return key_wallet_ffi_initialize()
    }

    /// Get library version
    public static var version: String {
        guard let versionPtr = key_wallet_ffi_version() else {
            return "Unknown"
        }
        return String(cString: versionPtr)
    }

    // MARK: - Initialization

    /// Create a wallet from a mnemonic phrase
    /// - Parameters:
    ///   - mnemonic: The mnemonic phrase
    ///   - network: The network type
    ///   - accountOptions: Account creation options
    public init(mnemonic: String,
                network: Network = .mainnet,
                accountOptions: AccountCreationOption = .default) throws {

        var error = FFIError()
        let walletPtr: OpaquePointer?

        if case .specificAccounts = accountOptions {
            // Use the with_options variant for specific accounts
            var options = accountOptions.toFFIOptions()

            // Note: For production, we'd need to properly manage the memory for the arrays
            // This is a simplified version
            walletPtr = mnemonic.withCString { mnemonicCStr in
                wallet_create_from_mnemonic_with_options(
                    mnemonicCStr,
                    network.ffiValue,
                    &options,
                    &error
                )
            }
        } else {
            // Use simpler variant for default options
            walletPtr = mnemonic.withCString { mnemonicCStr in
                wallet_create_from_mnemonic(
                    mnemonicCStr,
                    network.ffiValue,
                    &error
                )
            }
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        if let handle = walletPtr {
            self.handle = handle
            self.ownsHandle = true
            return
        }

        let ffiFailure = KeyWalletError(ffiError: error)
        // `wallet_create_from_mnemonic` parses with a hardcoded English
        // wordlist. A phrase in any other supported language still validates
        // (`mnemonic_validate` tries every wordlist), and wallet ids and
        // derived keys are determined by the BIP-39 seed alone, so build the
        // identical wallet from the seed instead. `Mnemonic.toSeed` handles
        // every supported language.
        guard case .invalidMnemonic = ffiFailure, Mnemonic.validate(mnemonic) else {
            throw ffiFailure
        }
        let seed = try Mnemonic.toSeed(mnemonic: mnemonic)
        self.handle = try Self.createHandle(
            seed: seed, network: network, accountOptions: accountOptions)
        self.ownsHandle = true
    }

    /// Create a wallet from seed bytes
    /// - Parameters:
    ///   - seed: The seed bytes (typically 64 bytes)
    ///   - network: The network type
    ///   - accountOptions: Account creation options
    public init(seed: Data, network: Network = .mainnet,
                accountOptions: AccountCreationOption = .default) throws {
        self.handle = try Self.createHandle(
            seed: seed, network: network, accountOptions: accountOptions)
        self.ownsHandle = true
    }

    private static func createHandle(seed: Data, network: Network,
                                     accountOptions: AccountCreationOption) throws -> OpaquePointer {
        var error = FFIError()
        let walletPtr: OpaquePointer? = seed.withUnsafeBytes { seedBytes in
            let seedPtr = seedBytes.bindMemory(to: UInt8.self).baseAddress

            if case .specificAccounts = accountOptions {
                var options = accountOptions.toFFIOptions()
                return wallet_create_from_seed_with_options(
                    seedPtr,
                    seed.count,
                    network.ffiValue,
                    &options,
                    &error
                )
            } else {
                return wallet_create_from_seed(
                    seedPtr,
                    seed.count,
                    network.ffiValue,
                    &error
                )
            }
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let handle = walletPtr else {
            throw KeyWalletError(ffiError: error)
        }

        return handle
    }

    /// Create a watch-only wallet from extended public key
    /// - Parameters:
    ///   - xpub: The extended public key string
    ///   - network: The network type
    public init(xpub: String, network: Network = .mainnet) throws {
        // Create an empty wallet first (no accounts)
        var error = FFIError()
        var options = AccountCreationOption.noAccounts.toFFIOptions()

        // Create a random wallet with no accounts
        let walletPtr = wallet_create_random_with_options(network.ffiValue, &options, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let handle = walletPtr else {
            throw KeyWalletError(ffiError: error)
        }

        self.handle = handle
        self.ownsHandle = true

        // Now add the watch-only account with the provided xpub
        do {
            _ = try addAccount(type: .standardBIP44, index: 0, xpub: xpub)
        } catch {
            // Clean up the wallet if adding account failed
            wallet_free(handle)
            throw error
        }
    }

    /// Create a new random wallet
    /// - Parameters:
    ///   - network: The network type
    ///   - accountOptions: Account creation options
    public static func createRandom(network: Network = .mainnet,
                                   accountOptions: AccountCreationOption = .default) throws -> Wallet {
        var error = FFIError()
        let walletPtr: OpaquePointer?

        if case .specificAccounts = accountOptions {
            var options = accountOptions.toFFIOptions()
            walletPtr = wallet_create_random_with_options(network.ffiValue, &options, &error)
        } else {
            walletPtr = wallet_create_random(network.ffiValue, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = walletPtr else {
            throw KeyWalletError(ffiError: error)
        }

        // Create a wrapper that takes ownership
        let wallet = Wallet(handle: ptr, network: network)
        return wallet
    }

    /// Private initializer for internal use (takes ownership)
    private init(handle: OpaquePointer, network: Network) {
        self.handle = handle
        self.ownsHandle = true
    }

    // MARK: - Wallet Properties

    /// Get the wallet ID (32-byte hash)
    public var id: Data {
        get throws {
            var id = Data(count: 32)
            var error = FFIError()

            let success = id.withUnsafeMutableBytes { idBytes in
                let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
                return wallet_get_id(handle, idPtr, &error)
            }

            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }

            guard success else {
                throw KeyWalletError(ffiError: error)
            }

            return id
        }
    }

    /// Check if wallet has a mnemonic
    public var hasMnemonic: Bool {
        var error = FFIError()
        let result = wallet_has_mnemonic(handle, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        return result
    }

    /// Check if wallet is watch-only
    public var isWatchOnly: Bool {
        var error = FFIError()
        let result = wallet_is_watch_only(handle, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        return result
    }

    // MARK: - Account Management

    /// Get an account by type and index
    /// - Parameters:
    ///   - type: The account type
    ///   - index: The account index
    /// - Returns: An account handle
    public func getAccount(type: AccountType, index: UInt32 = 0) throws -> Account {
        let result = wallet_get_account(handle, index, type.ffiValue)

        defer {
            if result.error_message != nil {
                var mutableResult = result
                account_result_free_error(&mutableResult)
            }
        }

        guard let accountHandle = result.account else {
            var error = FFIError()
            error.code = FFIErrorCode(rawValue: UInt32(result.error_code))
            if let msg = result.error_message {
                error.message = msg
            }
            throw KeyWalletError(ffiError: error)
        }

        return Account(handle: accountHandle, wallet: self)
    }

    /// Get an identity top-up account with a specific registration index
    /// - Parameter registrationIndex: The identity registration index
    /// - Returns: An account handle
    public func getTopUpAccount(registrationIndex: UInt32) throws -> Account {
        let result = wallet_get_top_up_account_with_registration_index(
            handle, registrationIndex)

        defer {
            if result.error_message != nil {
                var mutableResult = result
                account_result_free_error(&mutableResult)
            }
        }

        guard let accountHandle = result.account else {
            var error = FFIError()
            error.code = FFIErrorCode(rawValue: UInt32(result.error_code))
            if let msg = result.error_message {
                error.message = msg
            }
            throw KeyWalletError(ffiError: error)
        }

        return Account(handle: accountHandle, wallet: self)
    }

    /// Add an account to the wallet
    /// - Parameters:
    ///   - type: The account type
    ///   - index: The account index
    ///   - xpub: Optional extended public key for watch-only accounts
    /// - Returns: The newly added account
    public func addAccount(type: AccountType, index: UInt32, xpub: String? = nil) throws -> Account {
        let result: FFIAccountResult

        if let xpub = xpub {
            result = xpub.withCString { xpubCStr in
                wallet_add_account_with_string_xpub(
                    handle, type.ffiValue, index, xpubCStr)
            }
        } else {
            result = wallet_add_account(
                handle, type.ffiValue, index)
        }

        defer {
            if result.error_message != nil {
                var mutableResult = result
                account_result_free_error(&mutableResult)
            }
        }

        guard let accountHandle = result.account else {
            var error = FFIError()
            error.code = FFIErrorCode(rawValue: UInt32(result.error_code))
            if let msg = result.error_message {
                error.message = msg
            }
            throw KeyWalletError(ffiError: error)
        }

        return Account(handle: accountHandle, wallet: self)
    }

    /// Add a Platform Payment account (DIP-17) to the wallet.
    ///
    /// Platform Payment accounts use derivation path:
    /// `m/9'/coin_type'/17'/account'/key_class'/index`
    ///
    /// - Parameters:
    ///   - accountIndex: The account index (hardened).
    ///   - keyClass: The key class (hardened). 0 = receive.
    public func addPlatformPaymentAccount(accountIndex: UInt32 = 0, keyClass: UInt32 = 0) throws {
        let result = wallet_add_platform_payment_account(handle, accountIndex, keyClass)

        defer {
            if result.error_message != nil {
                var mutableResult = result
                account_result_free_error(&mutableResult)
            }
        }

        guard result.account != nil else {
            var error = FFIError()
            error.code = FFIErrorCode(rawValue: UInt32(result.error_code))
            if let msg = result.error_message {
                error.message = msg
            }
            throw KeyWalletError(ffiError: error)
        }
    }

    /// Get the number of accounts in the wallet
    public var accountCount: UInt32 {
        var error = FFIError()
        let count = wallet_get_account_count(handle, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        return count
    }

    // MARK: - Key Derivation

    /// Get the extended public key for an account
    /// - Parameter accountIndex: The account index
    /// - Returns: The extended public key string
    public func getAccountXpub(accountIndex: UInt32) throws -> String {
        var error = FFIError()
        let xpubPtr = wallet_get_account_xpub(handle, accountIndex, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = xpubPtr else {
            throw KeyWalletError(ffiError: error)
        }

        let xpub = String(cString: ptr)
        string_free(ptr)

        return xpub
    }

    /// Get the extended private key for an account (only for non-watch-only wallets)
    /// - Parameter accountIndex: The account index
    /// - Returns: The extended private key string
    public func getAccountXpriv(accountIndex: UInt32) throws -> String {
        guard !isWatchOnly else {
            throw KeyWalletError.invalidState("Cannot get private key from watch-only wallet")
        }

        var error = FFIError()
        let xprivPtr = wallet_get_account_xpriv(handle, accountIndex, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = xprivPtr else {
            throw KeyWalletError(ffiError: error)
        }

        let xpriv = String(cString: ptr)
        string_free(ptr)

        return xpriv
    }

    /// Derive a private key at a specific path
    /// - Parameter derivationPath: The BIP32 derivation path
    /// - Returns: The private key in WIF format
    public func derivePrivateKey(path: String) throws -> String {
        guard !isWatchOnly else {
            throw KeyWalletError.invalidState("Cannot derive private key from watch-only wallet")
        }

        var error = FFIError()
        let wifPtr = path.withCString { pathCStr in
            wallet_derive_private_key_as_wif(handle, pathCStr, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = wifPtr else {
            throw KeyWalletError(ffiError: error)
        }

        let wif = String(cString: ptr)
        string_free(ptr)

        return wif
    }

    /// Derive a public key at a specific path
    /// - Parameter derivationPath: The BIP32 derivation path
    /// - Returns: The public key as hex string
    public func derivePublicKey(path: String) throws -> String {
        var error = FFIError()
        let hexPtr = path.withCString { pathCStr in
            wallet_derive_public_key_as_hex(handle, pathCStr, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = hexPtr else {
            throw KeyWalletError(ffiError: error)
        }

        let hex = String(cString: ptr)
        string_free(ptr)

        return hex
    }

    // MARK: - Internal

    /// Get the raw FFI handle (for internal use)
    // MARK: - Account Collection

    /// Get a collection of all accounts in this wallet
    /// - Parameter network: The network type
    /// - Returns: The account collection
    public func getAccountCollection() throws -> AccountCollection {
        var error = FFIError()

        guard let collectionHandle = wallet_get_account_collection(handle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        return AccountCollection(handle: collectionHandle, wallet: self)
    }

    internal var ffiHandle: OpaquePointer { handle }

    // Non-owning initializer for wallets obtained from WalletManager
    public init(nonOwningHandle handle: UnsafeRawPointer) {
        self.handle = OpaquePointer(handle)
        self.ownsHandle = false
    }


    deinit {
        if ownsHandle {
            wallet_free(handle)
        }
    }
}
