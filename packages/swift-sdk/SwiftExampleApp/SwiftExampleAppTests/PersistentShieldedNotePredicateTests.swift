import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Coverage for the `PersistentShieldedNote` predicates that load note
/// rows: the per-wallet and all-wallets unspent filters and the account-0
/// filters of the send and identity funding screens. The rows mark a
/// wallet as not empty and drive local balance refreshes; no balance is
/// computed from them (balances come from Rust's local balance snapshot).
@MainActor
final class PersistentShieldedNotePredicateTests: XCTestCase {

    private func walletId(_ byte: UInt8) -> Data {
        Data(repeating: byte, count: 32)
    }

    private func nullifiers(_ notes: [PersistentShieldedNote]) -> Set<Data> {
        Set(notes.map(\.nullifier))
    }

    private func nullifier(_ byte: UInt8) -> Data {
        Data(repeating: byte, count: 32)
    }

    /// Insert three notes across two wallets, one of them spent, and
    /// assert both predicate variants filter as documented: the
    /// per-wallet predicate returns only that wallet's UNSPENT rows,
    /// and the all-wallets predicate returns every wallet's unspent
    /// rows (leaving the spent one out in both cases).
    func testUnspentPredicatesFilterByWalletAndSpentFlag() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        let walletA = walletId(0xAA)
        let walletB = walletId(0xBB)

        context.insert(makeNote(walletId: walletA, nullifier: 0x01, isSpent: false, value: 100))
        context.insert(makeNote(walletId: walletA, nullifier: 0x02, isSpent: true, value: 999))
        context.insert(makeNote(walletId: walletB, nullifier: 0x03, isSpent: false, value: 50))
        try context.save()

        let walletANotes = try context.fetch(
            FetchDescriptor<PersistentShieldedNote>(
                predicate: PersistentShieldedNote.unspentPredicate(walletId: walletA)
            )
        )
        XCTAssertEqual(nullifiers(walletANotes), [nullifier(0x01)])

        let allUnspent = try context.fetch(
            FetchDescriptor<PersistentShieldedNote>(
                predicate: PersistentShieldedNote.unspentPredicate
            )
        )
        XCTAssertEqual(nullifiers(allUnspent), [nullifier(0x01), nullifier(0x03)])
    }

    /// The per-wallet unspent rows cover every account, identity accounts
    /// included, so a wallet whose notes sit only in an identity account
    /// still counts as not empty.
    func testShouldLoadUnspentRowsOfEveryAccountIncludingIdentityAccounts() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let wallet = walletId(0xAA)
        context.insert(makeNote(walletId: wallet, nullifier: 0x01, isSpent: false, value: 100))
        context.insert(makeNote(walletId: wallet, accountIndex: 1,
                                nullifier: 0x02, isSpent: false, value: 50))
        context.insert(makeNote(walletId: wallet, accountIndex: 0x4000_0000,
                                nullifier: 0x03, isSpent: false, value: 900))
        try context.save()

        let notes = try context.fetch(FetchDescriptor<PersistentShieldedNote>(
            predicate: PersistentShieldedNote.unspentPredicate(walletId: wallet)
        ))
        XCTAssertEqual(nullifiers(notes), [nullifier(0x01), nullifier(0x02), nullifier(0x03)])
    }

    func testShouldLoadNoSendSourceRowsForIdentityAccountOnlyWallet() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let wallet = walletId(0xAA)
        context.insert(makeNote(walletId: wallet, accountIndex: 0x4000_0000,
                                nullifier: 0x01, isSpent: false, value: 900))
        try context.save()

        let notes = try context.fetch(FetchDescriptor<PersistentShieldedNote>(
            predicate: SendTransactionView.shieldedSourcePredicate(walletId: wallet)
        ))
        XCTAssertTrue(notes.isEmpty)
    }

    func testShouldLoadOnlyUnspentAccountZeroSendSourceRowsOfSelectedWallet() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let wallet = walletId(0xAA)
        context.insert(makeNote(walletId: wallet, nullifier: 0x01, isSpent: false, value: 100))
        context.insert(makeNote(walletId: wallet, nullifier: 0x02, isSpent: false, value: 50))
        context.insert(makeNote(walletId: wallet, nullifier: 0x03, isSpent: true, value: 999))
        context.insert(makeNote(walletId: wallet, accountIndex: 1,
                                nullifier: 0x04, isSpent: false, value: 200))
        context.insert(makeNote(walletId: wallet, accountIndex: 0x4000_0000,
                                nullifier: 0x05, isSpent: false, value: 900))
        context.insert(makeNote(walletId: walletId(0xBB),
                                nullifier: 0x06, isSpent: false, value: 300))
        try context.save()

        let notes = try context.fetch(FetchDescriptor<PersistentShieldedNote>(
            predicate: SendTransactionView.shieldedSourcePredicate(walletId: wallet)
        ))
        XCTAssertEqual(nullifiers(notes), [nullifier(0x01), nullifier(0x02)])
    }

    func testShouldLoadNoIdentityFundingRowsForIdentityAccountOnlyWallet() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let wallet = walletId(0xAA)
        context.insert(makeNote(walletId: wallet, accountIndex: 0x4000_0000,
                                nullifier: 0x01, isSpent: false, value: 900))
        try context.save()

        let notes = try context.fetch(FetchDescriptor<PersistentShieldedNote>(
            predicate: CreateIdentityView.shieldedFundingPredicate
        ))
        XCTAssertTrue(notes.isEmpty)
    }

    func testShouldLoadOnlyUnspentAccountZeroIdentityFundingRowsOfEveryWallet() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let wallet = walletId(0xAA)
        let otherWallet = walletId(0xBB)
        context.insert(makeNote(walletId: wallet, nullifier: 0x01, isSpent: false, value: 100))
        context.insert(makeNote(walletId: wallet, nullifier: 0x02, isSpent: false, value: 50))
        context.insert(makeNote(walletId: wallet, nullifier: 0x03, isSpent: true, value: 999))
        context.insert(makeNote(walletId: wallet, accountIndex: 1,
                                nullifier: 0x04, isSpent: false, value: 200))
        context.insert(makeNote(walletId: wallet, accountIndex: 0x4000_0000,
                                nullifier: 0x05, isSpent: false, value: 900))
        context.insert(makeNote(walletId: otherWallet,
                                nullifier: 0x06, isSpent: false, value: 300))
        try context.save()

        let notes = try context.fetch(FetchDescriptor<PersistentShieldedNote>(
            predicate: CreateIdentityView.shieldedFundingPredicate
        ))
        XCTAssertEqual(nullifiers(notes), [nullifier(0x01), nullifier(0x02), nullifier(0x06)])
    }

    private func makeNote(
        walletId: Data,
        accountIndex: UInt32 = 0,
        nullifier: UInt8,
        isSpent: Bool,
        value: UInt64
    ) -> PersistentShieldedNote {
        PersistentShieldedNote(
            walletId: walletId,
            accountIndex: accountIndex,
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
