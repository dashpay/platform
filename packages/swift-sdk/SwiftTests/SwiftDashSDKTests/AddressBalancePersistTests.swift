import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for `PlatformWalletPersistenceHandler.persistAddressBalances`
/// — the incremental BLAST / reconcile balance-update path.
///
/// The regression these tests pin: a reconcile *removal* (a fully
/// consumed input) is emitted from Rust carrying a *pool-resolved*
/// `addressIndex` that can collide with a different, funded address's
/// true derivation index (`commit_reconciliation`'s index-conflict
/// removal path deliberately still emits the zero so the balance can't
/// resurrect). The balance-update callback must NOT let that conflicting
/// index overwrite the row's authoritative index — otherwise two durable
/// rows end up claiming one `(accountIndex, addressIndex)` slot, and on
/// the next restore the Rust bijection rebuild
/// (`PerAccountPlatformAddressState::insert_persisted_entry`) drops the
/// funded pairing and orphans its balance.
///
/// The derivation index is owned by the address-emit path; this callback
/// only refreshes the volatile balance / nonce / `isUsed` fields.
@MainActor
final class AddressBalancePersistTests: XCTestCase {

    private let walletId = Data(repeating: 0x01, count: 32)
    private let accountIndex: UInt32 = 0

    // Two distinct addresses. `funded` legitimately owns derivation
    // index 5; `removed` legitimately owns index 2. The reconcile
    // removal for `removed` will (wrongly) carry index 5.
    private let fundedHash = Data(repeating: 0x77, count: 20)
    private let removedHash = Data(repeating: 0x22, count: 20)
    private let fundedIndex: UInt32 = 5
    private let removedIndex: UInt32 = 2
    private let conflictingIndex: UInt32 = 5

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// Seed a funded `PersistentPlatformAddress` row through a sibling
    /// context so the handler's own background context reads it back from
    /// the shared in-memory store — mirroring how the address-emit path
    /// seeds rows before balance updates arrive.
    private func seedRow(
        in container: ModelContainer,
        addressLabel: String,
        addressHash: Data,
        addressIndex: UInt32,
        balance: UInt64,
        nonce: UInt32
    ) throws {
        let context = ModelContext(container)
        let row = PersistentPlatformAddress(
            address: addressLabel,
            addressType: 0,
            addressHash: addressHash,
            publicKey: Data(),
            accountIndex: accountIndex,
            addressIndex: addressIndex,
            derivationPath: "m/9'/1'/17'/0'/\(accountIndex)'/\(addressIndex)",
            isUsed: true,
            balance: balance,
            nonce: nonce,
            walletId: walletId
        )
        context.insert(row)
        try context.save()
    }

    private func loadedRow(
        _ handler: PlatformWalletPersistenceHandler,
        hashByte: UInt8
    ) -> (balance: UInt64, nonce: UInt32, accountIndex: UInt32, addressIndex: UInt32, asOfHeight: UInt64)? {
        let rows = handler.loadCachedBalances(walletId: walletId)
        for (_, hash, balance, nonce, accountIndex, addressIndex, asOfHeight) in rows
        where hash.allSatisfy({ $0 == hashByte }) && hash.count == 20 {
            return (balance, nonce, accountIndex, addressIndex, asOfHeight)
        }
        return nil
    }

    /// The core regression: a zero-balance removal whose emitted
    /// `addressIndex` conflicts with a funded address's index must zero
    /// the removed row's balance WITHOUT stealing the funded address's
    /// index. Both durable rows keep their own indices, so the store
    /// stays a bijection and the Rust restore can't orphan the balance.
    func testConflictingRemovalDoesNotStealFundedIndex() throws {
        let (handler, container) = try makeHandler()

        // Address-emit seeded both rows at their true indices; both are
        // currently funded.
        try seedRow(
            in: container,
            addressLabel: "fixture-funded-77",
            addressHash: fundedHash,
            addressIndex: fundedIndex,
            balance: 200,
            nonce: 3
        )
        try seedRow(
            in: container,
            addressLabel: "fixture-removed-22",
            addressHash: removedHash,
            addressIndex: removedIndex,
            balance: 500,
            nonce: 4
        )

        // Reconcile removal for `removed`: fully consumed, zero funds,
        // and — the hazard — carrying `funded`'s index 5, not its own 2.
        let removal: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32, UInt64)] = [
            (0, removedHash, 0, 0, accountIndex, conflictingIndex, 379_790)
        ]
        handler.persistAddressBalances(walletId: walletId, entries: removal)

        let funded = try XCTUnwrap(loadedRow(handler, hashByte: 0x77))
        let removed = try XCTUnwrap(loadedRow(handler, hashByte: 0x22))

        // The removed row is zeroed...
        XCTAssertEqual(removed.balance, 0, "removal must zero the balance")
        XCTAssertEqual(removed.nonce, 0)
        // ...but keeps its OWN index — the conflicting index 5 was ignored.
        XCTAssertEqual(
            removed.addressIndex, removedIndex,
            "the balance path must not overwrite the row's authoritative derivation index"
        )

        // The funded row is untouched: same index, same balance.
        XCTAssertEqual(funded.addressIndex, fundedIndex)
        XCTAssertEqual(funded.balance, 200)

        // The durable store is still a bijection: the two rows do not
        // share an index, so the Rust restore rebuild can't evict either
        // pairing.
        XCTAssertNotEqual(
            funded.addressIndex, removed.addressIndex,
            "no two durable rows may claim the same derivation index"
        )
    }

    /// A normal (non-conflicting) balance update still refreshes the
    /// volatile fields and leaves the derivation index exactly as the
    /// address-emit path set it.
    func testBalanceUpdatePreservesDerivationIndex() throws {
        let (handler, container) = try makeHandler()

        try seedRow(
            in: container,
            addressLabel: "fixture-funded-77",
            addressHash: fundedHash,
            addressIndex: fundedIndex,
            balance: 0,
            nonce: 0
        )

        // BLAST reports a fresh balance pinned at the pass's proof
        // height; the entry echoes the true index.
        let update: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32, UInt64)] = [
            (0, fundedHash, 1_000, 7, accountIndex, fundedIndex, 379_784)
        ]
        handler.persistAddressBalances(walletId: walletId, entries: update)

        let funded = try XCTUnwrap(loadedRow(handler, hashByte: 0x77))
        XCTAssertEqual(funded.balance, 1_000)
        XCTAssertEqual(funded.nonce, 7)
        XCTAssertEqual(funded.addressIndex, fundedIndex)
        XCTAssertEqual(
            funded.asOfHeight, 379_784,
            "the balance height pin must round-trip through lastSeenHeight"
        )
    }

    /// A balance update for an address that was never address-emitted
    /// (no row exists) is skipped — no phantom row, no stray index.
    func testBalanceUpdateForUnknownAddressIsSkipped() throws {
        let (handler, _) = try makeHandler()

        let update: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32, UInt64)] = [
            (0, removedHash, 42, 1, accountIndex, conflictingIndex, 100)
        ]
        handler.persistAddressBalances(walletId: walletId, entries: update)

        XCTAssertTrue(
            handler.loadCachedBalances(walletId: walletId).isEmpty,
            "no row should be created for a never-emitted address"
        )
    }
}
