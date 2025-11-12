import Foundation
import DashSDKFFI

/// Utility class for mnemonic operations
public class Mnemonic {
    
    /// Generate a new mnemonic phrase
    /// - Parameters:
    ///   - wordCount: Number of words (12, 15, 18, 21, or 24)
    ///   - language: The language for the mnemonic
    /// - Returns: The generated mnemonic phrase
    public static func generate(wordCount: UInt32 = 24,
                               language: MnemonicLanguage = .english) throws -> String {
        guard [12, 15, 18, 21, 24].contains(wordCount) else {
            throw KeyWalletError.invalidInput("Word count must be 12, 15, 18, 21, or 24")
        }
        
        var error = FFIError()
        let mnemonicPtr = mnemonic_generate_with_language(wordCount, language.ffiValue, &error)
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard let ptr = mnemonicPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        let mnemonic = String(cString: ptr)
        mnemonic_free(ptr)
        
        return mnemonic
    }
    
    /// Validate a mnemonic phrase
    /// - Parameter mnemonic: The mnemonic phrase to validate
    /// - Returns: True if valid
    public static func validate(_ mnemonic: String) -> Bool {
        var error = FFIError()
        
        let isValid = mnemonic.withCString { mnemonicCStr in
            mnemonic_validate(mnemonicCStr, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        return isValid
    }
    
    /// Convert mnemonic to seed
    /// - Parameters:
    ///   - mnemonic: The mnemonic phrase
    ///   - passphrase: Optional BIP39 passphrase
    /// - Returns: The seed data (typically 64 bytes)
    public static func toSeed(mnemonic: String, passphrase: String? = nil) throws -> Data {
        var error = FFIError()
        var seed = Data(count: 64)
        var seedLen: size_t = 64
        
        let success = mnemonic.withCString { mnemonicCStr in
            seed.withUnsafeMutableBytes { seedBytes in
                let seedPtr = seedBytes.bindMemory(to: UInt8.self).baseAddress
                
                if let passphrase = passphrase {
                    return passphrase.withCString { passphraseCStr in
                        mnemonic_to_seed(mnemonicCStr, passphraseCStr,
                                       seedPtr, &seedLen, &error)
                    }
                } else {
                    return mnemonic_to_seed(mnemonicCStr, nil,
                                          seedPtr, &seedLen, &error)
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
        
        // Resize if necessary
        if seedLen < 64 {
            seed = seed.prefix(seedLen)
        }
        
        return seed
    }
    
    /// Get word count from a mnemonic phrase
    /// - Parameter mnemonic: The mnemonic phrase
    /// - Returns: The number of words
    public static func wordCount(of mnemonic: String) throws -> UInt32 {
        var error = FFIError()
        
        let count = mnemonic.withCString { mnemonicCStr in
            mnemonic_word_count(mnemonicCStr, &error)
        }
        
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