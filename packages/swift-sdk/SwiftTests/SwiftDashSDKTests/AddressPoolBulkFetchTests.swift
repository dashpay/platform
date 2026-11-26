import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Pins the chunked `IN` fetches of the address-pool persist paths.
///
/// One address emit carries a whole pool slice — thousands of entries on a
/// restore — and both `persistAccountAddresses` (Core pools) and
/// `persistPlatformPaymentAddresses` (DIP-17 pools) used to issue a fetch
/// per entry. The platform-address fetch and the Core TXO backfill keep
/// pending changes included, so each per-entry fetch also scanned every row
/// of that model the round had staged: quadratic over a large round. These
/// tests count the reads through the handler's `ModelFetching` seam, so a
/// regression back to per-entry fetches fails on the count, and they pin
/// that a failed chunk falls back to per-row reads instead of dropping rows.
@MainActor
final class AddressPoolBulkFetchTests: XCTestCase {

    private let walletId = Data(repeating: 0x0A, count: 32)
    private let coreTypeTag: UInt32 = 0
    private let platformPaymentTypeTag: UInt32 = 14

    // MARK: - Fixtures

    private func makeHandler(
        fetcher: FetchFaultInjector
    ) throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet,
            modelFetcher: fetcher
        )
        return (handler, container)
    }

    private func seedAccount(in container: ModelContainer, typeTag: UInt32, name: String = "Test") throws {
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet, accountType: typeTag, accountIndex: 0, accountTypeName: name)
        account.userIdentityId = Data(count: 32)
        account.friendIdentityId = Data(count: 32)
        context.insert(account)
        try context.save()
    }

    private func lookupKey(typeTag: UInt32) -> PlatformWalletPersistenceHandler.AccountLookupKey {
        .init(
            typeTag: typeTag,
            index: 0,
            standardTag: 0,
            registrationIndex: 0,
            keyClass: 0,
            userIdentityId: Data(count: 32),
            friendIdentityId: Data(count: 32)
        )
    }

    private func coreAddress(_ i: Int) -> String { "yBulkFetchAddress\(i)" }

    /// A valid testnet DIP-18 P2PKH platform address whose hash encodes `i`.
    private func platformAddress(_ i: Int) throws -> String {
        var payload = Data([0xb0])
        payload.append(withUnsafeBytes(of: UInt64(i).littleEndian) { Data($0) })
        payload.append(Data(count: 12))
        return try XCTUnwrap(Bech32m.encode(hrp: Bech32m.platformHrp(mainnet: false), data: payload))
    }

    private func entry(
        _ address: String,
        index: Int,
        isUsed: Bool = false,
        balance: UInt64 = 0
    ) -> PlatformWalletPersistenceHandler.CoreAddressEntrySnapshot {
        .init(
            address: address,
            publicKey: Data(count: 33),
            keyType: 0,
            poolTypeTag: 0,
            addressIndex: UInt32(index),
            isUsed: isUsed,
            balance: balance,
            derivationPath: "m/44'/1'/0'/0/\(index)"
        )
    }

    /// A saved TXO at `address` whose `coreAddress` link is still nil —
    /// what the SPV pass leaves when a UTXO lands before its address row.
    private func seedUnlinkedTxo(in container: ModelContainer, address: String, seed: UInt8) throws {
        let context = ModelContext(container)
        let tx = PersistentTransaction(txid: Data(repeating: seed, count: 32), transactionData: Data())
        context.insert(tx)
        context.insert(PersistentTxo(transaction: tx, vout: 0, amount: 1_000, address: address))
        try context.save()
    }

    private func seedCoreAddressRow(in container: ModelContainer, address: String) throws {
        let context = ModelContext(container)
        context.insert(PersistentCoreAddress(
            address: address, poolTypeTag: 0, addressIndex: 0, derivationPath: "m/0", isUsed: false))
        try context.save()
    }

    private func coreAddressRows(_ container: ModelContainer) throws -> [PersistentCoreAddress] {
        try ModelContext(container).fetch(FetchDescriptor<PersistentCoreAddress>())
    }

    private func linkedAddress(ofTxoAt address: String, in container: ModelContainer) throws -> String? {
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.address == address }
        ))
        return try XCTUnwrap(rows.first).coreAddress?.address
    }

    private func persistInRound(
        _ handler: PlatformWalletPersistenceHandler,
        typeTag: UInt32,
        entries: [PlatformWalletPersistenceHandler.CoreAddressEntrySnapshot]
    ) {
        handler.beginChangeset(walletId: walletId)
        let ok = handler.persistAccountAddresses(
            walletId: walletId, accountKey: lookupKey(typeTag: typeTag), entries: entries)
        XCTAssertTrue(ok)
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    // MARK: - Helpers

    func testChunkedSplitsBelowTheBindLimitAndKeepsOrder() {
        let keys = Array(0..<2_000)
        let chunks = PlatformWalletPersistenceHandler.chunked(keys)
        XCTAssertEqual(chunks.map(\.count), [900, 900, 200])
        XCTAssertEqual(Array(chunks.joined()), keys)
        XCTAssertEqual(PlatformWalletPersistenceHandler.chunked(Array(0..<900)).map(\.count), [900])
        XCTAssertTrue(PlatformWalletPersistenceHandler.chunked([Int]()).isEmpty)
        XCTAssertEqual(
            PlatformWalletPersistenceHandler.uniqueInOrder(["b", "a", "b", "c", "a"]),
            ["b", "a", "c"]
        )
    }

    /// The contract both paths build on: a `[String]`-captured `contains`
    /// predicate translates to SQL `IN` across chunks, a store-only fetch
    /// sees saved rows only, and the default fetch also sees staged rows.
    func testStringContainsPredicateSeesSavedRowsAndOnlyThePendingFetchSeesStagedRows() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        for i in 0..<2_000 {
            context.insert(PersistentCoreAddress(
                address: coreAddress(i), poolTypeTag: 0, addressIndex: UInt32(i), derivationPath: "m/\(i)"))
        }
        try context.save()
        context.insert(PersistentCoreAddress(
            address: "staged", poolTypeTag: 0, addressIndex: 0, derivationPath: "m/s"))

        let keys = (0..<2_000).map(coreAddress) + ["staged", "missing"]
        func fetchAll(includePending: Bool) throws -> Set<String> {
            var found = Set<String>()
            for chunk in PlatformWalletPersistenceHandler.chunked(keys) {
                var descriptor = FetchDescriptor<PersistentCoreAddress>(
                    predicate: #Predicate { chunk.contains($0.address) }
                )
                descriptor.includePendingChanges = includePending
                found.formUnion(try context.fetch(descriptor).map(\.address))
            }
            return found
        }

        let storeOnly = try fetchAll(includePending: false)
        XCTAssertEqual(storeOnly.count, 2_000)
        XCTAssertFalse(storeOnly.contains("staged"))
        let withPending = try fetchAll(includePending: true)
        XCTAssertEqual(withPending.count, 2_001)
        XCTAssertTrue(withPending.contains("staged"))
    }

    // MARK: - Core address pools

    func testCoreAddressEmitIssuesOneReadPerChunk() throws {
        let fetcher = FetchFaultInjector()
        let (handler, container) = try makeHandler(fetcher: fetcher)
        try seedAccount(in: container, typeTag: coreTypeTag)

        persistInRound(handler, typeTag: coreTypeTag, entries: (0..<2_000).map { entry(coreAddress($0), index: $0) })

        // 2,000 addresses = 3 chunks, for the row prime and the backfill.
        XCTAssertEqual(fetcher.readCount(of: PersistentCoreAddress.self), 3)
        XCTAssertEqual(fetcher.readCount(of: PersistentTxo.self), 3)
        XCTAssertEqual(try coreAddressRows(container).count, 2_000)
    }

    func testCoreAddressEmitUpdatesExistingRowsDedupesAndBackfillsTxos() throws {
        let fetcher = FetchFaultInjector()
        let (handler, container) = try makeHandler(fetcher: fetcher)
        try seedAccount(in: container, typeTag: coreTypeTag)
        try seedCoreAddressRow(in: container, address: coreAddress(0))
        try seedUnlinkedTxo(in: container, address: coreAddress(1), seed: 0x11)

        persistInRound(handler, typeTag: coreTypeTag, entries: [
            entry(coreAddress(0), index: 0, isUsed: true),
            entry(coreAddress(1), index: 1),
            entry(coreAddress(1), index: 1, isUsed: true),
            entry(coreAddress(2), index: 2),
        ])

        let rows = try coreAddressRows(container)
        XCTAssertEqual(Set(rows.map(\.address)), [coreAddress(0), coreAddress(1), coreAddress(2)])
        XCTAssertEqual(rows.count, 3, "a repeated or pre-existing address must not insert a second row")
        XCTAssertTrue(try XCTUnwrap(rows.first { $0.address == coreAddress(0) }).isUsed)
        XCTAssertTrue(try XCTUnwrap(rows.first { $0.address == coreAddress(1) }).isUsed)
        XCTAssertEqual(try linkedAddress(ofTxoAt: coreAddress(1), in: container), coreAddress(1))
        // The repeat was de-duplicated out of the `IN` lists, not re-fetched.
        XCTAssertEqual(fetcher.readCount(of: PersistentCoreAddress.self), 1)
        XCTAssertEqual(fetcher.readCount(of: PersistentTxo.self), 1)
    }

    /// A thrown backfill chunk falls back to one fetch per address, so the
    /// TXO still gets its link (the per-entry `try?` lost only one address
    /// per failure; a whole chunk must not be skipped).
    func testFailedTxoBackfillChunkFallsBackPerAddress() throws {
        let fetcher = FetchFaultInjector(faulting: PersistentTxo.self, faultLimit: 1)
        let (handler, container) = try makeHandler(fetcher: fetcher)
        try seedAccount(in: container, typeTag: coreTypeTag)
        try seedUnlinkedTxo(in: container, address: coreAddress(1), seed: 0x21)

        persistInRound(handler, typeTag: coreTypeTag, entries: (0..<3).map { entry(coreAddress($0), index: $0) })

        XCTAssertEqual(try linkedAddress(ofTxoAt: coreAddress(1), in: container), coreAddress(1))
        XCTAssertEqual(fetcher.readCount(of: PersistentTxo.self), 1 + 3)
    }

    /// A thrown row-prime chunk proves nothing absent, so each address takes
    /// the per-row lookup and an existing row is updated, not shadowed.
    func testFailedAddressRowChunkFallsBackPerRow() throws {
        let fetcher = FetchFaultInjector(faulting: PersistentCoreAddress.self, faultLimit: 1)
        let (handler, container) = try makeHandler(fetcher: fetcher)
        try seedAccount(in: container, typeTag: coreTypeTag)
        try seedCoreAddressRow(in: container, address: coreAddress(0))

        persistInRound(handler, typeTag: coreTypeTag, entries: [
            entry(coreAddress(0), index: 0, isUsed: true),
            entry(coreAddress(1), index: 1),
        ])

        let rows = try coreAddressRows(container)
        XCTAssertEqual(rows.count, 2)
        XCTAssertTrue(try XCTUnwrap(rows.first { $0.address == coreAddress(0) }).isUsed)
        // The faulted prime, then one per-row fallback per address — and no
        // more: the fallback reads go through the same seam.
        XCTAssertEqual(fetcher.readCount(of: PersistentCoreAddress.self), 1 + 2)
    }

    // MARK: - Same-round preservation

    /// The batching is only equivalent to the per-entry fetches if it honours
    /// what the round already staged. Within ONE round, through the handler:
    /// a wallet changeset first stages a new TXO and updates a saved one
    /// (spent by verdict, confirmed, InstantSend-locked, new height); then two
    /// overlapping address emits follow. The backfill must find the staged
    /// insert and keep the saved row's unsaved updates (it needs pending
    /// changes), and the second emit must not re-fetch the address rows the
    /// first one registered (the untouched-key filter on the store-only
    /// prime, which would otherwise refresh registered rows to store values).
    func testBulkLookupsHonourEarlierWritesInTheSameRound() throws {
        let fetcher = FetchFaultInjector()
        let (handler, container) = try makeHandler(fetcher: fetcher)
        try seedAccount(in: container, typeTag: coreTypeTag, name: "Standard { index: 0 }")
        let savedTxid = Data(repeating: 0x31, count: 32)
        let newTxid = Data(repeating: 0x32, count: 32)
        let savedAddress = coreAddress(10)
        let newAddress = coreAddress(11)
        try seedUnlinkedTxo(in: container, address: savedAddress, seed: savedTxid[0])

        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(stageObservedSpentVerdict(handler, txid: savedTxid, vout: 0))
        XCTAssertTrue(stageUtxosAdded(handler, [
            (savedTxid, savedAddress, 2_400_001),
            (newTxid, newAddress, 2_400_002),
        ]))

        let coreReadsBefore = fetcher.readCount(of: PersistentCoreAddress.self)
        let txoReadsBefore = fetcher.readCount(of: PersistentTxo.self)
        XCTAssertTrue(handler.persistAccountAddresses(
            walletId: walletId, accountKey: lookupKey(typeTag: coreTypeTag),
            entries: [entry(savedAddress, index: 10), entry(newAddress, index: 11), entry(coreAddress(12), index: 12)]))
        XCTAssertEqual(fetcher.readCount(of: PersistentCoreAddress.self) - coreReadsBefore, 1)
        XCTAssertEqual(fetcher.readCount(of: PersistentTxo.self) - txoReadsBefore, 1)

        // Overlapping re-emit in the same round: every key is registered.
        let coreReadsMid = fetcher.readCount(of: PersistentCoreAddress.self)
        XCTAssertTrue(handler.persistAccountAddresses(
            walletId: walletId, accountKey: lookupKey(typeTag: coreTypeTag),
            entries: [entry(newAddress, index: 11, isUsed: true), entry(coreAddress(12), index: 12)]))
        XCTAssertEqual(
            fetcher.readCount(of: PersistentCoreAddress.self) - coreReadsMid, 0,
            "registered address rows must be answered from the round index, not re-fetched")
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        XCTAssertEqual(try linkedAddress(ofTxoAt: savedAddress, in: container), savedAddress)
        XCTAssertEqual(try linkedAddress(ofTxoAt: newAddress, in: container), newAddress)
        let saved = try XCTUnwrap(try ModelContext(container).fetch(FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.address == savedAddress }
        )).first)
        XCTAssertTrue(saved.isSpent, "the staged spend must survive the backfill fetch")
        XCTAssertTrue(saved.isConfirmed)
        XCTAssertTrue(saved.isInstantLocked)
        XCTAssertEqual(saved.height, 2_400_001)
        let rows = try coreAddressRows(container)
        XCTAssertEqual(rows.count, 3)
        XCTAssertTrue(try XCTUnwrap(rows.first { $0.address == newAddress }).isUsed)
    }

    private func stageObservedSpentVerdict(
        _ handler: PlatformWalletPersistenceHandler,
        txid: Data,
        vout: UInt32
    ) -> Bool {
        var verdict = UtxoCreditVerdictFFI()
        Swift.withUnsafeMutableBytes(of: &verdict.outpoint.txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        verdict.outpoint.vout = vout
        verdict.verdict = 1  // observed spent
        verdict.spent_at_height = 2_400_010
        return withUnsafePointer(to: &verdict) { ptr in
            handler.persistWalletChangesetUtxoVerdicts(walletId: walletId, verdicts: ptr, count: 1)
        }
    }

    /// One wallet changeset whose BIP44 account adds a confirmed,
    /// InstantSend-locked UTXO at vout 0 of each `(txid, address, height)`.
    private func stageUtxosAdded(
        _ handler: PlatformWalletPersistenceHandler,
        _ outputs: [(txid: Data, address: String, height: UInt32)]
    ) -> Bool {
        let name = strdup("Standard { index: 0 }")
        let addresses = outputs.map { strdup($0.address) }
        defer {
            free(name)
            addresses.forEach { free($0) }
        }
        var utxos: [UtxoEntryFFI] = outputs.enumerated().map { i, output in
            var utxo = UtxoEntryFFI()
            Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
                output.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            utxo.outpoint.vout = 0
            utxo.amount = 1_000
            utxo.address = addresses[i]
            utxo.height = output.height
            utxo.is_confirmed = true
            utxo.is_instantlocked = true
            return utxo
        }
        return utxos.withUnsafeMutableBufferPointer { utxoPtr in
            var account = AccountChangeSetFFI()
            account.account_type_name = name
            account.utxos_added = utxoPtr.baseAddress
            account.utxos_added_count = UInt(utxoPtr.count)
            return withUnsafeMutablePointer(to: &account) { accountPtr in
                var cs = WalletChangeSetFFI()
                cs.accounts = accountPtr
                cs.accounts_count = 1
                return withUnsafePointer(to: &cs) { csPtr in
                    handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                }
            }
        }
    }

    // MARK: - Platform payment (DIP-17) pools

    func testPlatformPaymentEmitIssuesOneReadPerChunkDedupesAndKeepsBalances() throws {
        let fetcher = FetchFaultInjector()
        let (handler, container) = try makeHandler(fetcher: fetcher)
        try seedAccount(in: container, typeTag: platformPaymentTypeTag)
        let addresses = try (0..<1_000).map(platformAddress)
        try seedPlatformRow(in: container, address: addresses[0], balance: 5)

        var entries = addresses.enumerated().map { entry($1, index: $0) }
        entries.append(entry(addresses[999], index: 999, isUsed: true))
        persistInRound(handler, typeTag: platformPaymentTypeTag, entries: entries)

        // 1,000 distinct addresses = 2 chunks; the repeat costs no read.
        XCTAssertEqual(fetcher.readCount(of: PersistentPlatformAddress.self), 2)
        let rows = try platformRows(container)
        XCTAssertEqual(rows.count, 1_000)
        XCTAssertEqual(rows[addresses[0]]?.balance, 5, "an existing row is updated in place")
        XCTAssertEqual(rows[addresses[999]]?.isUsed, true, "the repeat updated the staged row")
    }

    func testFailedPlatformPaymentChunkFallsBackPerRow() throws {
        let fetcher = FetchFaultInjector(faulting: PersistentPlatformAddress.self, faultLimit: 1)
        let (handler, container) = try makeHandler(fetcher: fetcher)
        try seedAccount(in: container, typeTag: platformPaymentTypeTag)
        let addresses = try (0..<3).map(platformAddress)
        try seedPlatformRow(in: container, address: addresses[0], balance: 5)

        persistInRound(
            handler, typeTag: platformPaymentTypeTag,
            entries: addresses.enumerated().map { entry($1, index: $0) })

        XCTAssertEqual(fetcher.readCount(of: PersistentPlatformAddress.self), 1 + 3)
        let rows = try platformRows(container)
        XCTAssertEqual(rows.count, 3)
        XCTAssertEqual(rows[addresses[0]]?.balance, 5)
    }

    private func seedPlatformRow(in container: ModelContainer, address: String, balance: UInt64) throws {
        let context = ModelContext(container)
        context.insert(PersistentPlatformAddress(
            address: address,
            addressType: 0,
            addressHash: Data(repeating: 0xEE, count: 20),
            publicKey: Data(),
            accountIndex: 0,
            addressIndex: 0,
            derivationPath: "m/9'/1'/17'/0'/0'/0",
            isUsed: true,
            balance: balance,
            nonce: 0,
            walletId: walletId
        ))
        try context.save()
    }

    private func platformRows(_ container: ModelContainer) throws -> [String: PersistentPlatformAddress] {
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentPlatformAddress>())
        return Dictionary(rows.map { ($0.address, $0) }, uniquingKeysWith: { first, _ in first })
    }
}
