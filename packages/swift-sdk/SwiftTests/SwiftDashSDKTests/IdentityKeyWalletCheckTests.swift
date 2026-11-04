import XCTest

@testable import SwiftDashSDK

/// `ManagedPlatformWallet.checkIdentityKey` / `checkIdentityKeys`, the
/// bridges to the wallet-scoped Rust checks. Rust decides every outcome,
/// including refusing another wallet's recovery phrase; those outcomes
/// are pinned by the `identity_key_wallet_check` FFI tests against a
/// real wallet handle. A unit test here has no registered wallet handle
/// (that needs a configured `PlatformWalletManager`), so these pin what
/// the bridge does without one: it throws, and never reports a verdict.
@MainActor
final class IdentityKeyWalletCheckTests: XCTestCase {

    private let walletId = Data(repeating: 0x07, count: 32)

    private func foreignMasterInput() -> ManagedPlatformWallet.IdentityKeyCheckInput {
        ManagedPlatformWallet.IdentityKeyCheckInput(
            keyId: 0, keyType: .ecdsaSecp256k1,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.master.rawValue,
            publicKeyData: Data(repeating: 0x02, count: 33), disabledAt: nil)
    }

    /// Without a wallet Rust holds there is no seed binding to check, so
    /// the identity check throws instead of calling the identity foreign.
    func testShouldThrowForAWalletRustDoesNotHold() throws {
        let wallet = ManagedPlatformWallet(handle: NULL_HANDLE, walletId: walletId)

        XCTAssertThrowsError(
            try wallet.checkIdentityKeys(
                identityIndex: 0, keys: [foreignMasterInput()], unreadableKeyCount: 0))
        XCTAssertThrowsError(
            try wallet.checkIdentityKey(
                identityIndex: 0, keyId: 0, keyType: .ecdsaSecp256k1,
                purpose: KeyPurpose.authentication.rawValue,
                securityLevel: SecurityLevel.master.rawValue,
                publicKeyData: Data(repeating: 0x02, count: 33)))
    }

    func testShouldRefuseANegativeUnreadableKeyCount() throws {
        let wallet = ManagedPlatformWallet(handle: NULL_HANDLE, walletId: walletId)

        XCTAssertThrowsError(
            try wallet.checkIdentityKeys(
                identityIndex: 0, keys: [foreignMasterInput()], unreadableKeyCount: -1)
        ) { error in
            guard case PlatformWalletError.invalidParameter = error else {
                return XCTFail("expected invalidParameter, got \(error)")
            }
        }
    }
}
