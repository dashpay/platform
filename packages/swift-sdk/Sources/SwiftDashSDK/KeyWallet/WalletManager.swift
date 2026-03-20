import Foundation
import DashSDKFFI

/// Swift wrapper for wallet manager that manages multiple wallets
public class WalletManager {
    private let handle: UnsafeMutablePointer<FFIWalletManager>
    internal let network: KeyWalletNetwork
    private let ownsHandle: Bool

    /// Create a new standalone wallet manager
    /// Note: Consider using SPVClient.getWalletManager() instead if you have an SPV client
    public init(network: KeyWalletNetwork = .mainnet,) throws {
        var error = FFIError()
      guard let managerHandle = wallet_manager_create(network.ffiValue, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        self.handle = managerHandle
        self.network = network
        self.ownsHandle = true
    }

    /// Create a wallet manager wrapper from an existing handle (does not own the handle)
    /// - Parameter handle: The FFI wallet manager handle
    internal init(handle: UnsafeMutablePointer<FFIWalletManager>) throws {
        var error = FFIError()
        let network = wallet_manager_network(handle, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        // Check if there was an error
        if error.code != FFIErrorCode(rawValue: 0) {
            throw KeyWalletError(ffiError: error)
        }

        self.handle = handle
        self.network = KeyWalletNetwork(ffiNetwork: network)
        self.ownsHandle = false
    }

    deinit {
        if ownsHandle {
            dash_spv_ffi_wallet_manager_free(handle)
        }
    }

    // MARK: - Wallet Management

    /// Add a wallet from mnemonic
    /// - Parameters:
    ///   - mnemonic: The mnemonic phrase
    ///   - passphrase: Optional BIP39 passphrase
    ///   - accountOptions: Account creation options
    /// - Returns: The wallet ID
    @discardableResult
    public func addWallet(mnemonic: String, passphrase: String? = nil,
                          accountOptions: AccountCreationOption = .default) throws -> Data {
        var error = FFIError()

        let success = mnemonic.withCString { mnemonicCStr in
            if case .specificAccounts = accountOptions {
                var options = accountOptions.toFFIOptions()

                if let passphrase = passphrase {
                    return passphrase.withCString { passphraseCStr in
                        wallet_manager_add_wallet_from_mnemonic_with_options(
                            handle, mnemonicCStr, passphraseCStr,
                            &options, &error)
                    }
                } else {
                    return wallet_manager_add_wallet_from_mnemonic_with_options(
                        handle, mnemonicCStr, nil,
                        &options, &error)
                }
            } else {
                if let passphrase = passphrase {
                    return passphrase.withCString { passphraseCStr in
                        wallet_manager_add_wallet_from_mnemonic(
                            handle, mnemonicCStr, passphraseCStr,
                            &error)
                    }
                } else {
                    return wallet_manager_add_wallet_from_mnemonic(
                        handle, mnemonicCStr, nil,
                        &error)
        }
            }
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        // Get the wallet IDs to return the newly added wallet ID
        return try getWalletIds().last ?? Data()
    }

    /// Get all wallet IDs
    /// - Returns: Array of wallet IDs (32-byte Data objects)
    public func getWalletIds() throws -> [Data] {
        var error = FFIError()
        var walletIdsPtr: UnsafeMutablePointer<UInt8>?
        var count: size_t = 0

        let success = wallet_manager_get_wallet_ids(handle, &walletIdsPtr, &count, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = walletIdsPtr {
                wallet_manager_free_wallet_ids(ptr, count)
            }
        }

        guard success, let ptr = walletIdsPtr else {
            throw KeyWalletError(ffiError: error)
        }

        var walletIds: [Data] = []
        for i in 0..<count {
            let offset = i * 32
            let idData = Data(bytes: ptr.advanced(by: offset), count: 32)
            walletIds.append(idData)
        }

        return walletIds
    }

    /// Get a wallet by ID
    /// - Parameter walletId: The wallet ID (32 bytes)
    /// - Returns: The wallet if found
    public func getWallet(id walletId: Data) throws -> Wallet? {
        guard walletId.count == 32 else {
            throw KeyWalletError.invalidInput("Wallet ID must be exactly 32 bytes")
        }
        var error = FFIError()
        let walletPtr = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_wallet(handle, idPtr, &error)
        }
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        guard let ptr = walletPtr else {
            if error.code == FFIErrorCode(rawValue: 10) { // NOT_FOUND
                return nil
            }
            throw KeyWalletError(ffiError: error)
        }
        // Wrap as non-owning wallet; the manager retains ownership
        let wallet = Wallet(nonOwningHandle: UnsafeRawPointer(ptr))
        return wallet
    }

    /// Get the number of wallets
    public var walletCount: Int {
        get throws {
            var error = FFIError()
            let count = wallet_manager_wallet_count(handle, &error)

            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }

            // Check if there was an error
            if error.code != FFIErrorCode(rawValue: 0) {
                throw KeyWalletError(ffiError: error)
            }

            return count
        }
    }

    // MARK: - Address Management

    /// Get next receive address for a wallet
    /// - Parameters:
    ///   - walletId: The wallet ID
    ///   - accountIndex: The account index
    /// - Returns: The next receive address
    public func getReceiveAddress(walletId: Data, accountIndex: UInt32 = 0) throws -> String {
        guard walletId.count == 32 else {
            throw KeyWalletError.invalidInput("Wallet ID must be exactly 32 bytes")
        }

        var error = FFIError()

        // First get the managed wallet info
        guard let managedInfo = walletId.withUnsafeBytes({ idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_managed_wallet_info(handle, idPtr, &error)
        }) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        defer {
            managed_wallet_info_free(managedInfo)
        }

        // Get the wallet
        guard let wallet = walletId.withUnsafeBytes({ idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_wallet(handle, idPtr, &error)
        }) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        // Now get the receive address
        let addressPtr = managed_wallet_get_next_bip44_receive_address(
            managedInfo, wallet, accountIndex, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = addressPtr else {
            throw KeyWalletError(ffiError: error)
        }

        let address = String(cString: ptr)
        address_free(ptr)

        return address
    }

    /// Get next change address for a wallet
    /// - Parameters:
    ///   - walletId: The wallet ID
    ///   - accountIndex: The account index
    /// - Returns: The next change address
    public func getChangeAddress(walletId: Data, accountIndex: UInt32 = 0) throws -> String {
        guard walletId.count == 32 else {
            throw KeyWalletError.invalidInput("Wallet ID must be exactly 32 bytes")
        }

        var error = FFIError()

        // First get the managed wallet info
        guard let managedInfo = walletId.withUnsafeBytes({ idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_managed_wallet_info(handle, idPtr, &error)
        }) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        defer {
            managed_wallet_info_free(managedInfo)
        }

        // Get the wallet
        guard let wallet = walletId.withUnsafeBytes({ idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_wallet(handle, idPtr, &error)
        }) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        // Now get the change address
        let addressPtr = managed_wallet_get_next_bip44_change_address(
            managedInfo, wallet, accountIndex, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let ptr = addressPtr else {
            throw KeyWalletError(ffiError: error)
        }

        let address = String(cString: ptr)
        address_free(ptr)

        return address
    }


    // MARK: - Balance

    /// Get wallet balance
    /// - Parameter walletId: The wallet ID
    /// - Returns: Tuple of (confirmed, unconfirmed) balance
    public func getWalletBalance(walletId: Data) throws -> (confirmed: UInt64, unconfirmed: UInt64) {
        guard walletId.count == 32 else {
            throw KeyWalletError.invalidInput("Wallet ID must be exactly 32 bytes")
        }

        var error = FFIError()
        var confirmed: UInt64 = 0
        var unconfirmed: UInt64 = 0

        let success = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_get_wallet_balance(
                handle, idPtr, &confirmed, &unconfirmed, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return (confirmed: confirmed, unconfirmed: unconfirmed)
    }

    // MARK: - Transaction Processing

    /// Process a transaction through all wallets
    /// - Parameters:
    ///   - transactionData: The transaction bytes
    ///   - contextDetails: Transaction context details
    ///   - updateStateIfFound: Whether to update wallet state if transaction is relevant
    /// - Returns: True if transaction was relevant to at least one wallet
    @discardableResult
    public func processTransaction(_ transactionData: Data,
                                  contextDetails: TransactionContextDetails,
                                  updateStateIfFound: Bool = true) throws -> Bool {
        var error = FFIError()
        var ffiContext = contextDetails.toFFI()

        let success = transactionData.withUnsafeBytes { txBytes in
            let txPtr = txBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_manager_process_transaction(
                handle, txPtr, transactionData.count,
                &ffiContext,
                updateStateIfFound, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return success
    }

    // MARK: - Block Height Management

    /// Get the current block height for a network
    /// - Parameter network: The network type
    /// - Returns: The current block height
    public func currentHeight() throws -> UInt32 {
        var error = FFIError()

        let height = wallet_manager_current_height(handle, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        // Check if there was an error
        if error.code != FFIErrorCode(rawValue: 0) {
            throw KeyWalletError(ffiError: error)
        }

        return height
    }

    // MARK: - Managed Accounts

    /// Get a managed account from a wallet
    /// - Parameters:
    ///   - walletId: The wallet ID
    ///   - accountIndex: The account index
    ///   - accountType: The type of account to get
    /// - Returns: The managed account
    public func getManagedAccount(walletId: Data, accountIndex: UInt32, accountType: AccountType) throws -> ManagedAccount {
        guard walletId.count == 32 else {
            throw KeyWalletError.invalidInput("Wallet ID must be exactly 32 bytes")
        }

        var result = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return managed_wallet_get_account(handle, idPtr, accountIndex, accountType.ffiValue)
        }

        defer {
            if result.error_message != nil {
                managed_core_account_result_free_error(&result)
            }
        }

        guard let accountHandle = result.account else {
            let errorMessage = result.error_message != nil ? String(cString: result.error_message!) : "Unknown error"
            throw KeyWalletError.walletError(errorMessage)
        }

        return ManagedAccount(handle: accountHandle, manager: self)
    }

    /// Get a managed top-up account with a specific registration index
    /// - Parameters:
    ///   - walletId: The wallet ID
    ///   - registrationIndex: The registration index
    /// - Returns: The managed account
    public func getManagedTopUpAccount(walletId: Data, registrationIndex: UInt32) throws -> ManagedAccount {
        guard walletId.count == 32 else {
            throw KeyWalletError.invalidInput("Wallet ID must be exactly 32 bytes")
        }

        var result = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return managed_wallet_get_top_up_account_with_registration_index(
                handle, idPtr, registrationIndex)
        }

        defer {
            if result.error_message != nil {
                managed_core_account_result_free_error(&result)
            }
        }

        guard let accountHandle = result.account else {
            let errorMessage = result.error_message != nil ? String(cString: result.error_message!) : "Unknown error"
            throw KeyWalletError.walletError(errorMessage)
        }

        return ManagedAccount(handle: accountHandle, manager: self)
    }

    /// Get a collection of all managed accounts for a wallet
    /// - Parameters:
    ///   - walletId: The wallet ID
    /// - Returns: The managed account collection
    public func getManagedAccountCollection(walletId: Data) -> ManagedAccountCollection? {
        guard walletId.count == 32 else {
            return nil
        }

        var error = FFIError()

        let collectionHandle = walletId.withUnsafeBytes { idBytes in
            let idPtr = idBytes.bindMemory(to: UInt8.self).baseAddress
            return managed_wallet_get_account_collection(handle, idPtr, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let collection = collectionHandle else {
            return nil
        }

        return ManagedAccountCollection(handle: collection, manager: self)
    }

    /// Ensure a wallet has a DIP-17 Platform Payment account created.
    /// If it already exists, this is a no-op.
    ///
    /// - Parameters:
    ///   - walletId: The wallet ID (32 bytes)
    ///   - accountIndex: Account index (default 0)
    ///   - keyClass: Key class (default 0 = receive)
    public func ensurePlatformPaymentAccount(
        walletId: Data,
        accountIndex: UInt32 = 0,
        keyClass: UInt32 = 0
    ) throws {
        guard let wallet = try getWallet(id: walletId) else {
            throw KeyWalletError.notFound("Wallet not found")
        }

        // Check if the account already exists
        guard let collection = getManagedAccountCollection(walletId: walletId) else {
            throw KeyWalletError.notFound("Account collection not found")
        }
        if collection.getPlatformPaymentAccount(accountIndex: accountIndex, keyClass: keyClass) != nil {
            return // Already exists
        }

        // Create the platform payment account
        try wallet.addPlatformPaymentAccount(accountIndex: accountIndex, keyClass: keyClass)
    }

    /// Get platform payment addresses for a wallet, suitable for BLAST sync.
    ///
    /// Returns (derivation index, address key bytes) tuples from the platform
    /// payment account's address pool.
    ///
    /// - Parameters:
    ///   - walletId: The wallet ID (32 bytes)
    ///   - accountIndex: Account index (default 0)
    ///   - keyClass: Key class (default 0 = receive)
    /// - Returns: Array of (index, key) tuples for BLAST sync, or empty if no platform account.
    public func getPlatformAddresses(
        walletId: Data,
        accountIndex: UInt32 = 0,
        keyClass: UInt32 = 0
    ) throws -> [(index: UInt32, key: Data)] {
        guard let collection = getManagedAccountCollection(walletId: walletId) else {
            return []
        }

        guard let platformAccount = collection.getPlatformPaymentAccount(
            accountIndex: accountIndex, keyClass: keyClass
        ) else {
            return []
        }

        guard let pool = platformAccount.getAddressPool() else {
            return []
        }

        // Get all generated addresses from the pool
        let addresses = try pool.getAddresses(from: 0, to: 0) // 0,0 = all addresses
        return addresses.compactMap { info in
            // Convert scriptPubKey to GroveDB storage key via Rust FFI
            var outKey = Data(count: 21)
            var outKeyLen: UInt32 = 21 // capacity of outKey buffer
            let success = info.scriptPubKey.withUnsafeBytes { scriptBuffer -> Bool in
                guard let scriptBase = scriptBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    return false
                }
                return outKey.withUnsafeMutableBytes { keyBuffer -> Bool in
                    guard let keyBase = keyBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return false
                    }
                    return dash_sdk_script_to_platform_address_key(
                        scriptBase,
                        UInt32(info.scriptPubKey.count),
                        keyBase,
                        &outKeyLen
                    )
                }
            }
            guard success, outKeyLen > 0 else { return nil }
            return (index: info.index, key: outKey.prefix(Int(outKeyLen)))
        }
    }

    internal var ffiHandle: UnsafeMutablePointer<FFIWalletManager> { handle }

    // MARK: - Serialization

    /// Add a wallet from mnemonic and return serialized wallet bytes
    /// - Parameters:
    ///   - mnemonic: The mnemonic phrase
    ///   - passphrase: Optional BIP39 passphrase
    ///   - birthHeight: Optional birth height for wallet
    ///   - accountOptions: Account creation options
    ///   - downgradeToPublicKeyWallet: If true, creates a watch-only or externally signable wallet
    ///   - allowExternalSigning: If true AND downgradeToPublicKeyWallet is true, creates an externally signable wallet
    /// - Returns: Tuple containing (walletId: Data, serializedWallet: Data)
    public func addWalletAndSerialize(
        mnemonic: String,
        passphrase: String? = nil,
        birthHeight: UInt32 = 0,
        accountOptions: AccountCreationOption = .default,
        downgradeToPublicKeyWallet: Bool = false,
        allowExternalSigning: Bool = false
    ) throws -> (walletId: Data, serializedWallet: Data) {
        var error = FFIError()
        var walletBytesPtr: UnsafeMutablePointer<UInt8>?
        var walletBytesLen: size_t = 0
        var walletId = [UInt8](repeating: 0, count: 32)

        let success = mnemonic.withCString { mnemonicCStr in
            var options = accountOptions.toFFIOptions()

            if let passphrase = passphrase {
                return passphrase.withCString { passphraseCStr in
                    wallet_manager_add_wallet_from_mnemonic_return_serialized_bytes(
                        handle,
                        mnemonicCStr,
                        passphraseCStr,
                        birthHeight,
                        &options,
                        downgradeToPublicKeyWallet,
                        allowExternalSigning,
                        &walletBytesPtr,
                        &walletBytesLen,
                        &walletId,
                        &error
                    )
                }
            } else {
                return wallet_manager_add_wallet_from_mnemonic_return_serialized_bytes(
                    handle,
                    mnemonicCStr,
                    nil,
                    birthHeight,
                    &options,
                    downgradeToPublicKeyWallet,
                    allowExternalSigning,
                    &walletBytesPtr,
                    &walletBytesLen,
                    &walletId,
                    &error
                )
            }
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            // Free the allocated bytes after copying
            if let ptr = walletBytesPtr {
                wallet_manager_free_wallet_bytes(ptr, walletBytesLen)
            }
        }

        guard success, let bytesPtr = walletBytesPtr else {
            throw KeyWalletError(ffiError: error)
        }

        // Copy the data before freeing (which happens in defer)
        let serializedData = Data(bytes: bytesPtr, count: Int(walletBytesLen))
        let walletIdData = Data(walletId)

        return (walletId: walletIdData, serializedWallet: serializedData)
    }

    /// Import a wallet from serialized bytes
    /// - Parameters:
    ///   - walletBytes: The serialized wallet data
    /// - Returns: The wallet ID of the imported wallet
    public func importWallet(from walletBytes: Data) throws -> Data {
        guard !walletBytes.isEmpty else {
            throw KeyWalletError.invalidInput("Wallet bytes cannot be empty")
        }

        var error = FFIError()
        var walletId = [UInt8](repeating: 0, count: 32)

        let success = walletBytes.withUnsafeBytes { bytes in
            wallet_manager_import_wallet_from_bytes(
                handle,
                bytes.bindMemory(to: UInt8.self).baseAddress,
                size_t(walletBytes.count),
                &walletId,
                &error
            )
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return Data(walletId)
    }
}
