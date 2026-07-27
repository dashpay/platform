import XCTest
@testable import SwiftDashSDK

final class ValidationTests: XCTestCase {

    // MARK: - AddressValidator Tests

    func testValidateHexAddress_valid42CharHex() {
        // 42 hex characters = 21 bytes (valid Platform address)
        let validAddress = "00aaff11223344556677889900aabbccddeeff0011"
        XCTAssertTrue(AddressValidator.validateHexAddress(validAddress))
    }

    func testValidateHexAddress_invalidLength() {
        // Too short
        XCTAssertFalse(AddressValidator.validateHexAddress("00aabb"))
        // Too long
        XCTAssertFalse(AddressValidator.validateHexAddress("00aaff11223344556677889900aabbccddeeff001122"))
        // Empty
        XCTAssertFalse(AddressValidator.validateHexAddress(""))
    }

    func testValidateHexAddress_invalidCharacters() {
        // Contains 'g' which is not valid hex
        let invalidHex = "00aaff11223344556677889900aabbccddeeff00gg"
        XCTAssertFalse(AddressValidator.validateHexAddress(invalidHex))
        // Contains spaces
        XCTAssertFalse(AddressValidator.validateHexAddress("00 aabb"))
    }

    func testValidatePrivateKey_valid64CharHex() {
        // 64 hex characters = 32 bytes (valid private key)
        let validKey = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        XCTAssertTrue(AddressValidator.validatePrivateKey(validKey))
    }

    func testValidatePrivateKey_invalidLength() {
        // Too short
        XCTAssertFalse(AddressValidator.validatePrivateKey("aabbccdd"))
        // Too long
        XCTAssertFalse(AddressValidator.validatePrivateKey("aaff11223344556677889900aabbccddeeff001122334455667788990011223355"))
        // Empty
        XCTAssertFalse(AddressValidator.validatePrivateKey(""))
    }

    func testValidateHexString_customByteLength() {
        // 36 bytes = 72 hex characters (outpoint)
        let valid72 = String(repeating: "ab", count: 36)
        XCTAssertTrue(AddressValidator.validateHexString(valid72, byteLength: 36))
        XCTAssertFalse(AddressValidator.validateHexString(valid72, byteLength: 32))
    }

    func testValidateAmount_validPositive() {
        XCTAssertEqual(AddressValidator.validateAmount("100"), 100)
        XCTAssertEqual(AddressValidator.validateAmount("1"), 1)
        XCTAssertEqual(AddressValidator.validateAmount("18446744073709551615"), UInt64.max) // max UInt64
        XCTAssertEqual(AddressValidator.validateAmount("  500  "), 500) // whitespace trimmed
    }

    func testValidateAmount_invalid() {
        XCTAssertNil(AddressValidator.validateAmount("0")) // zero not allowed
        XCTAssertNil(AddressValidator.validateAmount("-100")) // negative
        XCTAssertNil(AddressValidator.validateAmount("abc"))
        XCTAssertNil(AddressValidator.validateAmount(""))
        XCTAssertNil(AddressValidator.validateAmount("12.5")) // decimal
    }

    func testValidateIdentityId_valid() {
        XCTAssertTrue(AddressValidator.validateIdentityId("someIdentityId123"))
        XCTAssertTrue(AddressValidator.validateIdentityId("aaff11223344556677889900aabbccddeeff0011223344556677889900112233"))
    }

    func testValidateIdentityId_invalid() {
        XCTAssertFalse(AddressValidator.validateIdentityId(""))
        XCTAssertFalse(AddressValidator.validateIdentityId("   "))
    }

    func testValidateBech32mAddress_valid() {
        // A real DIP-0018 testnet platform address (tdash1..., 21-byte
        // payload, valid bech32m checksum) — proof-verified on testnet.
        let validTestnetAddress = "tdash1kzdl4c3apkekqevkqrzctgagv2v2ng5hysegt5x4"
        XCTAssertTrue(AddressValidator.validateBech32mAddress(validTestnetAddress))
    }

    func testValidateBech32mAddress_invalid() {
        // Wrong HRP
        XCTAssertFalse(AddressValidator.validateBech32mAddress("bitcoin1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"))
        // Pre-DIP-0018 HRP ("tdashevo") — rejected since the DIP-0018
        // decode fix; the canonical testnet HRP is "tdash".
        XCTAssertFalse(
            AddressValidator.validateBech32mAddress("tdashevo1qz4242424242424242424242424242424g4dj6u7"))
        // No separator
        XCTAssertFalse(AddressValidator.validateBech32mAddress("tdashevoqqqqqqqqqqq"))
        // Empty
        XCTAssertFalse(AddressValidator.validateBech32mAddress(""))
        // Random string
        XCTAssertFalse(AddressValidator.validateBech32mAddress("notanaddress"))
    }

    func testValidateAddress_autoDetect() {
        // Hex address
        let hexAddr = "00aaff11223344556677889900aabbccddeeff0011"
        XCTAssertTrue(AddressValidator.validateAddress(hexAddr))

        // DIP-0018 bech32m address
        let bech32mAddr = "tdash1kzdl4c3apkekqevkqrzctgagv2v2ng5hysegt5x4"
        XCTAssertTrue(AddressValidator.validateAddress(bech32mAddr))

        // Invalid
        XCTAssertFalse(AddressValidator.validateAddress("invalid"))
    }

    // MARK: - TransferInputValidator Tests

    func testTransferInputValidator_allValid() {
        let result = TransferInputValidator.validate(
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            outputAddressHex: "11bbcc22334455667788990011aabbccddeeff0022",
            inputAmount: "1000",
            outputAmount: "500"
        )
        XCTAssertTrue(result.isValid)
        XCTAssertTrue(result.errors.isEmpty)
    }

    func testTransferInputValidator_invalidInputAddress() {
        let result = TransferInputValidator.validate(
            inputAddressHex: "invalid",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            outputAddressHex: "11bbcc22334455667788990011aabbccddeeff0022",
            inputAmount: "1000",
            outputAmount: "500"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Input address") })
    }

    func testTransferInputValidator_invalidPrivateKey() {
        let result = TransferInputValidator.validate(
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "tooshort",
            outputAddressHex: "11bbcc22334455667788990011aabbccddeeff0022",
            inputAmount: "1000",
            outputAmount: "500"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Private key") })
    }

    func testTransferInputValidator_inputLessThanOutput() {
        let result = TransferInputValidator.validate(
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            outputAddressHex: "11bbcc22334455667788990011aabbccddeeff0022",
            inputAmount: "500",
            outputAmount: "1000"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Input amount must be greater") })
    }

    func testTransferInputValidator_multipleErrors() {
        let result = TransferInputValidator.validate(
            inputAddressHex: "bad",
            inputPrivateKeyHex: "bad",
            outputAddressHex: "bad",
            inputAmount: "abc",
            outputAmount: "xyz"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertEqual(result.errors.count, 5)
    }

    // MARK: - WithdrawInputValidator Tests

    func testWithdrawInputValidator_allValid() {
        let result = WithdrawInputValidator.validate(
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            coreAddress: "yXdMkHQZwWqtQNzCLXWPWrZvJT4RCJGxkp",
            inputAmount: "1000",
            useChangeAddress: false,
            changeAddressHex: ""
        )
        XCTAssertTrue(result.isValid)
        XCTAssertTrue(result.errors.isEmpty)
    }

    func testWithdrawInputValidator_withChangeAddress() {
        let result = WithdrawInputValidator.validate(
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            coreAddress: "yXdMkHQZwWqtQNzCLXWPWrZvJT4RCJGxkp",
            inputAmount: "1000",
            useChangeAddress: true,
            changeAddressHex: "22ccdd33445566778899001122aabbccddeeff0033"
        )
        XCTAssertTrue(result.isValid)
    }

    func testWithdrawInputValidator_invalidChangeAddress() {
        let result = WithdrawInputValidator.validate(
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            coreAddress: "yXdMkHQZwWqtQNzCLXWPWrZvJT4RCJGxkp",
            inputAmount: "1000",
            useChangeAddress: true,
            changeAddressHex: "invalid"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Change address") })
    }

    func testWithdrawInputValidator_emptyCoreAddress() {
        let result = WithdrawInputValidator.validate(
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            coreAddress: "",
            inputAmount: "1000",
            useChangeAddress: false,
            changeAddressHex: ""
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Core address") })
    }

    // MARK: - TopUpAddressFromAssetLockValidator Tests

    func testTopUpValidator_validInstant() {
        let result = TopUpAddressFromAssetLockValidator.validate(
            outputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            assetLockPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            proofType: .instant,
            instantLockHex: "someinstantlockdata",
            transactionHex: "sometxdata",
            outputIndex: "0",
            coreChainLockedHeight: "",
            outPointHex: ""
        )
        XCTAssertTrue(result.isValid)
    }

    func testTopUpValidator_validChain() {
        let outpoint72 = String(repeating: "ab", count: 36) // 72 hex chars
        let result = TopUpAddressFromAssetLockValidator.validate(
            outputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            assetLockPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            proofType: .chain,
            instantLockHex: "",
            transactionHex: "",
            outputIndex: "",
            coreChainLockedHeight: "12345",
            outPointHex: outpoint72
        )
        XCTAssertTrue(result.isValid)
    }

    func testTopUpValidator_invalidOutpoint() {
        let result = TopUpAddressFromAssetLockValidator.validate(
            outputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            assetLockPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            proofType: .chain,
            instantLockHex: "",
            transactionHex: "",
            outputIndex: "",
            coreChainLockedHeight: "12345",
            outPointHex: "tooshort"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Outpoint") })
    }

    // MARK: - IdentityTopUpFromAddressesValidator Tests

    func testIdentityTopUpValidator_valid() {
        let result = IdentityTopUpFromAddressesValidator.validate(
            identityId: "someIdentityId",
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            amount: "1000"
        )
        XCTAssertTrue(result.isValid)
    }

    func testIdentityTopUpValidator_emptyIdentityId() {
        let result = IdentityTopUpFromAddressesValidator.validate(
            identityId: "",
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            amount: "1000"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Identity ID") })
    }

    // MARK: - IdentityTransferToAddressesValidator Tests

    func testIdentityTransferValidator_valid() {
        let result = IdentityTransferToAddressesValidator.validate(
            identityId: "someIdentityId",
            outputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            identityPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            amount: "1000"
        )
        XCTAssertTrue(result.isValid)
    }

    // MARK: - IdentityCreateFromAddressesValidator Tests

    func testIdentityCreateValidator_valid() {
        let result = IdentityCreateFromAddressesValidator.validate(
            identityId: "someIdentityId",
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            identityPrivateKeyHex: "bbff11223344556677889900aabbccddeeff0011223344556677889900112233",
            amount: "1000",
            nonce: "0",
            useChangeAddress: false,
            changeAddressHex: ""
        )
        XCTAssertTrue(result.isValid)
    }

    func testIdentityCreateValidator_invalidNonce() {
        let result = IdentityCreateFromAddressesValidator.validate(
            identityId: "someIdentityId",
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            identityPrivateKeyHex: "bbff11223344556677889900aabbccddeeff0011223344556677889900112233",
            amount: "1000",
            nonce: "notanumber",
            useChangeAddress: false,
            changeAddressHex: ""
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Nonce") })
    }

    func testIdentityCreateValidator_invalidChangeAddress() {
        let result = IdentityCreateFromAddressesValidator.validate(
            identityId: "someIdentityId",
            inputAddressHex: "00aaff11223344556677889900aabbccddeeff0011",
            inputPrivateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233",
            identityPrivateKeyHex: "bbff11223344556677889900aabbccddeeff0011223344556677889900112233",
            amount: "1000",
            nonce: "0",
            useChangeAddress: true,
            changeAddressHex: "invalid"
        )
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains { $0.contains("Change address") })
    }

    // MARK: - ValidationResult Tests

    func testValidationResult_valid() {
        let result = ValidationResult.valid()
        XCTAssertTrue(result.isValid)
        XCTAssertTrue(result.errors.isEmpty)
    }

    func testValidationResult_invalid() {
        let result = ValidationResult.invalid(["Error 1", "Error 2"])
        XCTAssertFalse(result.isValid)
        XCTAssertEqual(result.errors.count, 2)
    }

    func testValidationResult_invalidWithEmptyErrors() {
        // When errors is empty, isValid should be true
        let result = ValidationResult.invalid([])
        XCTAssertTrue(result.isValid)
    }

    // MARK: - Hash Validation Tests

    func testValidateHash_valid64CharHex() {
        let validHash = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        XCTAssertTrue(AddressValidator.validateHash(validHash))
    }

    func testValidateHash_invalidLength() {
        XCTAssertFalse(AddressValidator.validateHash("aabbccdd"))
        XCTAssertFalse(AddressValidator.validateHash(""))
    }

    // MARK: - Identity ID Hex Validation Tests

    func testValidateIdentityIdHex_valid() {
        let validId = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        XCTAssertTrue(AddressValidator.validateIdentityIdHex(validId))
    }

    func testIsHexIdentityId_valid() {
        let validId = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        XCTAssertTrue(AddressValidator.isHexIdentityId(validId))
    }

    func testIsHexIdentityId_invalid() {
        // Too short (base58 style)
        XCTAssertFalse(AddressValidator.isHexIdentityId("4EfA9Wam5vEXTbYbP8YFZP35o9F"))
        // Empty
        XCTAssertFalse(AddressValidator.isHexIdentityId(""))
        // Invalid chars
        XCTAssertFalse(AddressValidator.isHexIdentityId("ggff11223344556677889900aabbccddeeff0011223344556677889900112233"))
    }
}
