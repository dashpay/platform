import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Coverage for the `PersistentShieldedNote` unspent-note predicate
/// helpers — the single home every shielded-balance surface uses to
/// agree on what "unspent" means (per-wallet `@Query` filters in the
/// wallet detail / send views; the all-wallets variant filtered in
/// memory by the identity funding picker).
@MainActor
final class PersistentShieldedNotePredicateTests: XCTestCase {

    private func walletId(_ byte: UInt8) -> Data {
        Data(repeating: byte, count: 32)
    }

    /// Insert three notes across two wallets — one of them spent — and
    /// assert both predicate variants filter as documented: the
    /// per-wallet predicate returns only that wallet's UNSPENT rows,
    /// and the all-wallets predicate returns every wallet's unspent
    /// rows (leaving the spent one out in both cases).
    func testUnspentPredicatesFilterByWalletAndSpentFlag() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        let walletA = walletId(0xAA)
        let walletB = walletId(0xBB)

        // Wallet A: one unspent (value 100) + one spent (value 999,
        // must be excluded everywhere).
        context.insert(makeNote(walletId: walletA, nullifier: 0x01, isSpent: false, value: 100))
        context.insert(makeNote(walletId: walletA, nullifier: 0x02, isSpent: true, value: 999))
        // Wallet B: one unspent (value 50).
        context.insert(makeNote(walletId: walletB, nullifier: 0x03, isSpent: false, value: 50))
        try context.save()

        // Per-wallet predicate: only wallet A's UNSPENT row.
        let walletANotes = try context.fetch(
            FetchDescriptor<PersistentShieldedNote>(
                predicate: PersistentShieldedNote.unspentPredicate(walletId: walletA)
            )
        )
        XCTAssertEqual(walletANotes.count, 1)
        XCTAssertEqual(walletANotes.first?.value, 100)

        // All-wallets predicate: both unspent rows, spent one dropped.
        let allUnspent = try context.fetch(
            FetchDescriptor<PersistentShieldedNote>(
                predicate: PersistentShieldedNote.unspentPredicate
            )
        )
        XCTAssertEqual(allUnspent.count, 2)
        // In-memory per-wallet scoping (the identity funding picker's
        // pattern) sums the right subset.
        XCTAssertEqual(
            allUnspent.filter { $0.walletId == walletA }.reduce(0) { $0 + $1.value },
            100
        )
        XCTAssertEqual(
            allUnspent.filter { $0.walletId == walletB }.reduce(0) { $0 + $1.value },
            50
        )
    }

    private func makeNote(
        walletId: Data,
        nullifier: UInt8,
        isSpent: Bool,
        value: UInt64
    ) -> PersistentShieldedNote {
        PersistentShieldedNote(
            walletId: walletId,
            accountIndex: 0,
            position: 0,
            cmx: Data(repeating: 0, count: 32),
            nullifier: Data(repeating: nullifier, count: 32),
            blockHeight: 1,
            isSpent: isSpent,
            value: value,
            noteData: Data(repeating: 0, count: 115)
        )
    }
}
