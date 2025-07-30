import Foundation

// MARK: - Core SDK Wrapper

public class CoreSDKWrapper {
    public static let shared = CoreSDKWrapper()
    
    private let ffi = WalletFFIBridge.shared
    
    private init() {}
    
    // MARK: - Mnemonic Operations
    
    public func generateMnemonic(wordCount: Int = 12) -> String? {
        return ffi.generateMnemonic(wordCount: UInt8(wordCount))
    }
    
    public func validateMnemonic(_ mnemonic: String) -> Bool {
        return ffi.validateMnemonic(mnemonic)
    }
    
    public func mnemonicToSeed(_ mnemonic: String, passphrase: String = "") -> Data? {
        return ffi.mnemonicToSeed(mnemonic, passphrase: passphrase)
    }
    
    // MARK: - Key Operations
    
    public func derivePublicKey(from privateKey: Data) -> Data? {
        // Derive a key at a dummy path just to get the public key
        let seed = Data(repeating: 0, count: 64)
        guard let derived = ffi.deriveKey(seed: seed, path: "m/0", network: .testnet) else {
            return nil
        }
        return derived.publicKey
    }
    
    public func addPrivateKeys(_ key1: Data, _ key2: Data) -> Data? {
        // This would need to be implemented in the FFI layer
        // For now, return nil as it's not directly supported
        return nil
    }
    
    // MARK: - Transaction Operations
    
    public func signTransaction(_ transaction: Data, with privateKey: Data) -> Data? {
        // This would use the transaction signing FFI functions
        // For now, return nil as it needs proper transaction structure
        return nil
    }
    
    public func verifyTransaction(_ transaction: Data, signature: Data, publicKey: Data) -> Bool {
        // This would need to be implemented in the FFI layer
        return false
    }
    
    // MARK: - Address Operations
    
    public func validateAddress(_ address: String, network: DashNetwork) -> Bool {
        return ffi.validateAddress(address, network: network)
    }
}

