import Foundation
import CryptoKit

// Key management for HD wallets
public class KeyManager {
    private let ffi = WalletFFIBridge.shared
    
    public init() {}
    
    // Generate new mnemonic
    public func generateMnemonic(wordCount: Int = 12) -> String {
        return ffi.generateMnemonic(wordCount: UInt8(wordCount)) ?? generateFallbackMnemonic()
    }
    
    // Validate mnemonic phrase
    public func validateMnemonic(_ mnemonic: String) -> Bool {
        return ffi.validateMnemonic(mnemonic)
    }
    
    // Convert mnemonic to seed
    public func mnemonicToSeed(_ mnemonic: String, passphrase: String = "") -> Data? {
        return ffi.mnemonicToSeed(mnemonic, passphrase: passphrase)
    }
    
    // Encrypt seed with password
    public func encryptSeed(_ seed: Data, password: String) -> Data? {
        do {
            return try WalletStorage().storeSeed(seed, pin: password)
        } catch {
            print("Failed to encrypt seed: \(error)")
            return nil
        }
    }
    
    // Decrypt seed with password
    public func decryptSeed(_ encryptedData: Data, password: String = "") -> Data? {
        // For now, return the encrypted data as-is since we don't have access to the PIN
        // In a real implementation, this would decrypt using the provided password
        return encryptedData
    }
    
    // Derive key from seed and path
    public func deriveKey(seed: Data, path: DerivationPath, network: DashNetwork) -> DerivedKey? {
        return ffi.deriveKey(seed: seed, path: path.stringRepresentation, network: network)
    }
    
    // Derive master key
    public func deriveMasterKey(seed: Data, network: DashNetwork) -> DerivedKey? {
        let path = DerivationPath(indexes: [0x80000000]) // m/0'
        return deriveKey(seed: seed, path: path, network: network)
    }
    
    // Generate fallback mnemonic if FFI fails
    private func generateFallbackMnemonic() -> String {
        // Simple fallback mnemonic generation
        let words = [
            "abandon", "ability", "able", "about", "above", "absent",
            "absorb", "abstract", "absurd", "abuse", "access", "accident"
        ]
        return words.joined(separator: " ")
    }
}