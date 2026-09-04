// ErrorHandlingTests.swift
// SwiftExampleAppTests
//
// Unit tests for centralized error handling utilities.

import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

final class ErrorHandlingTests: XCTestCase {

    func testCoreInsufficientFundsFFIResultMapping() {
        XCTAssertEqual(
            PlatformWalletResultCode(
                ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_CORE_INSUFFICIENT_FUNDS
            ),
            .errorCoreInsufficientFunds
        )
    }

    func testAssetLockRecoveryFFIResultMappings() {
        XCTAssertEqual(
            PlatformWalletResultCode(ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_NOT_TRACKED),
            .errorAssetLockNotTracked
        )
        XCTAssertEqual(
            PlatformWalletResultCode(ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_ALREADY_CONSUMED),
            .errorAssetLockAlreadyConsumed
        )
        XCTAssertEqual(
            PlatformWalletResultCode(ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_FUNDING_MISMATCH),
            .errorAssetLockFundingMismatch
        )
    }

    func testAssetLockInsufficientFundsFFIResultMapping() {
        // The asset-lock coin-selection shortfall (dashpay/platform#4073).
        // Swift could not decode code 29 at all before this mirror existed —
        // no raw-value case and no C-enum arm meant it fell through to
        // .errorUnknown, so the Rust-side typed code died at the Swift
        // boundary while Kotlin already branched on it.
        XCTAssertEqual(
            PlatformWalletResultCode(
                ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_INSUFFICIENT_FUNDS
            ),
            .errorAssetLockInsufficientFunds
        )
        XCTAssertNotEqual(
            PlatformWalletResultCode(
                ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_INSUFFICIENT_FUNDS
            ),
            .errorUnknown
        )
        // The raw value is hand-mirrored ABI, not a derived ordinal.
        XCTAssertEqual(
            PlatformWalletResultCode.errorAssetLockInsufficientFunds.rawValue,
            29
        )

        // The structured available/required duffs ride the message string —
        // PlatformWalletFFIResult is ABI-frozen at code + message — so the
        // typed error must carry them through unaltered.
        let rendered = "asset lock coin selection is short: available 18000000 duffs, "
            + "required 100000000 duffs"
        let error = PlatformWalletError(
            code: .errorAssetLockInsufficientFunds,
            message: rendered
        )
        guard case .assetLockInsufficientFunds(let message) = error else {
            return XCTFail("expected typed assetLockInsufficientFunds error")
        }
        XCTAssertEqual(message, rendered)
        XCTAssertEqual(error.errorDescription, rendered)
    }

    // TODO: not compiled or run locally — no Swift toolchain in the
    // authoring environment. CI is the first execution of the three
    // persister tests below and of the `PlatformWalletResult.swift` cases
    // they cover.
    /// The persister block (49-54). Each code must decode from its
    /// generated C constant, keep its own raw value, and reach a typed
    /// `PlatformWalletError` case — the three edits a new code needs on
    /// this side. Without the `init(ffi:)` arm a code compiles fine and
    /// silently degrades to `.errorUnknown`, losing the classification the
    /// Rust side went to the trouble of carrying across.
    func testPersisterFFIResultMappings() {
        let mappings: [(PlatformWalletFFIResultCode, PlatformWalletResultCode, Int32)] = [
            (
                PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_LOAD_TRANSIENT,
                .errorPersisterLoadTransient, 49
            ),
            (
                PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_LOAD_FATAL,
                .errorPersisterLoadFatal, 50
            ),
            (
                PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_STORE_TRANSIENT,
                .errorPersisterStoreTransient, 51
            ),
            (
                PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_STORE_FATAL,
                .errorPersisterStoreFatal, 52
            ),
            (
                PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_STORE_CONSTRAINT,
                .errorPersisterStoreConstraint, 53
            ),
            (
                PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_PERSISTER_RESTORE,
                .errorPersisterRestore, 54
            ),
        ]

        for (ffi, expected, rawValue) in mappings {
            XCTAssertEqual(PlatformWalletResultCode(ffi: ffi), expected)
            XCTAssertNotEqual(PlatformWalletResultCode(ffi: ffi), .errorUnknown)
            // Hand-mirrored ABI, not a derived ordinal.
            XCTAssertEqual(expected.rawValue, rawValue)
        }
    }

    /// The two retryable persister codes must arrive as their own typed
    /// cases carrying the Rust message, and must not be confused with the
    /// non-retryable siblings that share an operation.
    func testPersisterTypedErrorCases() {
        let busy = "failed to persist wallet registration changeset: "
            + "persistence backend error (Transient): database is locked"
        let storeTransient = PlatformWalletError(
            code: .errorPersisterStoreTransient,
            message: busy
        )
        guard case .persisterStoreTransient(let storeMessage) = storeTransient else {
            return XCTFail("expected typed persisterStoreTransient error")
        }
        XCTAssertEqual(storeMessage, busy)
        // The chain stays reachable for logs — on the associated value and
        // on failureReason — but must never be the alert text.
        XCTAssertEqual(storeTransient.failureReason, busy)

        guard case .persisterStoreConstraint = PlatformWalletError(
            code: .errorPersisterStoreConstraint,
            message: "constraint failed"
        ) else {
            return XCTFail("a constraint violation must not read as a transient or fatal store")
        }

        guard case .persisterLoadTransient = PlatformWalletError(
            code: .errorPersisterLoadTransient,
            message: busy
        ) else {
            return XCTFail("expected typed persisterLoadTransient error")
        }

        guard case .persisterRestore(let restoreMessage) = PlatformWalletError(
            code: .errorPersisterRestore,
            message: "failed to restore persisted platform-address state: wallet is locked"
        ) else {
            return XCTFail("expected typed persisterRestore error")
        }
        XCTAssertEqual(
            restoreMessage,
            "failed to restore persisted platform-address state: wallet is locked"
        )
    }

    /// `errorDescription` is what a default SwiftUI alert renders, so the
    /// persister cases must answer it with an instruction rather than the
    /// Rust error chain — and must not describe a failed write as a failed
    /// read. The chain belongs on `failureReason`.
    func testPersisterErrorsSplitUserTextFromDiagnostics() {
        let busy = "failed to persist wallet registration changeset: "
            + "persistence backend error (Transient): database is locked"
        let expected: [(PlatformWalletResultCode, String)] = [
            (.errorPersisterLoadTransient, "The wallet database is busy. Try again in a moment."),
            (.errorPersisterStoreTransient, "The wallet database is busy. Try again in a moment."),
            (
                .errorPersisterLoadFatal,
                "The wallet data could not be read and may need to be restored."
            ),
            (
                .errorPersisterRestore,
                "The wallet data could not be read and may need to be restored."
            ),
            (
                .errorPersisterStoreFatal,
                "The wallet data could not be saved and may need to be restored."
            ),
            (
                .errorPersisterStoreConstraint,
                "The wallet data could not be saved and may need to be restored."
            ),
        ]

        for (code, userText) in expected {
            let error = PlatformWalletError(code: code, message: busy)
            XCTAssertEqual(error.errorDescription, userText, "code \(code)")
            XCTAssertEqual(error.failureReason, busy, "code \(code) must keep the chain for logs")
        }
    }

    func testPlatformWalletNotFoundFFIResultMapping() {
        // Code 98 (the blanket Option→result miss) stays typed inside the
        // wallet-error family — the mapping Kotlin now converges on
        // (DashSdkError.PlatformWallet.NotFound), dashpay/platform#4060.
        XCTAssertEqual(
            PlatformWalletResultCode(ffi: PLATFORM_WALLET_FFI_RESULT_CODE_NOT_FOUND),
            .notFound
        )
    }

    func testSigningKeyUnavailableFFIResultMapping() {
        // The structured signer discriminator (dashpay/platform#4060
        // finding 7): PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
        // (31) surfaces as the typed .errorSigningKeyUnavailable /
        // PlatformWalletError.signingKeyUnavailable — no message sniffing.
        XCTAssertEqual(
            PlatformWalletResultCode(
                ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SIGNING_KEY_UNAVAILABLE
            ),
            .errorSigningKeyUnavailable
        )
    }

    func testShieldedInsufficientBalanceFFIResultMapping() {
        XCTAssertEqual(
            PlatformWalletResultCode(
                ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_INSUFFICIENT_BALANCE
            ),
            .errorShieldedInsufficientBalance
        )
        XCTAssertEqual(
            PlatformWalletResultCode.errorShieldedInsufficientBalance.rawValue,
            41
        )

        let error = PlatformWalletError(
            code: .errorShieldedInsufficientBalance,
            message: "available 3623849220, required 3623849221"
        )
        guard case .shieldedInsufficientBalance(let message) = error else {
            return XCTFail("expected typed shieldedInsufficientBalance error")
        }
        XCTAssertEqual(message, "available 3623849220, required 3623849221")
    }

    @MainActor
    func testShieldedShieldPreflightRejectsUnconfiguredManager() async {
        let manager = PlatformWalletManager()
        do {
            _ = try await manager.shieldedShieldPreflight(
                walletId: Data(repeating: 0, count: 32)
            )
            XCTFail("Expected invalidHandle")
        } catch PlatformWalletError.invalidHandle {
            // Expected.
        } catch {
            XCTFail("Expected invalidHandle, got \(error)")
        }
    }

    func testKeychainSignerMissingKeyErrorsClassifyAsSigningKeyUnavailable() {
        // The trampoline's structured completion code: "no stored key"
        // outcomes carry SigningKeyUnavailable (1); operational failures
        // stay Generic (0).
        XCTAssertEqual(
            keychainSignerCompletionErrorCode(for: .publicKeyNotFound),
            KeychainSignerCompletionErrorCode.signingKeyUnavailable
        )
        XCTAssertEqual(
            keychainSignerCompletionErrorCode(
                for: .privateKeyMissingFromKeychain(account: "acct")
            ),
            KeychainSignerCompletionErrorCode.signingKeyUnavailable
        )
        XCTAssertEqual(
            keychainSignerCompletionErrorCode(for: .ffiSignFailed(message: "boom")),
            KeychainSignerCompletionErrorCode.generic
        )
    }

    // MARK: - ErrorCategory Tests

    func testErrorCategoryIsUserRecoverable() {
        // Recoverable
        XCTAssertTrue(ErrorCategory.validation.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.userInput.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.authentication.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.timeout.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.network.isUserRecoverable)

        // Not recoverable
        XCTAssertFalse(ErrorCategory.authorization.isUserRecoverable)
        XCTAssertFalse(ErrorCategory.serialization.isUserRecoverable)
        XCTAssertFalse(ErrorCategory.cryptography.isUserRecoverable)
        XCTAssertFalse(ErrorCategory.system.isUserRecoverable)
    }

    func testErrorCategoryShouldLog() {
        // Should not log user input errors
        XCTAssertFalse(ErrorCategory.validation.shouldLog)
        XCTAssertFalse(ErrorCategory.userInput.shouldLog)

        // Should log system errors
        XCTAssertTrue(ErrorCategory.network.shouldLog)
        XCTAssertTrue(ErrorCategory.system.shouldLog)
        XCTAssertTrue(ErrorCategory.cryptography.shouldLog)
    }

    // MARK: - UserFacingError Tests

    func testUserFacingErrorCreation() {
        let error = UserFacingError(
            title: "Test Error",
            message: "Something went wrong",
            category: .network,
            recoverySuggestion: "Try again",
            underlyingError: "Original error",
            isRetryable: true
        )

        XCTAssertEqual(error.title, "Test Error")
        XCTAssertEqual(error.message, "Something went wrong")
        XCTAssertEqual(error.category, .network)
        XCTAssertEqual(error.recoverySuggestion, "Try again")
        XCTAssertEqual(error.underlyingError, "Original error")
        XCTAssertTrue(error.isRetryable)
    }

    func testUserFacingErrorLocalizedError() {
        let error = UserFacingError(
            title: "Title",
            message: "Message",
            category: .validation,
            recoverySuggestion: "Suggestion"
        )

        XCTAssertEqual(error.errorDescription, "Message")
        XCTAssertEqual(error.failureReason, "Title")
        XCTAssertEqual(error.localizedRecoverySuggestion, "Suggestion")
    }

    func testUserFacingErrorDisplayText() {
        let error = UserFacingError(
            title: "Network Error",
            message: "Connection failed"
        )

        XCTAssertEqual(error.displayText, "Network Error: Connection failed")
    }

    func testUserFacingErrorFullDescription() {
        let error = UserFacingError(
            title: "Error",
            message: "Failed",
            recoverySuggestion: "Try again"
        )

        XCTAssertEqual(error.fullDescription, "Error: Failed\nTry again")
    }

    func testUserFacingErrorFullDescriptionWithoutSuggestion() {
        let error = UserFacingError(
            title: "Error",
            message: "Failed"
        )

        XCTAssertEqual(error.fullDescription, "Error: Failed")
    }

    func testUserFacingErrorEquatable() {
        let error1 = UserFacingError(title: "A", message: "B", category: .network)
        let error2 = UserFacingError(title: "A", message: "B", category: .network)
        let error3 = UserFacingError(title: "C", message: "D", category: .network)

        XCTAssertEqual(error1, error2)
        XCTAssertNotEqual(error1, error3)
    }

    // MARK: - ErrorFormatter Tests

    func testErrorFormatterFormatForDisplay() {
        let userFacing = UserFacingError(title: "Title", message: "Message")
        XCTAssertEqual(ErrorFormatter.formatForDisplay(userFacing), "Title: Message")
    }

    func testErrorFormatterFormatWithCategory() {
        struct SimpleError: Error {}
        let message = ErrorFormatter.formatWithCategory(SimpleError(), category: .network)
        XCTAssertTrue(message.hasPrefix("[Network]"))
    }

    func testErrorFormatterFormatValidationErrorsSingle() {
        let errors = ["Invalid email address"]
        let formatted = ErrorFormatter.formatValidationErrors(errors)
        XCTAssertEqual(formatted, "Invalid email address")
    }

    func testErrorFormatterFormatValidationErrorsMultiple() {
        let errors = ["Error 1", "Error 2", "Error 3"]
        let formatted = ErrorFormatter.formatValidationErrors(errors)
        XCTAssertEqual(formatted, "1. Error 1\n2. Error 2\n3. Error 3")
    }

    func testErrorFormatterFormatValidationErrorsEmpty() {
        let formatted = ErrorFormatter.formatValidationErrors([])
        XCTAssertEqual(formatted, "")
    }

    func testErrorFormatterFormatForLogging() {
        struct TestError: Error {}
        let formatted = ErrorFormatter.formatForLogging(TestError(), context: "TestContext")
        XCTAssertTrue(formatted.contains("[TestContext]"))
        XCTAssertTrue(formatted.contains("TestError"))
    }

    // MARK: - ErrorRecovery Tests

    func testErrorRecoverySuggestionForCategory() {
        let networkSuggestion = ErrorRecovery.suggestion(for: .network)
        XCTAssertTrue(networkSuggestion.contains("internet") || networkSuggestion.contains("connection"))

        let validationSuggestion = ErrorRecovery.suggestion(for: .validation)
        XCTAssertTrue(validationSuggestion.contains("check") || validationSuggestion.contains("input"))

        let timeoutSuggestion = ErrorRecovery.suggestion(for: .timeout)
        XCTAssertTrue(timeoutSuggestion.contains("long") || timeoutSuggestion.contains("again"))
    }

    func testErrorRecoverySuggestionFromMessage() {
        XCTAssertNotNil(ErrorRecovery.suggestionFromMessage("Network connection failed"))
        XCTAssertNotNil(ErrorRecovery.suggestionFromMessage("Request timed out"))
        XCTAssertNotNil(ErrorRecovery.suggestionFromMessage("Invalid input"))
        XCTAssertNotNil(ErrorRecovery.suggestionFromMessage("Resource not found"))
        XCTAssertNotNil(ErrorRecovery.suggestionFromMessage("Unauthorized access"))
        XCTAssertNil(ErrorRecovery.suggestionFromMessage("Some random error"))
    }

    func testErrorRecoveryIsRetryable() {
        XCTAssertTrue(ErrorRecovery.isRetryable(category: .network))
        XCTAssertTrue(ErrorRecovery.isRetryable(category: .timeout))
        XCTAssertTrue(ErrorRecovery.isRetryable(category: .system))

        XCTAssertFalse(ErrorRecovery.isRetryable(category: .validation))
        XCTAssertFalse(ErrorRecovery.isRetryable(category: .userInput))
        XCTAssertFalse(ErrorRecovery.isRetryable(category: .authentication))
    }

    // MARK: - ErrorCategorizer Tests

    func testErrorCategorizerCategorize() {
        struct ValidationError: Error {}
        struct NetworkError: Error {}

        // UserFacingError returns its own category
        let userFacing = UserFacingError(title: "T", message: "M", category: .storage)
        XCTAssertEqual(ErrorCategorizer.categorize(userFacing), .storage)
    }

    func testErrorCategorizerToUserFacingError() {
        struct TestError: Error, LocalizedError {
            var errorDescription: String? { "Test error message" }
        }

        let userFacing = ErrorCategorizer.toUserFacingError(TestError())

        XCTAssertEqual(userFacing.message, "Test error message")
        XCTAssertNotNil(userFacing.recoverySuggestion)
    }

    func testErrorCategorizerPreservesUserFacingError() {
        let original = UserFacingError(
            title: "Original",
            message: "Keep this",
            category: .cryptography
        )

        let result = ErrorCategorizer.toUserFacingError(original)
        XCTAssertEqual(result.title, "Original")
        XCTAssertEqual(result.message, "Keep this")
        XCTAssertEqual(result.category, .cryptography)
    }

    // MARK: - ErrorBuilder Tests

    func testErrorBuilderBasic() {
        let error = ErrorBuilder()
            .withTitle("Custom Title")
            .withMessage("Custom message")
            .withCategory(.authentication)
            .build()

        XCTAssertEqual(error.title, "Custom Title")
        XCTAssertEqual(error.message, "Custom message")
        XCTAssertEqual(error.category, .authentication)
    }

    func testErrorBuilderWithAllOptions() {
        struct UnderlyingError: Error {}

        let error = ErrorBuilder()
            .withTitle("Title")
            .withMessage("Message")
            .withCategory(.network)
            .withRecoverySuggestion("Custom suggestion")
            .withUnderlyingError(UnderlyingError())
            .retryable()
            .build()

        XCTAssertEqual(error.title, "Title")
        XCTAssertEqual(error.message, "Message")
        XCTAssertEqual(error.category, .network)
        XCTAssertEqual(error.recoverySuggestion, "Custom suggestion")
        XCTAssertNotNil(error.underlyingError)
        XCTAssertTrue(error.isRetryable)
    }

    func testErrorBuilderValidationFactory() {
        let error = ErrorBuilder.validation("Invalid email")

        XCTAssertEqual(error.title, "Validation Error")
        XCTAssertEqual(error.message, "Invalid email")
        XCTAssertEqual(error.category, .validation)
    }

    func testErrorBuilderNetworkFactory() {
        let error = ErrorBuilder.network("Connection failed")

        XCTAssertEqual(error.title, "Network Error")
        XCTAssertEqual(error.message, "Connection failed")
        XCTAssertEqual(error.category, .network)
        XCTAssertTrue(error.isRetryable)
    }

    func testErrorBuilderNotFoundFactory() {
        let error = ErrorBuilder.notFound("User not found")

        XCTAssertEqual(error.title, "Not Found")
        XCTAssertEqual(error.message, "User not found")
        XCTAssertEqual(error.category, .notFound)
    }

    func testErrorBuilderTimeoutFactory() {
        let error = ErrorBuilder.timeout()

        XCTAssertEqual(error.title, "Timeout")
        XCTAssertEqual(error.category, .timeout)
        XCTAssertTrue(error.isRetryable)
    }

    func testErrorBuilderAuthenticationFactory() {
        let error = ErrorBuilder.authentication("Invalid credentials")

        XCTAssertEqual(error.title, "Authentication Error")
        XCTAssertEqual(error.message, "Invalid credentials")
        XCTAssertEqual(error.category, .authentication)
    }

    func testErrorBuilderUserInputFactory() {
        let error = ErrorBuilder.userInput("Field required")

        XCTAssertEqual(error.title, "Input Error")
        XCTAssertEqual(error.message, "Field required")
        XCTAssertEqual(error.category, .userInput)
    }

    // MARK: - ErrorAggregator Tests

    func testErrorAggregatorAddErrors() {
        var aggregator = ErrorAggregator()

        XCTAssertFalse(aggregator.hasErrors)
        XCTAssertEqual(aggregator.count, 0)

        aggregator.add("Error 1", category: .validation)
        aggregator.add("Error 2", category: .network)

        XCTAssertTrue(aggregator.hasErrors)
        XCTAssertEqual(aggregator.count, 2)
    }

    func testErrorAggregatorAddValidation() {
        var aggregator = ErrorAggregator()
        aggregator.addValidation("Invalid input")

        XCTAssertEqual(aggregator.allErrors.first?.category, .validation)
    }

    func testErrorAggregatorMessages() {
        var aggregator = ErrorAggregator()
        aggregator.add("First error")
        aggregator.add("Second error")

        XCTAssertEqual(aggregator.messages, ["First error", "Second error"])
    }

    func testErrorAggregatorCombinedMessage() {
        var aggregator = ErrorAggregator()
        aggregator.add("Error A")
        aggregator.add("Error B")

        let combined = aggregator.combinedMessage
        XCTAssertTrue(combined.contains("Error A"))
        XCTAssertTrue(combined.contains("Error B"))
    }

    func testErrorAggregatorPrimaryError() {
        var aggregator = ErrorAggregator()
        aggregator.add("Validation error", category: .validation)
        aggregator.add("Network error", category: .network)
        aggregator.add("System error", category: .system)

        // System should be highest priority
        let primary = aggregator.primaryError
        XCTAssertEqual(primary?.category, .system)
    }

    func testErrorAggregatorClear() {
        var aggregator = ErrorAggregator()
        aggregator.add("Error")
        XCTAssertTrue(aggregator.hasErrors)

        aggregator.clear()
        XCTAssertFalse(aggregator.hasErrors)
        XCTAssertEqual(aggregator.count, 0)
    }

    // MARK: - Result Extensions Tests

    func testResultMapToUserFacingError() {
        struct TestError: Error {}

        let failureResult: Result<String, Error> = .failure(TestError())
        let mapped = failureResult.mapToUserFacingError()

        if case .failure(let error) = mapped {
            XCTAssertNotNil(error.recoverySuggestion)
        } else {
            XCTFail("Expected failure")
        }
    }

    func testResultErrorMessage() {
        struct TestError: Error, LocalizedError {
            var errorDescription: String? { "Test failure" }
        }

        let successResult: Result<String, Error> = .success("OK")
        XCTAssertNil(successResult.errorMessage)

        let failureResult: Result<String, Error> = .failure(TestError())
        XCTAssertEqual(failureResult.errorMessage, "Test failure")
    }

    // MARK: - Core broadcast outcome mapping

    func testCoreBroadcastOutcomeMapping() throws {
        XCTAssertEqual(
            try CoreTransactionBroadcastOutcome(
                resultCode: .success,
                txid: "accepted-id",
                reason: ""
            ),
            .accepted(txid: "accepted-id")
        )
        XCTAssertEqual(
            try CoreTransactionBroadcastOutcome(
                resultCode: .errorTransactionBroadcastRejected,
                txid: "rejected-id",
                reason: "policy"
            ),
            .rejected(txid: "rejected-id", reason: "policy")
        )
        XCTAssertEqual(
            try CoreTransactionBroadcastOutcome(
                resultCode: .errorTransactionBroadcastUnconfirmed,
                txid: "unknown-id",
                reason: "timeout"
            ),
            .unknown(txid: "unknown-id", reason: "timeout")
        )
    }

    func testCoreBroadcastOutcomeRejectsOperationalResultCode() {
        XCTAssertThrowsError(
            try CoreTransactionBroadcastOutcome(
                resultCode: .errorInvalidHandle,
                txid: "unused",
                reason: "invalid handle"
            )
        )
    }
}
