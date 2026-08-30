import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Proving ground for the bulk `IN`-style fetches the wallet-changeset
/// round cache relies on (`PlatformWalletPersistenceHandler`'s
/// prefetch pass).
///
/// SwiftData translates `[Data].contains($0.column)` into a SQL
/// `IN (?, ?, …)` — but nothing else in this package exercised that
/// form before the round cache, and the sibling `Set.contains` form
/// famously does NOT translate (it throws at predicate-compile time).
/// These tests pin the exact contract the cache builder depends on:
///
/// 1. an `[Data]`-captured `contains` predicate round-trips BLOB keys
///    through the store, in chunks below SQLite's bind-variable limit;
/// 2. rows staged (unsaved) in the same context remain visible to the
///    bulk fetch (`includePendingChanges` default), which is what lets
///    the prefetch see rows earlier per-kind callbacks inserted in the
///    same begin/end changeset round.
@MainActor
final class BulkFetchPredicateTests: XCTestCase {

    func testChunkedDataContainsPredicateFetchesAllSavedRows() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        // More rows than one SQLite bind chunk (900) so the chunked
        // fetch path is genuinely exercised.
        let total = 2_000
        var outpoints: [Data] = []
        outpoints.reserveCapacity(total)
        for i in 0..<total {
            let tx = PersistentTransaction(txid: makeTxid(i), transactionData: Data())
            context.insert(tx)
            let txo = PersistentTxo(transaction: tx, vout: 0, amount: UInt64(i), address: "addr\(i)")
            context.insert(txo)
            outpoints.append(txo.outpoint)
        }
        try context.save()

        var fetched: [Data: PersistentTxo] = [:]
        for chunk in stride(from: 0, to: outpoints.count, by: 900).map({
            Array(outpoints[$0..<min($0 + 900, outpoints.count)])
        }) {
            let descriptor = FetchDescriptor<PersistentTxo>(
                predicate: #Predicate { chunk.contains($0.outpoint) }
            )
            for row in try context.fetch(descriptor) {
                fetched[row.outpoint] = row
            }
        }

        XCTAssertEqual(fetched.count, total)
        for outpoint in outpoints {
            XCTAssertNotNil(fetched[outpoint])
        }
        // Spot-check a payload survived the BLOB round trip.
        XCTAssertEqual(fetched[outpoints[1234]]?.amount, 1234)
    }

    func testDataContainsPredicateSeesUnsavedPendingRows() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        // One durably saved row, one staged-only row — the bulk fetch
        // must see both, exactly like a mid-round prefetch that runs
        // after earlier callbacks staged inserts without saving.
        let savedTx = PersistentTransaction(txid: makeTxid(1), transactionData: Data())
        context.insert(savedTx)
        try context.save()

        let pendingTx = PersistentTransaction(txid: makeTxid(2), transactionData: Data())
        context.insert(pendingTx)

        let txids = [makeTxid(1), makeTxid(2), makeTxid(3)]
        let descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { txids.contains($0.txid) }
        )
        let rows = try context.fetch(descriptor)

        XCTAssertEqual(Set(rows.map(\.txid)), [makeTxid(1), makeTxid(2)])
    }
}
