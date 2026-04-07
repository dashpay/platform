import Foundation
import SwiftData
import Combine

import DashSDKFFI

// MARK: - Wallet Manager

/// CoreWalletManager is a wrapper around the SDK's WalletManager
/// It delegates all wallet operations to the SDK layer while maintaining
/// SwiftUI compatibility through ObservableObject and SwiftData persistence
@MainActor
public class CoreWalletManager: ObservableObject {
    @Published public private(set) var wallets: [HDWallet] = []
    @Published public private(set) var error: WalletError?

    // SDK wallet manager - this is the real wallet manager from the SDK
    private let sdkWalletManager: WalletManager
    private let modelContainer: ModelContainer
    private let storage = WalletStorage()

    /// Initialize with a valid SPVClient instance
    init(spvClient: SPVClient, modelContainer: ModelContainer) throws {
        print("=== WalletManager.init START ===")

        self.sdkWalletManager = try spvClient.getWalletManager()
        self.modelContainer = modelContainer

        print("=== WalletManager.init SUCCESS ===")

        Task {
            await loadWallets()
        }
    }

    /// Public convenience initializer for one-shot / migration use cases.
    ///
    /// Constructs an `SPVClient` with no on-disk data directory (`dataDir: nil`)
    /// and an in-process SwiftData `ModelContainer` using the SDK's standard
    /// schema, then chains to the designated initializer. Suitable for callers
    /// that need to perform offline wallet operations — e.g. importing a wallet
    /// from an existing mnemonic during a key migration — without configuring
    /// an SPV peer set or owning their own persistence stack.
    ///
    /// The created `CoreWalletManager` is fully functional for `createWallet`,
    /// `addAccount`, and other in-process operations. It does not start any
    /// network sync; consumers that need a live SPV peer connection should
    /// continue to use the designated initializer with a fully configured
    /// `SPVClient`.
    ///
    /// - Parameter keyWalletNetwork: The Dash network to operate against
    ///   (`.mainnet`, `.testnet`, `.regtest`, or `.devnet`).
    /// - Throws: Any error raised by `ModelContainerHelper.createContainer()`,
    ///   `SPVClient.init`, or the designated initializer.
    public convenience init(keyWalletNetwork: KeyWalletNetwork) throws {
        let modelContainer = try ModelContainerHelper.createContainer()
        let dashSDKNetwork = DashSDKNetwork(rawValue: keyWalletNetwork.rawValue)
        let spvClient = try SPVClient(
            network: dashSDKNetwork,
            dataDir: nil,
            startHeight: 0,
            eventHandlers: nil)
        try self.init(spvClient: spvClient, modelContainer: modelContainer)
    }

    // MARK: - Wallet Management
    public func createWallet(label: String, mnemonic: String, pin: String, isImport: Bool = false) async throws -> HDWallet {
        print("WalletManager.createWallet called")

        print("Validating provided mnemonic...")
        guard SwiftDashSDK.Mnemonic.validate(mnemonic) else {
            print("Mnemonic validation failed")
            throw WalletError.invalidMnemonic
        }

        // Add wallet through SDK (with bitfield networks) and capture serialized bytes for persistence
        let walletId: Data
        let serializedBytes: Data
        do {
            // Calculate birthHeight based on wallet type
            // For imported wallets: use 730k for mainnet, 0 for test/devnets (need to sync from genesis)
            let birthHeight: UInt32
            if isImport {
                // Imported wallet should sync from a reasonable historical point
                birthHeight = sdkWalletManager.network == .mainnet ? 730_000 : 0
            } else {
                birthHeight = 0
            }

            print("Creating wallet with birthHeight: \(birthHeight) (isImport: \(isImport)")

            // Add wallet using SDK's WalletManager with combined network bitfield and serialize
            let result = try sdkWalletManager.addWalletAndSerialize(
                mnemonic: mnemonic,
                passphrase: nil,
                birthHeight: birthHeight,
                accountOptions: .default,
                downgradeToPublicKeyWallet: false,
                allowExternalSigning: false
            )
            walletId = result.walletId
            serializedBytes = result.serializedWallet

            print("Wallet added with ID: \(walletId.hexString)")

            // Ensure a DIP-17 Platform Payment account is created for BLAST sync
            do {
                try sdkWalletManager.ensurePlatformPaymentAccount(walletId: walletId)
                print("Platform payment account ensured for wallet \(walletId.hexString)")
            } catch {
                print("Warning: Failed to create platform payment account: \(error)")
                // Non-fatal — wallet still works without platform addresses
            }
        } catch {
            print("Failed to add wallet: \(error)")
            throw WalletError.walletError("Failed to add wallet: \(error.localizedDescription)")
        }

        // Create HDWallet model for SwiftUI
        let network = AppNetwork(network: sdkWalletManager.network)
        let wallet = HDWallet(walletId: walletId, serializedWalletBytes: serializedBytes, label: label, network: network, isImported: isImport)

        do {
            let seed = try SwiftDashSDK.Mnemonic.toSeed(mnemonic: mnemonic)
            _ = try storage.storeSeed(seed, pin: pin)
        } catch {
            print("Failed to store seed: \(error)")
            // Continue anyway - wallet is already created
        }

        // Insert wallet into context ans save it
        modelContainer.mainContext.insert(wallet)
        try modelContainer.mainContext.save()

        return wallet
    }

    /// Add a new account to a wallet.
    public func addAccount(to wallet: HDWallet, type: AccountType, index: UInt32, keyClass: UInt32 = 0) throws {
        guard let sdkWallet = try sdkWalletManager.getWallet(id: wallet.walletId) else {
            throw WalletError.walletError("Wallet not found")
        }
        if type == .platformPayment {
            try sdkWallet.addPlatformPaymentAccount(accountIndex: index, keyClass: keyClass)
        } else {
            _ = try sdkWallet.addAccount(type: type, index: index)
        }
    }

    // MARK: - Asset Lock Transaction

    /// Result of building an asset lock transaction.
    public struct AssetLockTransactionResult {
        /// Serialized transaction bytes.
        public let transactionBytes: Data
        /// Index of the asset lock output in the transaction.
        public let outputIndex: UInt32
        /// One-time private key for the asset lock proof (32 bytes).
        public let privateKey: Data
        /// Actual fee paid in duffs.
        public let fee: UInt64
    }

    /// Asset lock funding type.
    public enum AssetLockFundingType: UInt32 {
        case identityRegistration = 0
        case identityTopUp = 1
        case identityTopUpNotBound = 2
        case identityInvitation = 3
        case assetLockAddressTopUp = 4
        case assetLockShieldedAddressTopUp = 5
    }

    /// Build and sign an asset lock transaction for Core → Platform transfers.
    ///
    /// Creates a Core special transaction (type 8) with AssetLockPayload that locks
    /// Dash for Platform credits.
    ///
    /// - Parameters:
    ///   - wallet: The wallet to fund from.
    ///   - accountIndex: BIP44 account index (typically 0).
    ///   - fundingType: The type of asset lock funding account for key derivation.
    ///   - identityIndex: Identity index for key derivation (0 for new).
    ///   - creditOutputs: Array of (scriptPubKey, amount) pairs for platform credit outputs.
    ///   - feePerKb: Fee rate in duffs per kilobyte (0 for default).
    /// - Returns: `AssetLockTransactionResult` with tx bytes, output index, private key, and fee.
    public func buildAssetLockTransaction(
        for wallet: HDWallet,
        accountIndex: UInt32 = 0,
        fundingType: AssetLockFundingType = .assetLockAddressTopUp,
        identityIndex: UInt32 = 0,
        creditOutputs: [(scriptPubKey: Data, amount: UInt64)],
        feePerKb: UInt64 = 1000
    ) throws -> AssetLockTransactionResult {
        guard let sdkWallet = try sdkWalletManager.getWallet(id: wallet.walletId) else {
            throw WalletError.walletError("Wallet not found")
        }

        let count = creditOutputs.count
        guard count > 0 else {
            throw WalletError.walletError("At least one credit output required")
        }

        // Concatenate all scripts into a single contiguous buffer
        // and build an array of pointers into it
        var scriptLens: [Int] = creditOutputs.map { $0.scriptPubKey.count }
        var amounts: [UInt64] = creditOutputs.map { $0.amount }
        var concatenatedScripts = Data()
        for output in creditOutputs {
            concatenatedScripts.append(output.scriptPubKey)
        }

        var feeOut: UInt64 = 0
        var txBytesOut: UnsafeMutablePointer<UInt8>? = nil
        var txLenOut: Int = 0
        var outputIndexOut: UInt32 = 0
        var privateKeyOut: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        var ffiError = FFIError()

        // Build pointers inside withUnsafeBytes so they remain valid
        let success = concatenatedScripts.withUnsafeBytes { allScriptsBuffer -> Bool in
            guard let allScriptsBase = allScriptsBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                return false
            }
            // Build array of pointers into the concatenated buffer
            var scriptPtrs: [UnsafePointer<UInt8>?] = []
            var offset = 0
            for len in scriptLens {
                scriptPtrs.append(allScriptsBase.advanced(by: offset))
                offset += len
            }

            return scriptPtrs.withUnsafeMutableBufferPointer { scriptPtrsBuffer in
                scriptLens.withUnsafeMutableBufferPointer { scriptLensBuffer in
                    amounts.withUnsafeMutableBufferPointer { amountsBuffer in
                        wallet_build_and_sign_asset_lock_transaction(
                            sdkWalletManager.handle,
                            sdkWallet.handle,
                            accountIndex,
                            fundingType.rawValue,
                            identityIndex,
                            scriptPtrsBuffer.baseAddress,
                            scriptLensBuffer.baseAddress,
                            amountsBuffer.baseAddress,
                            count,
                            feePerKb,
                            &feeOut,
                            &txBytesOut,
                            &txLenOut,
                        &outputIndexOut,
                        &privateKeyOut,
                        &ffiError
                    )
                }
            }
        }
        }

        guard success else {
            let msg = ffiError.message != nil ? String(cString: ffiError.message!) : "Unknown error"
            if ffiError.message != nil {
                error_message_free(ffiError.message)
            }
            throw WalletError.walletError("Asset lock transaction failed: \(msg)")
        }

        // Copy transaction bytes
        let txData: Data
        if let ptr = txBytesOut, txLenOut > 0 {
            txData = Data(bytes: ptr, count: txLenOut)
            transaction_bytes_free(ptr)
        } else {
            throw WalletError.walletError("No transaction bytes returned")
        }

        // Copy private key from tuple to Data
        let privateKeyData = withUnsafeBytes(of: privateKeyOut) { Data($0) }

        return AssetLockTransactionResult(
            transactionBytes: txData,
            outputIndex: outputIndexOut,
            privateKey: privateKeyData,
            fee: feeOut
        )
    }

    public func deleteWallet(_ wallet: HDWallet) async throws {
        let walletId = wallet.id

        wallets.removeAll(where: { $0.id == walletId })

        // Now safe to delete from SwiftData (cascade will delete accounts/addresses)
        modelContainer.mainContext.delete(wallet)
        try modelContainer.mainContext.save()
    }

    func importWallet(label: String, network: AppNetwork, mnemonic: String, pin: String) async throws -> HDWallet {
        let wallet = try await createWallet(label: label, mnemonic: mnemonic, pin: pin)
        wallet.isImported = true
        try modelContainer.mainContext.save()
        return wallet
    }

    public func decryptSeed(_ encryptedSeed: Data?) -> Data? {
        // This method is used internally by other services
        // In a real implementation, this would decrypt using the current PIN
        // For now, return nil to indicate manual unlock is needed
        return nil
    }

    public func changeWalletPIN(currentPIN: String, newPIN: String) async throws {
        // Retrieve seed with current PIN
        let seed = try storage.retrieveSeed(pin: currentPIN)

        // Re-encrypt with new PIN
        _ = try storage.storeSeed(seed, pin: newPIN)
    }

    public func enableBiometricProtection(pin: String) async throws {
        // First verify PIN and get seed
        let seed = try storage.retrieveSeed(pin: pin)

        // Enable biometric protection
        try storage.enableBiometricProtection(for: seed)
    }

    public func unlockWithBiometric() async throws -> Data {
        return try storage.retrieveSeedWithBiometric()
    }

    // MARK: - Account Management

    /// Build a signed transaction
    /// - Parameters:
    ///   - accountIndex: The account index to use
    ///   - outputs: The transaction outputs
    /// - Returns: The signed transaction bytes
    public func buildSignedTransaction(for wallet: HDWallet, accIndex: UInt32, outputs: [Transaction.Output]) throws -> (Data, UInt64) {
        try sdkWalletManager.buildSignedTransaction(for: wallet, accIndex: accIndex, outputs: outputs)
    }

    /// Get transactions for a wallet
    /// - Parameters:
    ///   - wallet: The wallet to get transactions for
    ///   - accountIndex: The account index (default 0)
    /// - Returns: Array of wallet transactions
    public func getTransactions(for wallet: HDWallet, accountIndex: UInt32 = 0) -> [WalletTransaction] {
        // Get managed account
        let managedAccount = try! sdkWalletManager.getManagedAccount(
            walletId: wallet.walletId,
            accountIndex: accountIndex,
            accountType: .standardBIP44
        )

        return managedAccount.getTransactions()
    }

    public func getBalance(for wallet: HDWallet) -> Balance {
        let accounts = self.getAccounts(for: wallet)

        var confirmed: UInt64 = 0
        var unconfirmed: UInt64 = 0
        var immature: UInt64 = 0
        var locked: UInt64 = 0

        for account in accounts {
            confirmed += account.balance.confirmed
            unconfirmed += account.balance.unconfirmed
            immature += account.balance.immature
            locked += account.balance.locked
        }

        return Balance(
            confirmed: confirmed,
            unconfirmed: unconfirmed,
            immature: immature,
            locked: locked
        )
    }

    public func getReceiveAddress(for wallet: HDWallet, accountIndex: UInt32 = 0) -> String {
        return try! sdkWalletManager.getReceiveAddress(walletId: wallet.walletId, accountIndex: accountIndex)
    }

    /// Ensure a DIP-17 Platform Payment account exists for the given wallet.
    public func ensurePlatformPaymentAccount(for wallet: HDWallet) throws {
        try sdkWalletManager.ensurePlatformPaymentAccount(walletId: wallet.walletId)
    }

    /// Get platform payment addresses for BLAST sync.
    /// Returns (derivation index, address key) tuples from the DIP-17 address pool.
    public func getPlatformAddresses(for wallet: HDWallet) throws -> [(index: UInt32, key: Data)] {
        return try sdkWalletManager.getPlatformAddresses(walletId: wallet.walletId)
    }

    /// Get the managed account collection for a wallet.
    public func getManagedAccountCollection(for wallet: HDWallet) -> ManagedAccountCollection? {
        return sdkWalletManager.getManagedAccountCollection(walletId: wallet.walletId)
    }
    /// Get detailed account information including xpub and addresses
    /// - Parameters:
    ///   - wallet: The wallet containing the account
    ///   - accountInfo: The account info to get details for
    /// - Returns: Detailed account information
    public func getAccountDetails(for wallet: HDWallet, accountInfo: AccountInfo) -> AccountDetailInfo? {
        let collection = sdkWalletManager.getManagedAccountCollection(walletId: wallet.walletId)

        guard let collection else { return nil }

        // Resolve managed account from category and optional index
        var managed: ManagedAccount?
        switch accountInfo.category {
        case .bip44:
            if let idx = accountInfo.index { managed = collection.getBIP44Account(at: idx) }
        case .bip32:
            if let idx = accountInfo.index { managed = collection.getBIP32Account(at: idx) }
        case .coinjoin:
            if let idx = accountInfo.index { managed = collection.getCoinJoinAccount(at: idx) }
        case .identityRegistration:
            managed = collection.getIdentityRegistrationAccount()
        case .identityInvitation:
            managed = collection.getIdentityInvitationAccount()
        case .identityTopupNotBound:
            managed = collection.getIdentityTopUpNotBoundAccount()
        case .identityTopup:
            if let idx = accountInfo.index { managed = collection.getIdentityTopUpAccount(registrationIndex: idx) }
        case .providerVotingKeys:
            managed = collection.getProviderVotingKeysAccount()
        case .providerOwnerKeys:
            managed = collection.getProviderOwnerKeysAccount()
        case .providerOperatorKeys:
            managed = collection.getProviderOperatorKeysAccount()
        case .providerPlatformKeys:
            managed = collection.getProviderPlatformKeysAccount()
        case .dashPayReceivingFunds, .dashPayExternalAccount:
            managed = nil
        case .platformPayment:
            // Platform Payment uses ManagedPlatformAccount, handled separately below
            managed = nil
        }

        let appNetwork = AppNetwork(network: sdkWalletManager.network)

        let derivationPath = derivationPath(for: accountInfo.category, index: accountInfo.index, network: appNetwork)
        var externalDetails: [AddressDetail] = []
        var internalDetails: [AddressDetail] = []
        var ffiType = FFIAccountType(rawValue: 0)

        // Special handling for Platform Payment accounts — encode as bech32m
        if accountInfo.category == .platformPayment {
            ffiType = FFIAccountType(rawValue: AccountType.platformPayment.rawValue)
            let networkValue: UInt32 = {
                switch appNetwork {
                case .mainnet: return 0
                case .testnet: return 1
                case .regtest: return 2
                case .devnet: return 3
                }
            }()
            if let platformAccount = collection.getPlatformPaymentAccount(accountIndex: accountInfo.index ?? 0, keyClass: 0),
               let pool = platformAccount.getAddressPool(),
               let infos = try? pool.getAddresses(from: 0, to: 0) {
                externalDetails = infos.compactMap { info in
                    let bech32Address = Self.encodePlatformAddress(scriptPubKey: info.scriptPubKey, networkValue: networkValue) ?? info.address
                    return AddressDetail(address: bech32Address, index: info.index, path: info.path, isUsed: info.used, publicKey: info.publicKey?.map { String(format: "%02x", $0) }.joined() ?? "")
                }
            }
        } else if let m = managed {
            ffiType = FFIAccountType(rawValue: m.accountType?.rawValue ?? 0)
            // Query all generated addresses (0 to 0 means "all addresses" in FFI)
            if let pool = m.getExternalAddressPool(), let infos = try? pool.getAddresses(from: 0, to: 0) {
                externalDetails = infos.map { info in
                    AddressDetail(address: info.address, index: info.index, path: info.path, isUsed: info.used, publicKey: info.publicKey?.map { String(format: "%02x", $0) }.joined() ?? "")
                }
            }
            if let pool = m.getInternalAddressPool(), let infos = try? pool.getAddresses(from: 0, to: 0) {
                internalDetails = infos.map { info in
                    AddressDetail(address: info.address, index: info.index, path: info.path, isUsed: info.used, publicKey: info.publicKey?.map { String(format: "%02x", $0) }.joined() ?? "")
                }
            }
            // Single pool fallback
            if externalDetails.isEmpty && internalDetails.isEmpty, let pool = m.getAddressPool(type: .single), let infos = try? pool.getAddresses(from: 0, to: 0) {
                externalDetails = infos.map { info in
                    AddressDetail(address: info.address, index: info.index, path: info.path, isUsed: info.used, publicKey: info.publicKey?.map { String(format: "%02x", $0) }.joined() ?? "")
                }
            }
        }

        let used = externalDetails.filter { $0.isUsed }.count + internalDetails.filter { $0.isUsed }.count
        let unused = externalDetails.filter { !$0.isUsed }.count + internalDetails.filter { !$0.isUsed }.count
        return AccountDetailInfo(
            account: accountInfo,
            accountType: ffiType,
            xpub: nil,
            derivationPath: derivationPath,
            gapLimit: 20,
            usedAddresses: used,
            unusedAddresses: unused,
            externalAddresses: externalDetails,
            internalAddresses: internalDetails
        )
    }

    /// Derive a private key as WIF from seed using a specific path (deferred to SDK)
    public func derivePrivateKeyAsWIF(for wallet: HDWallet, accountInfo: AccountInfo, addressIndex: UInt32) async throws -> String {
        // Obtain a non-owning Wallet wrapper from manager
        guard let sdkWallet = try sdkWalletManager.getWallet(id: wallet.walletId) else {
            throw WalletError.walletError("Wallet not found in manager")
        }

        // Map category to AccountType and master path root
        let coinType = (sdkWalletManager.network == .testnet) ? "1'" : "5'"
        let mapping: (AccountType, UInt32, String)? = {
            switch accountInfo.category {
            case .providerVotingKeys:
                return (.providerVotingKeys, 0, "m/9'/\(coinType)/3'/1'")
            case .providerOwnerKeys:
                return (.providerOwnerKeys, 0, "m/9'/\(coinType)/3'/2'")
            case .providerOperatorKeys:
                return (.providerOperatorKeys, 0, "m/9'/\(coinType)/3'/3'")
            case .providerPlatformKeys:
                return (.providerPlatformKeys, 0, "m/9'/\(coinType)/3'/4'")
            case .bip44:
                let idx = accountInfo.index ?? 0
                return (.standardBIP44, idx, "m/44'/\(coinType)/\(idx)'")
            case .bip32:
                let idx = accountInfo.index ?? 0
                return (.standardBIP32, idx, "m/\(idx)'")
            case .coinjoin:
                let idx = (accountInfo.index ?? 1000) - 1000
                return (.coinJoin, UInt32(idx), "m/9'/\(coinType)/4'/\(idx)'")
            case .identityRegistration, .identityInvitation, .identityTopupNotBound, .identityTopup,
                 .dashPayReceivingFunds, .dashPayExternalAccount, .platformPayment:
                return nil
            }
        }()

        guard let (type, accountIndex, masterPath) = mapping else {
            throw WalletError.notImplemented("Derivation not supported for this account type")
        }

        // Get account and derive
        let account = try sdkWallet.getAccount(type: type, index: accountIndex)
        let wif = try account.derivePrivateKeyWIF(wallet: sdkWallet, masterPath: masterPath, index: addressIndex)
        return wif
    }

    // Index-based derivation was removed. We now map paths by AccountCategory
    // via derivationPath(for:index:network:) below to avoid conflating type with index.

    private func derivationPath(for category: AccountCategory, index: UInt32?, network: AppNetwork) -> String {
        let coinType = network == .testnet ? "1'" : "5'"
        switch category {
        case .bip44:
            return "m/44'/\(coinType)/\(index ?? 0)'"
        case .bip32:
            return "m/\((index ?? 0))'"
        case .coinjoin:
            // Account-level path for coinjoin: m/9'/coinType/4'/account'
            return "m/9'/\(coinType)/4'/\(index ?? 0)'"
        case .identityRegistration:
            return "m/9'/\(coinType)/5'/1'/x"
        case .identityInvitation:
            return "m/9'/\(coinType)/5'/3'/x"
        case .identityTopupNotBound:
            return "m/9'/\(coinType)/5'/2'/x"
        case .identityTopup:
            return "m/9'/\(coinType)/5'/2'/\(index ?? 0)'/x"
        case .providerVotingKeys:
            return "m/9'/\(coinType)/3'/1'/x"
        case .providerOwnerKeys:
            return "m/9'/\(coinType)/3'/2'/x"
        case .providerOperatorKeys:
            return "m/9'/\(coinType)/3'/3'/x"
        case .providerPlatformKeys:
            return "m/9'/\(coinType)/3'/4'/x"
        case .dashPayReceivingFunds:
            return "m/9'/\(coinType)/5'/0'/x"
        case .dashPayExternalAccount:
            return "m/9'/\(coinType)/5'/0'/x"
        case .platformPayment:
            return "m/9'/\(coinType)/15'/\(index ?? 0)'/x"
        }
    }


    /// Encode a P2PKH scriptPubKey as a bech32m platform address (DIP-17/18).
    private static func encodePlatformAddress(scriptPubKey: Data, networkValue: UInt32) -> String? {
        let result = scriptPubKey.withUnsafeBytes { buffer -> DashSDKResult in
            guard let base = buffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                return DashSDKResult()
            }
            return dash_sdk_encode_platform_address(base, UInt32(scriptPubKey.count), networkValue)
        }
        guard result.error == nil, let dataPtr = result.data else {
            if let error = result.error { dash_sdk_error_free(error) }
            return nil
        }
        let str = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        return str
    }

    // Removed old FFI-based helper; using SwiftDashSDK wrappers instead

    /// Get all accounts for a wallet from the FFI wallet manager
    /// - Parameters:
    ///   - wallet: The wallet model
    /// - Returns: Account information including balances and address counts
    public func getAccounts(for wallet: HDWallet) -> [AccountInfo] {
        let collection = sdkWalletManager.getManagedAccountCollection(walletId: wallet.walletId)

        guard let collection else { return [] }

        var list: [AccountInfo] = []

        func counts(_ m: ManagedAccount) -> (Int, Int) {
            var ext = 0, intc = 0
            if let p = m.getExternalAddressPool(), let infos = try? p.getAddresses(from: 0, to: 1000) { ext = infos.count }
            if let p = m.getInternalAddressPool(), let infos = try? p.getAddresses(from: 0, to: 1000) { intc = infos.count }
            return (ext, intc)
        }

        // BIP44
        for idx in collection.getBIP44Indices() {
            if let m = collection.getBIP44Account(at: idx) {
                let b = m.getBalance()
                let c = counts(m)
                list.append(AccountInfo(category: .bip44, index: idx, label: "Account \(idx)", balance: b, addressCount: c))
            }
        }
        // BIP32 (5000+)
        for raw in collection.getBIP32Indices() {
            if let m = collection.getBIP32Account(at: raw) {
                let b = m.getBalance()
                let c = counts(m)
                list.append(AccountInfo(category: .bip32, index: raw, label: "BIP32 \(raw)", balance: b, addressCount: c))
            }
        }
        // CoinJoin (1000+)
        for raw in collection.getCoinJoinIndices() {
            if let m = collection.getCoinJoinAccount(at: raw) {
                let b = m.getBalance()
                var total = 0
                if let p = m.getAddressPool(type: .single), let infos = try? p.getAddresses(from: 0, to: 1000) { total = infos.count }
                list.append(AccountInfo(category: .coinjoin, index: raw, label: "CoinJoin \(raw)", balance: b, addressCount: (total, 0)))
            }
        }
        // Identity accounts
        if let m = collection.getIdentityRegistrationAccount() {
            let b = m.getBalance()
            list.append(AccountInfo(category: .identityRegistration, label: "Identity Registration", balance: b, addressCount: (0, 0)))
        }
        if let m = collection.getIdentityInvitationAccount() {
            let b = m.getBalance()
            list.append(AccountInfo(category: .identityInvitation, label: "Identity Invitation", balance: b, addressCount: (0, 0)))
        }
        if let m = collection.getIdentityTopUpNotBoundAccount() {
            let b = m.getBalance()
            list.append(AccountInfo(category: .identityTopupNotBound, label: "Identity Topup (Not Bound)", balance: b, addressCount: (0, 0)))
        }
        for raw in collection.getIdentityTopUpIndices() {
            if let m = collection.getIdentityTopUpAccount(registrationIndex: raw) {
                let b = m.getBalance()
                list.append(AccountInfo(category: .identityTopup, index: raw, label: "Identity Topup \(raw)", balance: b, addressCount: (0, 0)))
            }
        }
        // Provider
        if let m = collection.getProviderVotingKeysAccount() {
            let b = m.getBalance()
            list.append(AccountInfo(category: .providerVotingKeys, label: "Provider Voting Keys", balance: b, addressCount: (0, 0)))
        }
        if let m = collection.getProviderOwnerKeysAccount() {
            let b = m.getBalance()
            list.append(AccountInfo(category: .providerOwnerKeys, label: "Provider Owner Keys", balance: b, addressCount: (0, 0)))
        }
        if let m = collection.getProviderOperatorKeysAccount() {
            let b = m.getBalance()
            list.append(AccountInfo(category: .providerOperatorKeys, label: "Provider Operator Keys (BLS)", balance: b, addressCount: (0, 0)))
        }
        if let m = collection.getProviderPlatformKeysAccount() {
            let b = m.getBalance()
            list.append(AccountInfo(category: .providerPlatformKeys, label: "Provider Platform Keys (EdDSA)", balance: b, addressCount: (0, 0)))
        }

        // Platform Payment (DIP-17)
        if collection.hasPlatformPaymentAccounts {
            for accountIdx in 0..<collection.platformPaymentCount {
                if let m = collection.getPlatformPaymentAccount(accountIndex: accountIdx, keyClass: 0) {
                    var addrCount = 0
                    if let pool = m.getAddressPool(), let infos = try? pool.getAddresses(from: 0, to: 1000) {
                        addrCount = infos.count
                    }
                    list.append(AccountInfo(category: .platformPayment, index: accountIdx, label: "Platform Payment \(accountIdx)", balance: Balance(confirmed: 0, unconfirmed: 0), addressCount: (addrCount, 0)))
                }
            }
        }

        // Sort BIP44 by index first, then other types below
        list.sort { (a, b) in
            switch (a.category, b.category) {
            case (.bip44, .bip44): return (a.index ?? 0) < (b.index ?? 0)
            default: return a.label < b.label
            }
        }
        return list
    }

    // MARK: - Private Methods

    private func loadWallets() async {
        var wallets: [HDWallet] = []
        do {
            let descriptor = FetchDescriptor<HDWallet>(sortBy: [SortDescriptor(\.createdAt)])
            wallets = try modelContainer.mainContext.fetch(descriptor)
        } catch {
            self.error = WalletError.databaseError(error.localizedDescription)
            return
        }

        // Try to import each wallet into the FFI wallet manager
        // If it succeeds, we store the HDWallet for later querying. If it fails,
        // we log the error and remove that wallet from the database.
        for wallet in wallets {
            do {
                let restoredWalletId = try sdkWalletManager.importWallet(from: wallet.serializedWalletBytes)

                // Update wallet ID if it changed (shouldn't happen, but good to verify)
                if wallet.walletId != restoredWalletId {
                    print("Warning: Wallet ID changed during restoration. Old: \(wallet.walletId.hexString), New: \(restoredWalletId.hexString)")
                    wallet.walletId = restoredWalletId
                }

                self.wallets.append(wallet)

                print("Successfully restored wallet '\(wallet.label)' to FFI wallet manager")
            } catch {
                modelContainer.mainContext.delete(wallet)
            }
        }

        try? modelContainer.mainContext.save()
    }
}

// MARK: - Wallet Errors

public enum WalletError: LocalizedError {
    case invalidMnemonic
    case seedGenerationFailed
    case seedNotAvailable
    case watchOnlyWallet
    case addressGenerationFailed
    case invalidDerivationPath
    case databaseError(String)
    case notImplemented(String)
    case walletError(String)
    case invalidInput(String)

    public var errorDescription: String? {
        switch self {
        case .invalidMnemonic:
            return "Invalid mnemonic phrase"
        case .seedGenerationFailed:
            return "Failed to generate seed from mnemonic"
        case .seedNotAvailable:
            return "Seed not available for this wallet"
        case .watchOnlyWallet:
            return "Operation not available for watch-only wallet"
        case .addressGenerationFailed:
            return "Failed to generate address"
        case .invalidDerivationPath:
            return "Invalid derivation path"
        case .databaseError(let message):
            return "Database error: \(message)"
        case .notImplemented(let feature):
            return "\(feature) not implemented yet"
        case .walletError(let message):
            return "Wallet error: \(message)"
        case .invalidInput(let message):
            return "Invalid input: \(message)"
        }
    }
}
