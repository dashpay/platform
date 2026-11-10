import SwiftData
import XCTest

@testable import SwiftDashSDK

/// A shielded sync pass can deliver a note unspent after a concurrent send's
/// round already marked it spent. The persister must keep the spent flag:
/// the Rust store never un-spends a note, so a durable row that did would
/// offer the spent note again after a restart.
@MainActor
final class ShieldedNoteSpentPersistenceTests: XCTestCase {

    private let walletId = Data(repeating: 0xD1, count: 32)
    private let nullifier = Data(repeating: 0xD2, count: 32)

    private func saveNote(_ handler: PlatformWalletPersistenceHandler, isSpent: Bool) throws {
        handler.beginChangeset(walletId: walletId)
        try handler.persistShieldedNotes(
            walletId: walletId,
            snapshots: [
                .init(
                    walletId: walletId,
                    accountIndex: 0,
                    position: 8,
                    cmx: Data(repeating: 0xD3, count: 32),
                    nullifier: nullifier,
                    blockHeight: 51,
                    isSpent: isSpent,
                    value: 200_000,
                    noteData: Data(repeating: 0xD4, count: 115)
                )
            ]
        )
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    private func storedNote(_ container: ModelContainer) throws -> PersistentShieldedNote {
        let context = ModelContext(container)
        let rows = try context.fetch(FetchDescriptor<PersistentShieldedNote>())
        XCTAssertEqual(rows.count, 1, "one row per nullifier")
        return try XCTUnwrap(rows.first)
    }

    func testResaveNeverClearsSpent() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)

        try saveNote(handler, isSpent: false)
        // A send's round marks the note spent.
        handler.beginChangeset(walletId: walletId)
        handler.persistShieldedNullifiersSpent(
            walletId: walletId,
            entries: [(walletId: walletId, accountIndex: 0, nullifier: nullifier)]
        )
        _ = handler.endChangeset(walletId: walletId, success: true)
        XCTAssertTrue(try storedNote(container).isSpent)

        // A sync pass's later round re-saves it unspent.
        try saveNote(handler, isSpent: false)
        XCTAssertTrue(try storedNote(container).isSpent, "a re-save must not un-spend a note")
    }

    /// A send can confirm a note before the sync pass that found it has
    /// stored it. The spend's round then inserts the note spent, and the
    /// pass's later receipt must not clear it.
    func testReceiptAfterTheSpendRoundKeepsTheNoteSpent() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)

        try saveNote(handler, isSpent: true)
        try saveNote(handler, isSpent: false)
        XCTAssertTrue(try storedNote(container).isSpent)
    }

    /// The existing-row read is what keeps a spent flag. If it fails, the
    /// save must fail the round, not insert the stale copy over the row.
    func testUnreadableRowFailsTheSaveInsteadOfInsertingItUnspent() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)
        try saveNote(handler, isSpent: true)

        let faulting = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet,
            modelFetcher: FetchFaultInjector(faulting: PersistentShieldedNote.self)
        )
        faulting.beginChangeset(walletId: walletId)
        XCTAssertThrowsError(try faulting.persistShieldedNotes(
            walletId: walletId,
            snapshots: [
                .init(
                    walletId: walletId,
                    accountIndex: 0,
                    position: 8,
                    cmx: Data(repeating: 0xD3, count: 32),
                    nullifier: nullifier,
                    blockHeight: 51,
                    isSpent: false,
                    value: 200_000,
                    noteData: Data(repeating: 0xD4, count: 115)
                )
            ]
        ))
        _ = faulting.endChangeset(walletId: walletId, success: false)

        XCTAssertTrue(try storedNote(container).isSpent)
    }

    func testResaveStillMarksAnUnspentNoteSpent() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)

        try saveNote(handler, isSpent: false)
        XCTAssertFalse(try storedNote(container).isSpent)
        try saveNote(handler, isSpent: true)
        XCTAssertTrue(try storedNote(container).isSpent)
    }
}
