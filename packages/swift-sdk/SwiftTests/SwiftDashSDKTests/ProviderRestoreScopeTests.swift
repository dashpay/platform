import XCTest
@testable import SwiftDashSDK

final class ProviderRestoreScopeTests: XCTestCase {
    func testStandardAccountDoesNotAuthorizeProviderRestore() {
        let wallet = Data(repeating: 1, count: 32)
        XCTAssertFalse(
            PlatformWalletPersistenceHandler.shouldRestoreProviderSpecialTransaction(
                walletId: wallet,
                involvedAccounts: [(walletId: wallet, accountType: 0)]
            )
        )
    }

    func testUnrelatedProviderAccountDoesNotAuthorizeProviderRestore() {
        let wallet = Data(repeating: 1, count: 32)
        let other = Data(repeating: 2, count: 32)
        XCTAssertFalse(
            PlatformWalletPersistenceHandler.shouldRestoreProviderSpecialTransaction(
                walletId: wallet,
                involvedAccounts: [(walletId: other, accountType: 9)]
            )
        )
    }

    func testMatchingProviderAccountAuthorizesProviderRestore() {
        let wallet = Data(repeating: 1, count: 32)
        for type in UInt32(8)...UInt32(11) {
            XCTAssertTrue(
                PlatformWalletPersistenceHandler.shouldRestoreProviderSpecialTransaction(
                    walletId: wallet,
                    involvedAccounts: [(walletId: wallet, accountType: type)]
                )
            )
        }
    }
}
