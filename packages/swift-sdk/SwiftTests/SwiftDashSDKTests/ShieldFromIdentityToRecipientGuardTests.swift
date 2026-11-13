import XCTest
@testable import SwiftDashSDK

/// Input guards of `shieldedShieldFromIdentityToRecipient`: every guard throws
/// before any FFI call, so a fake handle is never dereferenced.
@MainActor
final class ShieldFromIdentityToRecipientGuardTests: XCTestCase {
    private static let noopCalls: PlatformWalletNativeTeardownCalls = {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok)
    }()

    private func makeSigner() throws -> KeychainSigner {
        KeychainSigner(modelContainer: try DashModelContainer.createInMemory())
    }

    private func call(
        _ manager: PlatformWalletManager,
        walletId: Data = Data(repeating: 1, count: 32),
        identityId: Data = Data(repeating: 2, count: 32),
        recipientRaw43: Data = Data(repeating: 3, count: 43)
    ) async throws {
        try await manager.shieldedShieldFromIdentityToRecipient(
            walletId: walletId, shieldedAccount: 0, identityId: identityId,
            recipientRaw43: recipientRaw43, amount: 1, identitySigner: makeSigner())
    }

    private func assertInvalidParameter(
        _ operation: () async throws -> Void, _ what: String,
        file: StaticString = #filePath, line: UInt = #line
    ) async {
        do {
            try await operation()
            XCTFail("\(what): expected invalidParameter, call succeeded", file: file, line: line)
        } catch PlatformWalletError.invalidParameter {
            // expected
        } catch {
            XCTFail("\(what): expected invalidParameter, got \(error)", file: file, line: line)
        }
    }

    func testWalletIdNot32BytesThrowsInvalidParameter() async {
        let manager = PlatformWalletManager.makeForTesting(handle: 91, calls: Self.noopCalls)
        await assertInvalidParameter({ try await self.call(manager, walletId: Data(repeating: 1, count: 31)) }, "walletId 31")
        await assertInvalidParameter({ try await self.call(manager, walletId: Data(repeating: 1, count: 33)) }, "walletId 33")
    }

    func testIdentityIdNot32BytesThrowsInvalidParameter() async {
        let manager = PlatformWalletManager.makeForTesting(handle: 92, calls: Self.noopCalls)
        await assertInvalidParameter({ try await self.call(manager, identityId: Data(repeating: 2, count: 31)) }, "identityId 31")
        await assertInvalidParameter({ try await self.call(manager, identityId: Data(repeating: 2, count: 33)) }, "identityId 33")
    }

    func testRecipientNot43BytesThrowsInvalidParameter() async {
        let manager = PlatformWalletManager.makeForTesting(handle: 93, calls: Self.noopCalls)
        await assertInvalidParameter({ try await self.call(manager, recipientRaw43: Data(repeating: 3, count: 42)) }, "recipient 42")
        await assertInvalidParameter({ try await self.call(manager, recipientRaw43: Data(repeating: 3, count: 44)) }, "recipient 44")
        await assertInvalidParameter({ try await self.call(manager, recipientRaw43: Data()) }, "recipient empty")
    }

    func testUnconfiguredManagerThrowsInvalidHandle() async {
        let manager = PlatformWalletManager()
        do {
            try await call(manager)
            XCTFail("expected invalidHandle, call succeeded")
        } catch PlatformWalletError.invalidHandle {
            // expected
        } catch {
            XCTFail("expected invalidHandle, got \(error)")
        }
    }
}
