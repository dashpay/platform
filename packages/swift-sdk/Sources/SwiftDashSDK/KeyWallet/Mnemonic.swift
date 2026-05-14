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
        var cstr: UnsafeMutablePointer<CChar>? = nil
        let result = platform_wallet_generate_mnemonic(wordCount, language.ffiValue, &cstr)
        try result.check()
        guard let ptr = cstr else {
            throw PlatformWalletError.invalidParameter(
                "platform_wallet_generate_mnemonic returned a null pointer"
            )
        }
        let phrase = String(cString: ptr)
        platform_wallet_string_free(ptr)
        return phrase
    }
}

public enum MnemonicLanguage: UInt32 {
    case english = 0
    case chineseSimplified = 1
    case chineseTraditional = 2
    case czech = 3
    case french = 4
    case italian = 5
    case japanese = 6
    case korean = 7
    case portuguese = 8
    case spanish = 9

    var ffiValue: FFILanguage {
        FFILanguage(rawValue: self.rawValue)
    }

    init(ffiLanguage: FFILanguage) {
        self = MnemonicLanguage(rawValue: ffiLanguage.rawValue) ?? .english
    }
}
