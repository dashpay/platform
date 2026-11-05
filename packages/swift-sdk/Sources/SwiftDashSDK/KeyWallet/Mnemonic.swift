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
        try KeyDerivation.ownedString(
            "platform_wallet_mnemonic_generate", free: platform_wallet_mnemonic_free
        ) { out in
            try platform_wallet_mnemonic_generate(wordCount, language.rawValue, &out).check()
        }
    }

    /// Validate a mnemonic phrase in any supported BIP-39 language
    /// - Parameter mnemonic: The mnemonic phrase to validate
    /// - Returns: True if valid
    public static func validate(_ mnemonic: String) -> Bool {
        // An embedded NUL truncates the C string, so the FFI would judge only
        // the prefix.
        guard !mnemonic.utf8.contains(0) else { return false }
        let result = mnemonic.withCString { platform_wallet_mnemonic_validate($0) }
        return PlatformWalletResult(result).isSuccess
    }

    /// The id the platform wallet manager gives the wallet of `mnemonic` on
    /// `network`, computed without creating or registering a wallet.
    ///
    /// The id is network-scoped, so one phrase has a distinct id per network.
    public static func walletId(for mnemonic: String, network: Network) throws -> Data {
        try KeyDerivation.requireNoNUL(mnemonic, "Mnemonic")
        var walletId = Data(count: 32)
        try mnemonic.withCString { cMnemonic in
            try walletId.withUnsafeMutableBytes { raw in
                try platform_wallet_wallet_id_from_mnemonic(
                    cMnemonic,
                    network.ffiValue,
                    raw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                ).check()
            }
        }
        return walletId
    }

    /// The first receive address of `mnemonic` on `network`: index 0 of the
    /// external chain of BIP-44 account 0. Only the address crosses into Swift.
    public static func firstReceiveAddress(for mnemonic: String, network: Network) throws -> String {
        try KeyDerivation.requireNoNUL(mnemonic, "Mnemonic")
        return try mnemonic.withCString { cMnemonic in
            try KeyDerivation.ownedString("platform_wallet_first_receive_address_from_mnemonic") { out in
                try platform_wallet_first_receive_address_from_mnemonic(
                    cMnemonic, network.ffiValue, &out
                ).check()
            }
        }
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
        try! platform_wallet_mnemonic_word_list(language.rawValue, &outPtr).check()
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
