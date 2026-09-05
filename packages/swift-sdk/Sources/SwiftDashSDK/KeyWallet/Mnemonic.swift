import CommonCrypto
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
            let ffiFailure = KeyWalletError(ffiError: error)
            // `mnemonic_to_seed` parses with a hardcoded English wordlist, so
            // a phrase in any other supported language fails with
            // `invalidMnemonic` even though `validate` (which tries every
            // wordlist) accepts it. BIP-39 seed derivation itself is
            // wordlist-independent — PBKDF2-HMAC-SHA512 over the NFKD
            // sentence — so derive it directly for phrases that validate.
            if case .invalidMnemonic = ffiFailure,
               let fallbackSeed = multiLanguageSeed(
                   mnemonicUTF8Bytes: mnemonicUTF8Bytes,
                   passphrase: passphrase) {
                return fallbackSeed
            }
            throw ffiFailure
        }

        // Resize if necessary
        if seedLen < 64 {
            seed = seed.prefix(seedLen)
        }

        return seed
    }

    /// BIP-39 seed for a phrase in any supported wordlist: PBKDF2-HMAC-SHA512
    /// over the NFKD-canonicalized sentence, salt `"mnemonic" + NFKD(passphrase)`,
    /// 2048 rounds, 64 bytes — the derivation every BIP-39 wallet (including
    /// DashSync and Electrum) uses, which never needs the wordlist itself.
    /// Returns nil when the phrase does not validate against any supported
    /// language (callers surface the original FFI error).
    private static func multiLanguageSeed(mnemonicUTF8Bytes: Data, passphrase: String?) -> Data? {
        let phrase = String(decoding: mnemonicUTF8Bytes, as: UTF8.self)
        guard validate(phrase) else { return nil }

        // `normalizePhrase` (NFKD + lowercase + single-space separators)
        // reconstructs the canonical BIP-39 sentence for a validated phrase:
        // wordlists ship lowercase NFKD, and NFKD maps the ideographic space
        // to ASCII space, so this matches the seed the FFI derives for
        // English phrases and the seed other wallets derive for this phrase.
        var password = [UInt8](normalizePhrase(phrase).utf8)
        defer { scrubMnemonicBytes(&password) }
        // The passphrase is only NFKD-normalized — case and punctuation are
        // significant in BIP-39 passphrases. It is scrubbed like the phrase:
        // a passphrase can be as sensitive as the mnemonic itself.
        var salt = [UInt8](
            ("mnemonic" + (passphrase ?? "")).decomposedStringWithCompatibilityMapping.utf8)
        defer { scrubMnemonicBytes(&salt) }

        var derived = [UInt8](repeating: 0, count: 64)
        let status = password.withUnsafeBufferPointer { passwordBuf in
            salt.withUnsafeBufferPointer { saltBuf in
                derived.withUnsafeMutableBufferPointer { derivedBuf in
                    CCKeyDerivationPBKDF(
                        CCPBKDFAlgorithm(kCCPBKDF2),
                        UnsafeRawPointer(passwordBuf.baseAddress)?
                            .assumingMemoryBound(to: Int8.self),
                        passwordBuf.count,
                        saltBuf.baseAddress,
                        saltBuf.count,
                        CCPseudoRandomAlgorithm(kCCPRFHmacAlgSHA512),
                        2048,
                        derivedBuf.baseAddress,
                        derivedBuf.count)
                }
            }
        }
        guard status == kCCSuccess else { return nil }
        return Data(derived)
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

    // MARK: - Recover-flow word/phrase helpers (DashSync DSBIP39Mnemonic parity)

    /// Raw BIP-39 wordlist (2048 words) for `language` — the low-level
    /// primitive the recover flow composes its word-validity checks from. The
    /// any-language "valid" union and the per-language "local" check now live
    /// in the app, not the SDK (callers build `Set`s from these lists). Returns
    /// `[]` only on the UTF-8/allocation error paths that never occur for a
    /// real wordlist.
    public static func wordList(language: MnemonicLanguage) -> [String] {
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        // The FFI can only error on a NULL out-param or an interior NUL in the
        // result — neither is reachable here (we pass &outPtr; BIP-39 words
        // contain no NUL). A failure would be a real bug, so trap rather than
        // mask it.
        try! platform_wallet_mnemonic_word_list(language.ffiMnemonicValue, &outPtr).check()
        guard let cStr = outPtr else { return [] }
        let joined = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return joined.split(separator: "\n").map(String.init)
    }

    /// NFKD + lowercase + whitespace-collapse (DashSync `normalizePhrase:`).
    /// An embedded-NUL phrase passes through unchanged; otherwise the FFI
    /// cannot fail, so a failure would be a bug and traps.
    public static func normalizePhrase(_ phrase: String) -> String {
        // An embedded NUL truncates the C string at the first \0, so the FFI
        // would normalize only the prefix — a different phrase than passed in.
        // Pass through unchanged instead.
        guard !phrase.utf8.contains(0) else { return phrase }
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        // Past the NUL guard the FFI cannot fail (valid UTF-8 in, NUL-free NFKD
        // out, non-NULL out-param), so trap on the impossible error.
        try! phrase.withCString { cPhrase in
            try platform_wallet_mnemonic_normalize_phrase(cPhrase, &outPtr).check()
        }
        guard let cStr = outPtr else { return phrase }
        let result = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return result
    }

    /// Minimal cleanup + CJK ideographic auto-split (DashSync `cleanupPhrase:`).
    public static func cleanupPhrase(_ phrase: String) -> String {
        // An embedded NUL truncates the C string at the first \0, so the FFI
        // would clean only the prefix — a different phrase than passed in.
        // Pass through unchanged instead (matches the error fallback below).
        guard !phrase.utf8.contains(0) else { return phrase }
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        // Past the NUL guard the FFI cannot fail (valid UTF-8 in, NUL-free
        // output, non-NULL out-param), so trap on the impossible error.
        try! phrase.withCString { cPhrase in
            try platform_wallet_mnemonic_cleanup_phrase(cPhrase, &outPtr).check()
        }
        guard let cStr = outPtr else { return phrase }
        let result = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return result
    }
}
