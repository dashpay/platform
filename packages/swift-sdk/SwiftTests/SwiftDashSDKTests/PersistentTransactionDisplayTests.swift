import XCTest
@testable import SwiftDashSDK

/// Pins the `transactionTypeKind` discriminant → display mapping on
/// `PersistentTransaction`. The raw bytes mirror Rust's
/// `transaction_type_to_u8` (rs-platform-wallet-ffi
/// `core_wallet_types.rs`); a drift there, a `TransactionTypeKind`
/// case renumbering, or a label typo shows up here without UI
/// automation.
final class PersistentTransactionDisplayTests: XCTestCase {

    /// Direction 2 = internal — the raw classification every special
    /// tx gets (asset locks, provider txs), which the display helpers
    /// exist to override.
    private func makeTransaction(kind: UInt8, direction: UInt32 = 2) -> PersistentTransaction {
        let tx = PersistentTransaction(
            txid: Data(repeating: 0xAB, count: 32),
            transactionData: Data(),
            direction: direction
        )
        tx.transactionTypeKind = kind
        return tx
    }

    func testProviderSpecialNameForProviderKinds() {
        let expected: [UInt8: String] = [
            2: "Provider Registration",
            3: "Provider Update Registrar",
            4: "Provider Update Service",
            5: "Provider Update Revocation",
        ]
        for (kind, name) in expected {
            let tx = makeTransaction(kind: kind)
            XCTAssertEqual(tx.providerSpecialName, name, "kind \(kind)")
            XCTAssertTrue(tx.isProviderSpecial, "kind \(kind)")
            XCTAssertEqual(tx.displayDirection, name, "kind \(kind)")
        }
    }

    func testNonProviderKindsAreNotProviderSpecial() {
        // Every other discriminant, plus the 0xFF "not populated"
        // sentinel and an out-of-range future byte.
        for kind: UInt8 in [0, 1, 6, 7, 8, 9, 0xFF, 42] {
            let tx = makeTransaction(kind: kind)
            XCTAssertNil(tx.providerSpecialName, "kind \(kind)")
            XCTAssertFalse(tx.isProviderSpecial, "kind \(kind)")
        }
    }

    func testDisplayDirectionPrecedenceUnchangedForOtherKinds() {
        // Asset lock/unlock keep their existing overrides…
        XCTAssertEqual(makeTransaction(kind: 6).displayDirection, "Asset Lock")
        XCTAssertEqual(makeTransaction(kind: 7).displayDirection, "Asset Unlock")
        // …and non-special kinds fall through to the raw direction.
        XCTAssertEqual(makeTransaction(kind: 0).displayDirection, "Internal")
        XCTAssertEqual(makeTransaction(kind: 0, direction: 0).displayDirection, "Incoming")
        XCTAssertEqual(makeTransaction(kind: 0xFF).displayDirection, "Internal")
    }
}
