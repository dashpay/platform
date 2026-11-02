import Foundation
import DashSDKFFI

/// Key derivation utilities
public class KeyDerivation {

    /// Create a new master extended private key from seed
    /// - Parameters:
    ///   - seed: The seed bytes
    ///   - network: The network type
    /// - Returns: Extended private key handle
    public static func createMasterKey(seed: Data, network: Network = .mainnet) throws -> ExtendedPrivateKey {
        var error = FFIError()

        let xprivPtr = seed.withUnsafeBytes { seedBytes in
            let seedPtr = seedBytes.bindMemory(to: UInt8.self).baseAddress
            return derivation_new_master_key(seedPtr, seed.count, network.ffiValue, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard let handle = xprivPtr else {
            throw KeyWalletError(ffiError: error)
        }

        return ExtendedPrivateKey(handle: handle)
    }

    /// Get BIP44 account path
    /// - Parameters:
    ///   - network: The network type
    ///   - accountIndex: The account index
    /// - Returns: The derivation path string
    public static func getBIP44AccountPath(network: Network = .mainnet,
                                          accountIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }

        let success = derivation_bip44_account_path(
            network.ffiValue, accountIndex, pathBuffer, maxPathLen, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return String(cString: pathBuffer)
    }

    /// Get BIP44 payment path
    /// - Parameters:
    ///   - network: The network type
    ///   - accountIndex: The account index
    ///   - isChange: Whether this is a change address
    ///   - addressIndex: The address index
    /// - Returns: The derivation path string
    public static func getBIP44PaymentPath(network: Network = .mainnet,
                                          accountIndex: UInt32,
                                          isChange: Bool,
                                          addressIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }

        let success = derivation_bip44_payment_path(
            network.ffiValue, accountIndex, isChange, addressIndex,
            pathBuffer, maxPathLen, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return String(cString: pathBuffer)
    }

    /// Get CoinJoin path
    /// - Parameters:
    ///   - network: The network type
    ///   - accountIndex: The account index
    /// - Returns: The derivation path string
    public static func getCoinJoinPath(network: Network = .mainnet,
                                      accountIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }

        let success = derivation_coinjoin_path(
            network.ffiValue, accountIndex, pathBuffer, maxPathLen, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return String(cString: pathBuffer)
    }

    /// Get identity registration path
    /// - Parameters:
    ///   - network: The network type
    ///   - identityIndex: The identity index
    /// - Returns: The derivation path string
    public static func getIdentityRegistrationPath(network: Network = .mainnet,
                                                  identityIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }

        let success = derivation_identity_registration_path(
            network.ffiValue, identityIndex, pathBuffer, maxPathLen, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return String(cString: pathBuffer)
    }

    /// Get identity top-up path
    /// - Parameters:
    ///   - network: The network type
    ///   - identityIndex: The identity index
    ///   - topupIndex: The top-up index
    /// - Returns: The derivation path string
    public static func getIdentityTopUpPath(network: Network = .mainnet,
                                           identityIndex: UInt32,
                                           topupIndex: UInt32) throws -> String {
        var error = FFIError()
        let maxPathLen = 256
        let pathBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: maxPathLen)
        defer {
            pathBuffer.deallocate()
        }

        let success = derivation_identity_topup_path(
            network.ffiValue, identityIndex, topupIndex,
            pathBuffer, maxPathLen, &error)

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return String(cString: pathBuffer)
    }

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

    /// An account whose derivation path `accountPath` returns.
    public enum Account {
        case bip44(index: UInt32)
        case providerVotingKeys
        case providerOwnerKeys
    }

    /// The derivation path of `account` on `network`, such as `m/44'/5'/0'`
    /// for BIP-44 account 0 or `m/9'/5'/3'/1'` for the provider voting keys on
    /// mainnet. The path comes from key-wallet's account definitions.
    public static func accountPath(_ account: Account, network: Network = .mainnet) throws -> String {
        var spec = AccountSpecFFI()
        switch account {
        case let .bip44(index):
            spec.type_tag = UInt8(ACCOUNT_TYPE_TAG_FFI_STANDARD.rawValue)
            spec.standard_tag = UInt8(STANDARD_ACCOUNT_TYPE_TAG_FFI_BIP44.rawValue)
            spec.index = index
        case .providerVotingKeys:
            spec.type_tag = UInt8(ACCOUNT_TYPE_TAG_FFI_PROVIDER_VOTING_KEYS.rawValue)
        case .providerOwnerKeys:
            spec.type_tag = UInt8(ACCOUNT_TYPE_TAG_FFI_PROVIDER_OWNER_KEYS.rawValue)
        }
        return try ownedString("platform_wallet_account_path") { out in
            try platform_wallet_account_path(network.ffiValue, &spec, &out).check()
        }
    }

    /// The BIP-44 P2PKH address of `mnemonic` at the given account, chain and
    /// index. Only the address crosses into Swift, never the seed or a key.
    public static func bip44Address(mnemonic: String,
                                    network: Network = .mainnet,
                                    accountIndex: UInt32,
                                    isChange: Bool,
                                    addressIndex: UInt32) throws -> String {
        try requireNoNUL(mnemonic, "Mnemonic")
        return try mnemonic.withCString { cMnemonic in
            try ownedString("platform_wallet_bip44_address_from_mnemonic") { out in
                try platform_wallet_bip44_address_from_mnemonic(
                    cMnemonic, network.ffiValue, accountIndex, isChange, addressIndex, &out
                ).check()
            }
        }
    }

    /// The P2PKH addresses of `mnemonic` at each of `paths`, in the same order.
    /// The seed is computed once for the whole list, and only the addresses
    /// cross into Swift.
    public static func addresses(mnemonic: String,
                                 network: Network = .mainnet,
                                 paths: [String]) throws -> [String] {
        try requireNoNUL(mnemonic, "Mnemonic")
        // The FFI takes the paths as one newline-separated string.
        guard !paths.contains(where: { $0.utf8.contains(0) || $0.utf8.contains(0x0A) }) else {
            throw PlatformWalletError.invalidParameter("Paths must not contain NUL or a newline")
        }
        guard !paths.isEmpty else { return [] }
        let joined = try mnemonic.withCString { cMnemonic in
            try paths.joined(separator: "\n").withCString { cPaths in
                try ownedString("platform_wallet_addresses_from_mnemonic") { out in
                    try platform_wallet_addresses_from_mnemonic(
                        cMnemonic, network.ffiValue, cPaths, &out
                    ).check()
                }
            }
        }
        let addresses = joined
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map(String.init)
        guard addresses.count == paths.count else {
            throw PlatformWalletError.unknown(
                "platform_wallet_addresses_from_mnemonic returned \(addresses.count) addresses for \(paths.count) paths"
            )
        }
        return addresses
    }

    /// The private key of `mnemonic` at the derivation `path`, as hex and WIF.
    public static func privateKey(mnemonic: String,
                                  network: Network = .mainnet,
                                  path: String) throws -> ManagedPlatformWallet.CoreAddressPrivateKey {
        try requireNoNUL(mnemonic, "Mnemonic")
        try requireNoNUL(path, "Path")
        var out = AddressPrivateKeyFFI()
        // The free function zeroizes the Rust-owned strings and no-ops on the
        // empty struct a failed call leaves behind.
        defer { platform_wallet_address_private_key_free(&out) }
        try mnemonic.withCString { cMnemonic in
            try path.withCString { cPath in
                try platform_wallet_private_key_from_mnemonic(
                    cMnemonic, network.ffiValue, cPath, &out
                ).check()
            }
        }
        guard let hex = out.private_key_hex, let wif = out.private_key_wif else {
            throw PlatformWalletError.nullPointer(
                "platform_wallet_private_key_from_mnemonic returned a NULL key pointer"
            )
        }
        return ManagedPlatformWallet.CoreAddressPrivateKey(
            hex: String(cString: hex), wif: String(cString: wif))
    }

    /// The base58 extended public key of `mnemonic` at the derivation `path`,
    /// such as the BIP-44 account path `m/44'/5'/0'`. Only the public key
    /// crosses into Swift.
    public static func xpub(mnemonic: String,
                            network: Network = .mainnet,
                            path: String) throws -> String {
        try requireNoNUL(mnemonic, "Mnemonic")
        try requireNoNUL(path, "Path")
        return try mnemonic.withCString { cMnemonic in
            try path.withCString { cPath in
                try ownedString("platform_wallet_xpub_from_mnemonic") { out in
                    try platform_wallet_xpub_from_mnemonic(
                        cMnemonic, network.ffiValue, cPath, &out
                    ).check()
                }
            }
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
    /// with `platform_wallet_free_string`.
    private static func ownedString(
        _ function: String,
        _ call: (inout UnsafeMutablePointer<CChar>?) throws -> Void
    ) throws -> String {
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        try call(&outPtr)
        guard let ptr = outPtr else {
            throw PlatformWalletError.nullPointer("\(function) returned a NULL string pointer")
        }
        defer { platform_wallet_free_string(ptr) }
        return String(cString: ptr)
    }

    /// Parse a derivation path string to indices
    /// - Parameter path: The derivation path string
    /// - Returns: Tuple of (indices, hardened flags)
    public static func parsePath(_ path: String) throws -> (indices: [UInt32], hardened: [Bool]) {
        var error = FFIError()
        var indicesPtr: UnsafeMutablePointer<UInt32>?
        var hardenedPtr: UnsafeMutablePointer<Bool>?
        var count: size_t = 0

        let success = path.withCString { pathCStr in
            derivation_path_parse(pathCStr, &indicesPtr, &hardenedPtr, &count, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let indices = indicesPtr, let hardened = hardenedPtr {
                derivation_path_free(indices, hardened, count)
            }
        }

        guard success, let indices = indicesPtr, let hardened = hardenedPtr else {
            throw KeyWalletError(ffiError: error)
        }

        // Copy the data before freeing
        var indicesArray: [UInt32] = []
        var hardenedArray: [Bool] = []

        for i in 0..<count {
            indicesArray.append(indices[i])
            hardenedArray.append(hardened[i])
        }

        return (indices: indicesArray, hardened: hardenedArray)
    }
}

/// Extended private key handle
public class ExtendedPrivateKey {
    private let handle: OpaquePointer

    internal init(handle: OpaquePointer) {
        self.handle = handle
    }

    deinit {
        derivation_xpriv_free(handle)
    }

    /// Convert to extended public key
    public func toPublicKey() throws -> ExtendedPublicKey {
        var error = FFIError()
        guard let xpubHandle = derivation_xpriv_to_xpub(handle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        return ExtendedPublicKey(handle: xpubHandle)
    }

    /// Get string representation
    public func toString() throws -> String {
        var error = FFIError()
        guard let strPtr = derivation_xpriv_to_string(handle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        let str = String(cString: strPtr)
        derivation_string_free(strPtr)
        return str
    }
}

/// Extended public key handle
public class ExtendedPublicKey {
    private let handle: OpaquePointer

    internal init(handle: OpaquePointer) {
        self.handle = handle
    }

    deinit {
        derivation_xpub_free(handle)
    }

    /// Get string representation
    public func toString() throws -> String {
        var error = FFIError()
        guard let strPtr = derivation_xpub_to_string(handle, &error) else {
            defer {
                if error.message != nil {
                    error_message_free(error.message)
                }
            }
            throw KeyWalletError(ffiError: error)
        }

        let str = String(cString: strPtr)
        derivation_string_free(strPtr)
        return str
    }

    /// Get fingerprint (4 bytes)
    public func getFingerprint() throws -> Data {
        var error = FFIError()
        var fingerprint = Data(count: 4)

        let success = fingerprint.withUnsafeMutableBytes { bytes in
            let ptr = bytes.bindMemory(to: UInt8.self).baseAddress
            return derivation_xpub_fingerprint(handle, ptr, &error)
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }

        guard success else {
            throw KeyWalletError(ffiError: error)
        }

        return fingerprint
    }
}
