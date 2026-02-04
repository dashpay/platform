import Foundation

// MARK: - Key Format

/// Detected format of a private key input.
public enum PrivateKeyFormat {
    case hex           // 64 hex characters (32 bytes)
    case wif           // WIF-encoded private key
    case unknown       // Could not detect format
}

// MARK: - Key Format Detector

/// Detects the format of a private key string.
public enum KeyFormatDetector {

    private static let hexCharacterSet = CharacterSet(charactersIn: "0123456789abcdefABCDEF")

    /// Detects the format of a private key string.
    /// - Parameter input: The private key string to analyze.
    /// - Returns: The detected format.
    public static func detectFormat(_ input: String) -> PrivateKeyFormat {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return .unknown }

        // Check for hex format (64 hex characters = 32 bytes)
        if trimmed.count == 64 && isHex(trimmed) {
            return .hex
        }

        // Check for WIF format (starts with specific characters)
        if isLikelyWIF(trimmed) {
            return .wif
        }

        return .unknown
    }

    /// Checks if a string contains only hex characters.
    /// - Parameter string: The string to check.
    /// - Returns: True if the string is valid hex.
    public static func isHex(_ string: String) -> Bool {
        string.unicodeScalars.allSatisfy { hexCharacterSet.contains($0) }
    }

    /// Checks if a string looks like a WIF-encoded key.
    /// - Parameter string: The string to check.
    /// - Returns: True if the string appears to be WIF format.
    public static func isLikelyWIF(_ string: String) -> Bool {
        guard string.count >= 51 && string.count <= 52 else { return false }

        // WIF keys start with specific characters:
        // - Mainnet uncompressed: '5'
        // - Mainnet compressed: 'K' or 'L'
        // - Testnet uncompressed: '9'
        // - Testnet compressed: 'c'
        // - Dash mainnet: '7' or 'X'
        // - Dash testnet: 'c' or 'X'
        let firstChar = string.first!
        let wifPrefixes: Set<Character> = ["5", "K", "L", "9", "c", "7", "X"]
        return wifPrefixes.contains(firstChar)
    }
}

// MARK: - Private Key Parser

/// Parses private keys from various formats (hex, WIF).
public enum PrivateKeyParser {

    /// Parse result with optional error message.
    public struct ParseResult {
        public let data: Data?
        public let format: PrivateKeyFormat
        public let error: String?

        public var isValid: Bool { data != nil }

        public static func success(_ data: Data, format: PrivateKeyFormat) -> ParseResult {
            ParseResult(data: data, format: format, error: nil)
        }

        public static func failure(_ error: String, format: PrivateKeyFormat = .unknown) -> ParseResult {
            ParseResult(data: nil, format: format, error: error)
        }
    }

    /// Parse a private key from hex or WIF format.
    /// - Parameter input: The private key string.
    /// - Returns: ParseResult with the key data or error message.
    public static func parse(_ input: String) -> ParseResult {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return .failure("Private key is empty")
        }

        let format = KeyFormatDetector.detectFormat(trimmed)

        switch format {
        case .hex:
            return parseHex(trimmed)
        case .wif:
            return parseWIF(trimmed)
        case .unknown:
            // Try both formats
            if let hexResult = tryParseHex(trimmed) {
                return .success(hexResult, format: .hex)
            }
            if let wifResult = WIFParser.parseWIF(trimmed) {
                return .success(wifResult, format: .wif)
            }
            return .failure("Invalid private key format. Expected 64 hex characters or WIF format.")
        }
    }

    /// Parse a hex-encoded private key.
    /// - Parameter hex: The hex string (must be 64 characters).
    /// - Returns: ParseResult with the key data or error message.
    public static func parseHex(_ hex: String) -> ParseResult {
        let trimmed = hex.trimmingCharacters(in: .whitespacesAndNewlines)

        guard trimmed.count == 64 else {
            return .failure("Hex private key must be 64 characters (32 bytes), got \(trimmed.count)")
        }

        guard KeyFormatDetector.isHex(trimmed) else {
            return .failure("Invalid hex characters in private key")
        }

        guard let data = AddressTransformer.hexToData(trimmed) else {
            return .failure("Failed to parse hex private key")
        }

        return .success(data, format: .hex)
    }

    /// Parse a WIF-encoded private key.
    /// - Parameter wif: The WIF string.
    /// - Returns: ParseResult with the key data or error message.
    public static func parseWIF(_ wif: String) -> ParseResult {
        let trimmed = wif.trimmingCharacters(in: .whitespacesAndNewlines)

        guard let data = WIFParser.parseWIF(trimmed) else {
            return .failure("Invalid WIF format or checksum")
        }

        guard data.count == 32 else {
            return .failure("WIF decoded to \(data.count) bytes, expected 32")
        }

        return .success(data, format: .wif)
    }

    /// Try to parse a hex string without strict validation.
    private static func tryParseHex(_ input: String) -> Data? {
        guard input.count == 64, KeyFormatDetector.isHex(input) else { return nil }
        return AddressTransformer.hexToData(input)
    }
}

// MARK: - Key Validator

/// Validates private keys and matches them to public keys.
public enum KeyValidator {

    /// Validation result with details.
    public struct ValidationResult {
        public let isValid: Bool
        public let matchedKey: IdentityPublicKey?
        public let error: String?

        public static func valid(matchedKey: IdentityPublicKey) -> ValidationResult {
            ValidationResult(isValid: true, matchedKey: matchedKey, error: nil)
        }

        public static func invalid(_ error: String) -> ValidationResult {
            ValidationResult(isValid: false, matchedKey: nil, error: error)
        }
    }

    /// Validates a private key against an identity's public keys.
    /// - Parameters:
    ///   - privateKey: The private key data (32 bytes).
    ///   - publicKeys: List of public keys to match against.
    ///   - isTestnet: Whether to use testnet parameters.
    /// - Returns: ValidationResult with the matched key or error.
    public static func validatePrivateKey(
        _ privateKey: Data,
        against publicKeys: [IdentityPublicKey],
        isTestnet: Bool = true
    ) -> ValidationResult {
        guard privateKey.count == 32 else {
            return .invalid("Private key must be 32 bytes, got \(privateKey.count)")
        }

        guard !publicKeys.isEmpty else {
            return .invalid("No public keys to validate against")
        }

        if let matchedKey = KeyValidation.matchPrivateKeyToPublicKeys(
            privateKeyData: privateKey,
            publicKeys: publicKeys,
            isTestnet: isTestnet
        ) {
            return .valid(matchedKey: matchedKey)
        }

        return .invalid("Private key does not match any public key")
    }

    /// Validates a private key string against an identity's public keys.
    /// - Parameters:
    ///   - privateKeyInput: The private key string (hex or WIF).
    ///   - publicKeys: List of public keys to match against.
    ///   - isTestnet: Whether to use testnet parameters.
    /// - Returns: ValidationResult with the matched key or error.
    public static func validatePrivateKeyInput(
        _ privateKeyInput: String,
        against publicKeys: [IdentityPublicKey],
        isTestnet: Bool = true
    ) -> ValidationResult {
        let parseResult = PrivateKeyParser.parse(privateKeyInput)

        guard let privateKey = parseResult.data else {
            return .invalid(parseResult.error ?? "Failed to parse private key")
        }

        return validatePrivateKey(privateKey, against: publicKeys, isTestnet: isTestnet)
    }
}

// MARK: - Key Size Validator

/// Validates key sizes for different key types.
public enum KeySizeValidator {

    /// Expected private key size for a given key type.
    /// - Parameter keyType: The type of key.
    /// - Returns: Expected size in bytes.
    public static func expectedPrivateKeySize(for keyType: KeyType) -> Int {
        switch keyType {
        case .ecdsaSecp256k1:
            return 32  // 256 bits
        case .bls12_381:
            return 32  // 256 bits
        case .ecdsaHash160:
            return 32  // 256 bits for the actual key
        case .bip13ScriptHash:
            return 32  // 256 bits
        case .eddsa25519Hash160:
            return 32  // 256 bits
        }
    }

    /// Validates that a private key has the correct size for its type.
    /// - Parameters:
    ///   - privateKey: The private key data.
    ///   - keyType: The type of key.
    /// - Returns: True if the size is correct.
    public static func isValidSize(_ privateKey: Data, for keyType: KeyType) -> Bool {
        privateKey.count == expectedPrivateKeySize(for: keyType)
    }
}

// MARK: - Key Formatter

/// Formats keys for display and export.
public enum KeyFormatter {

    /// Format a private key as hex string.
    /// - Parameter privateKey: The private key data.
    /// - Returns: Hex string representation.
    public static func toHex(_ privateKey: Data) -> String {
        AddressTransformer.dataToHex(privateKey)
    }

    /// Format a private key as WIF string.
    /// - Parameters:
    ///   - privateKey: The private key data.
    ///   - isTestnet: Whether to encode for testnet.
    /// - Returns: WIF string or nil if encoding fails.
    public static func toWIF(_ privateKey: Data, isTestnet: Bool = true) -> String? {
        WIFParser.encodeToWIF(privateKey, isTestnet: isTestnet)
    }

    /// Format a public key for display.
    /// - Parameters:
    ///   - publicKey: The public key.
    ///   - truncate: If true, shows only first and last 8 characters.
    /// - Returns: Formatted string.
    public static func formatPublicKey(_ publicKey: IdentityPublicKey, truncate: Bool = false) -> String {
        let hex = AddressTransformer.dataToHex(publicKey.data)
        if truncate && hex.count > 20 {
            return "\(hex.prefix(8))...\(hex.suffix(8))"
        }
        return hex
    }

    /// Format key information for display.
    /// - Parameter publicKey: The public key to format.
    /// - Returns: Multi-line description of the key.
    public static func formatKeyInfo(_ publicKey: IdentityPublicKey) -> String {
        """
        Key ID: #\(publicKey.id)
        Purpose: \(publicKey.purpose.name)
        Type: \(publicKey.keyType.name)
        Security: \(publicKey.securityLevel.name)
        """
    }
}
