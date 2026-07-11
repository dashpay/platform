import Foundation

// MARK: - Validation Result

/// Result of a validation check: either valid or a list of error messages.
public struct ValidationResult {
    public let isValid: Bool
    public let errors: [String]

    public static func valid() -> ValidationResult {
        ValidationResult(isValid: true, errors: [])
    }

    public static func invalid(_ errors: [String]) -> ValidationResult {
        ValidationResult(isValid: errors.isEmpty, errors: errors)
    }
}

// MARK: - Address Validator

/// Validates Platform address and key formats (hex, lengths).
public enum AddressValidator {

    private static let hexCharacterSet = CharacterSet(charactersIn: "0123456789abcdefABCDEF")

    /// Platform address in hex is 21 bytes = 42 hex characters.
    /// - Returns: `true` if string is exactly 42 hex characters.
    public static func validateHexAddress(_ hex: String) -> Bool {
        guard hex.count == 42 else { return false }
        return hex.unicodeScalars.allSatisfy { hexCharacterSet.contains($0) }
    }

    /// Private key in hex is 32 bytes = 64 hex characters.
    /// - Returns: `true` if string is exactly 64 hex characters.
    public static func validatePrivateKey(_ hex: String) -> Bool {
        guard hex.count == 64 else { return false }
        return hex.unicodeScalars.allSatisfy { hexCharacterSet.contains($0) }
    }

    /// Generic hex string validation for a given byte length (hex length = 2 * byteLength).
    /// - Returns: `true` if string has expected length and only hex characters.
    public static func validateHexString(_ hex: String, byteLength: Int) -> Bool {
        let expectedCount = byteLength * 2
        guard hex.count == expectedCount else { return false }
        return hex.unicodeScalars.allSatisfy { hexCharacterSet.contains($0) }
    }

    /// Validates and parses an amount string.
    /// - Returns: `UInt64` if the string is a positive number, otherwise `nil`.
    public static func validateAmount(_ amount: String) -> UInt64? {
        guard let value = UInt64(amount.trimmingCharacters(in: .whitespaces)), value > 0 else {
            return nil
        }
        return value
    }

    /// Validates non-empty identity ID (base58 or hex format; format not checked in detail).
    public static func validateIdentityId(_ id: String) -> Bool {
        !id.trimmingCharacters(in: .whitespaces).isEmpty
    }

    /// Validates a bech32m Platform address (dash1... or tdash1...).
    /// - Returns: `true` if string is a valid bech32m address with correct HRP and 21 bytes of data.
    public static func validateBech32mAddress(_ address: String) -> Bool {
        Bech32m.isValidPlatformAddress(address)
    }

    /// Validates an address in either hex (42 chars) or bech32m format.
    /// - Returns: `true` if string is valid in either format.
    public static func validateAddress(_ address: String) -> Bool {
        let trimmed = address.trimmingCharacters(in: .whitespacesAndNewlines)
        if Bech32m.looksLikePlatformAddress(trimmed) {
            return validateBech32mAddress(trimmed)
        }
        return validateHexAddress(trimmed)
    }

    /// Validates a 32-byte hash in hex format (64 characters).
    /// - Returns: `true` if string is exactly 64 hex characters.
    public static func validateHash(_ hex: String) -> Bool {
        validateHexString(hex, byteLength: 32)
    }

    /// Validates an identity ID in hex format (64 characters = 32 bytes).
    /// - Returns: `true` if string is exactly 64 hex characters.
    public static func validateIdentityIdHex(_ hex: String) -> Bool {
        validateHexString(hex, byteLength: 32)
    }

    /// Checks if a string is a valid hex identity ID (64 chars).
    /// Useful for format detection (hex vs base58).
    /// - Returns: `true` if string is exactly 64 hex characters.
    public static func isHexIdentityId(_ id: String) -> Bool {
        validateHexString(id, byteLength: 32)
    }
}

// MARK: - Transfer Input Validator

/// Validates the full transfer-funds form (input address, private key, output address, amounts).
public enum TransferInputValidator {

    /// Validates transfer form fields.
    /// Optionally enforces inputAmount >= outputAmount (caller can add fee logic later).
    public static func validate(
        inputAddressHex: String,
        inputPrivateKeyHex: String,
        outputAddressHex: String,
        inputAmount: String,
        outputAmount: String
    ) -> ValidationResult {
        var errors: [String] = []

        if !AddressValidator.validateHexAddress(inputAddressHex) {
            errors.append("Input address must be 42 hex characters")
        }
        if !AddressValidator.validatePrivateKey(inputPrivateKeyHex) {
            errors.append("Private key must be 64 hex characters")
        }
        if !AddressValidator.validateHexAddress(outputAddressHex) {
            errors.append("Output address must be 42 hex characters")
        }
        if AddressValidator.validateAmount(inputAmount) == nil {
            errors.append("Input amount must be a positive number")
        }
        if AddressValidator.validateAmount(outputAmount) == nil {
            errors.append("Output amount must be a positive number")
        }

        if let inputAmt = AddressValidator.validateAmount(inputAmount),
           let outputAmt = AddressValidator.validateAmount(outputAmount),
           inputAmt < outputAmt {
            errors.append("Input amount must be greater than or equal to output amount")
        }

        return errors.isEmpty ? .valid() : .invalid(errors)
    }
}

// MARK: - Withdraw Input Validator

/// Validates the withdraw-funds form.
public enum WithdrawInputValidator {

    public static func validate(
        inputAddressHex: String,
        inputPrivateKeyHex: String,
        coreAddress: String,
        inputAmount: String,
        useChangeAddress: Bool,
        changeAddressHex: String
    ) -> ValidationResult {
        var errors: [String] = []

        if !AddressValidator.validateHexAddress(inputAddressHex) {
            errors.append("Input address must be 42 hex characters")
        }
        if !AddressValidator.validatePrivateKey(inputPrivateKeyHex) {
            errors.append("Private key must be 64 hex characters")
        }
        if coreAddress.trimmingCharacters(in: .whitespaces).isEmpty {
            errors.append("Core address is required")
        }
        if AddressValidator.validateAmount(inputAmount) == nil {
            errors.append("Input amount must be a positive number")
        }
        if useChangeAddress {
            if !AddressValidator.validateHexAddress(changeAddressHex) {
                errors.append("Change address must be 42 hex characters")
            }
        }

        return errors.isEmpty ? .valid() : .invalid(errors)
    }
}

// MARK: - Top-Up Address From Asset Lock Validator

/// Asset lock proof type for top-up validation.
public enum AssetLockProofTypeForValidation {
    case instant
    case chain
}

/// Validates the top-up-address-from-asset-lock form.
public enum TopUpAddressFromAssetLockValidator {

    /// Outpoint in hex is 36 bytes = 72 hex characters.
    public static func validateOutpointHex(_ hex: String) -> Bool {
        AddressValidator.validateHexString(hex, byteLength: 36)
    }

    public static func validate(
        outputAddressHex: String,
        assetLockPrivateKeyHex: String,
        proofType: AssetLockProofTypeForValidation,
        instantLockHex: String,
        transactionHex: String,
        outputIndex: String,
        coreChainLockedHeight: String,
        outPointHex: String
    ) -> ValidationResult {
        var errors: [String] = []

        if !AddressValidator.validateHexAddress(outputAddressHex) {
            errors.append("Output address must be 42 hex characters")
        }
        if !AddressValidator.validatePrivateKey(assetLockPrivateKeyHex) {
            errors.append("Asset lock private key must be 64 hex characters")
        }

        switch proofType {
        case .instant:
            if instantLockHex.trimmingCharacters(in: .whitespaces).isEmpty {
                errors.append("Instant lock hex is required")
            }
            if transactionHex.trimmingCharacters(in: .whitespaces).isEmpty {
                errors.append("Transaction hex is required")
            }
            if UInt32(outputIndex.trimmingCharacters(in: .whitespaces)) == nil {
                errors.append("Output index must be a valid number")
            }
        case .chain:
            if coreChainLockedHeight.trimmingCharacters(in: .whitespaces).isEmpty {
                errors.append("Core chain locked height is required")
            }
            if UInt32(coreChainLockedHeight.trimmingCharacters(in: .whitespaces)) == nil {
                errors.append("Core chain locked height must be a valid number")
            }
            if !validateOutpointHex(outPointHex) {
                errors.append("Outpoint must be 72 hex characters")
            }
        }

        return errors.isEmpty ? .valid() : .invalid(errors)
    }
}

// MARK: - Identity Top-Up From Addresses Validator

/// Validates the identity top-up-from-addresses form.
public enum IdentityTopUpFromAddressesValidator {

    public static func validate(
        identityId: String,
        inputAddressHex: String,
        inputPrivateKeyHex: String,
        amount: String
    ) -> ValidationResult {
        var errors: [String] = []

        if !AddressValidator.validateIdentityId(identityId) {
            errors.append("Identity ID is required")
        }
        if !AddressValidator.validateHexAddress(inputAddressHex) {
            errors.append("Input address must be 42 hex characters")
        }
        if !AddressValidator.validatePrivateKey(inputPrivateKeyHex) {
            errors.append("Private key must be 64 hex characters")
        }
        if AddressValidator.validateAmount(amount) == nil {
            errors.append("Amount must be a positive number")
        }

        return errors.isEmpty ? .valid() : .invalid(errors)
    }
}

// MARK: - Identity Transfer To Addresses Validator

/// Validates the identity transfer-to-addresses form.
public enum IdentityTransferToAddressesValidator {

    public static func validate(
        identityId: String,
        outputAddressHex: String,
        identityPrivateKeyHex: String,
        amount: String
    ) -> ValidationResult {
        var errors: [String] = []

        if !AddressValidator.validateIdentityId(identityId) {
            errors.append("Identity ID is required")
        }
        if !AddressValidator.validateHexAddress(outputAddressHex) {
            errors.append("Output address must be 42 hex characters")
        }
        if !AddressValidator.validatePrivateKey(identityPrivateKeyHex) {
            errors.append("Identity private key must be 64 hex characters")
        }
        if AddressValidator.validateAmount(amount) == nil {
            errors.append("Amount must be a positive number")
        }

        return errors.isEmpty ? .valid() : .invalid(errors)
    }
}

// MARK: - Identity Create From Addresses Validator

/// Validates the identity create-from-addresses form.
public enum IdentityCreateFromAddressesValidator {

    public static func validate(
        identityId: String,
        inputAddressHex: String,
        inputPrivateKeyHex: String,
        identityPrivateKeyHex: String,
        amount: String,
        nonce: String,
        useChangeAddress: Bool,
        changeAddressHex: String
    ) -> ValidationResult {
        var errors: [String] = []

        if !AddressValidator.validateIdentityId(identityId) {
            errors.append("Identity ID is required")
        }
        if !AddressValidator.validateHexAddress(inputAddressHex) {
            errors.append("Input address must be 42 hex characters")
        }
        if !AddressValidator.validatePrivateKey(inputPrivateKeyHex) {
            errors.append("Input private key must be 64 hex characters")
        }
        if !AddressValidator.validatePrivateKey(identityPrivateKeyHex) {
            errors.append("Identity private key must be 64 hex characters")
        }
        if AddressValidator.validateAmount(amount) == nil {
            errors.append("Amount must be a positive number")
        }
        if UInt32(nonce.trimmingCharacters(in: .whitespaces)) == nil {
            errors.append("Nonce must be a valid number")
        }
        if useChangeAddress, !AddressValidator.validateHexAddress(changeAddressHex) {
            errors.append("Change address must be 42 hex characters")
        }

        return errors.isEmpty ? .valid() : .invalid(errors)
    }
}
