import Foundation
import DashSDKFFI

private func scrubMnemonicBytes(_ bytes: inout [UInt8]) {
    bytes.withUnsafeMutableBufferPointer { buffer in
        guard let base = buffer.baseAddress else { return }
        memset_s(base, buffer.count, 0, buffer.count)
    }
}

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
        try toSeed(mnemonicUTF8Bytes: Data(mnemonic.utf8), passphrase: passphrase)
    }

    /// Convert mnemonic UTF-8 bytes to seed without first
    /// materializing a Swift `String` at the call site.
    public static func toSeed(mnemonicUTF8Bytes: Data, passphrase: String? = nil) throws -> Data {
        guard !mnemonicUTF8Bytes.isEmpty else {
            throw KeyWalletError.invalidInput("Mnemonic must not be empty")
        }

        var error = FFIError()
        var seed = Data(count: 64)
        var seedLen: size_t = 64
        var mnemonicBytes = [UInt8](mnemonicUTF8Bytes)
        guard !mnemonicBytes.contains(0) else {
            scrubMnemonicBytes(&mnemonicBytes)
            throw KeyWalletError.invalidInput("Mnemonic bytes must not contain NUL")
        }
        mnemonicBytes.append(0)

        let success = mnemonicBytes.withUnsafeBufferPointer { mnemonicBuf in
            guard let mnemonicBase = mnemonicBuf.baseAddress else {
                return false
            }
            return mnemonicBase.withMemoryRebound(to: CChar.self, capacity: mnemonicBuf.count) { mnemonicCStr in
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
        }

        defer {
            scrubMnemonicBytes(&mnemonicBytes)
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

    // MARK: - Recover-flow word helpers (DashSync DSBIP39Mnemonic parity)

    /// `true` if `word` is a BIP-39 word in any bundled language
    /// (DashSync `wordIsValid:`). Exact wordlist membership — callers
    /// normalize first via `normalizePhrase` / `cleanupPhrase`.
    public static func wordIsValid(_ word: String) -> Bool {
        word.withCString { platform_wallet_mnemonic_word_is_valid($0) }
    }

    /// `true` if `word` is in the default (English) wordlist
    /// (DashSync `wordIsLocal:`).
    public static func wordIsLocal(_ word: String) -> Bool {
        word.withCString { platform_wallet_mnemonic_word_is_local($0) }
    }

    /// NFKD + lowercase + whitespace-collapse (DashSync `normalizePhrase:`).
    /// Returns the input unchanged on the UTF-8/allocation error paths that
    /// DashSync never hits for valid `NSString` input.
    public static func normalizePhrase(_ phrase: String) -> String {
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        do {
            try phrase.withCString { cPhrase in
                try platform_wallet_mnemonic_normalize_phrase(cPhrase, &outPtr).check()
            }
        } catch {
            return phrase
        }
        guard let cStr = outPtr else { return phrase }
        let result = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return result
    }

    /// Minimal cleanup + CJK ideographic auto-split (DashSync `cleanupPhrase:`).
    public static func cleanupPhrase(_ phrase: String) -> String {
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        do {
            try phrase.withCString { cPhrase in
                try platform_wallet_mnemonic_cleanup_phrase(cPhrase, &outPtr).check()
            }
        } catch {
            return phrase
        }
        guard let cStr = outPtr else { return phrase }
        let result = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return result
    }
}
