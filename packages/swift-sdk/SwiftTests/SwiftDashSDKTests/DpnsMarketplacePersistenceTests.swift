import XCTest
import SwiftData
@testable import SwiftDashSDK

final class DpnsMarketplacePersistenceTests: XCTestCase {
    private var container: ModelContainer!
    private var handler: PlatformWalletPersistenceHandler!

    private let walletId = Data(repeating: 0xAA, count: 32)
    private let ownerId = Data(repeating: 0x11, count: 32)
    private let nextOwnerId = Data(repeating: 0x22, count: 32)

    override func setUpWithError() throws {
        try super.setUpWithError()
        container = try DashModelContainer.createInMemory()
        handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet
        )
    }

    override func tearDown() {
        handler = nil
        container = nil
        super.tearDown()
    }

    private func identitySnapshot(
        id: Data,
        names: [(label: String, acquiredAt: UInt64)]
    ) -> PlatformWalletPersistenceHandler.IdentityEntrySnapshot {
        .init(
            identityId: id,
            balance: 0,
            revision: 1,
            identityIndex: 0,
            label: nil,
            status: 0,
            walletId: walletId,
            dpnsNames: names,
            dashpayProfile: nil,
            contactProfiles: []
        )
    }

    private func applyIdentitySnapshot(
        id: Data = Data(repeating: 0x11, count: 32),
        names: [(label: String, acquiredAt: UInt64)]
    ) {
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(
            walletId: walletId,
            upserts: [identitySnapshot(id: id, names: names)],
            removed: []
        )
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    func testCanonicalSnapshotsHideDepartedHistoryAndDeleteCacheOnlyRows() throws {
        let context = ModelContext(container)
        let owner = PersistentIdentity(
            identityId: ownerId,
            isLocal: false,
            dpnsName: "Alice",
            mainDpnsName: "Alice",
            network: .testnet
        )
        let alice = PersistentDPNSName(identity: owner, label: "Alice", acquiredAt: 10)
        alice.documentIdBase58 = Data(repeating: 0x33, count: 32).toBase58String()
        let bob = PersistentDPNSName(identity: owner, label: "Bob", acquiredAt: 20)
        context.insert(owner)
        context.insert(alice)
        context.insert(bob)
        try context.save()

        // Alice leaves the canonical owned set. Keep its marketplace history,
        // while Bob becomes the fallback display/main name.
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(
            walletId: walletId,
            upserts: [identitySnapshot(id: ownerId, names: [("Bob", 20)])],
            removed: []
        )
        XCTAssertTrue(handler.persistDpnsNameStates(
            walletId: walletId,
            upserts: [
                .init(
                    documentIdBase58: alice.documentIdBase58!,
                    walletIdentityId: ownerId,
                    label: "Alice",
                    normalizedLabel: PersistentDPNSName.normalize("Alice"),
                    normalizedParentDomainName: PersistentDPNSName.normalize("dash"),
                    priceCredits: 5_000,
                    statusRaw: 1,
                    counterpartyIdBase58: Data(repeating: 0x44, count: 32).toBase58String(),
                    lastSyncedAtMs: 100
                )
            ],
            removed: []
        ))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        var readContext = ModelContext(container)
        var owned = try readContext.fetch(
            FetchDescriptor<PersistentDPNSName>(
                predicate: PersistentDPNSName.predicate(identityId: ownerId)
            )
        )
        XCTAssertEqual(owned.map(\.label), ["Bob"])
        var identity = try XCTUnwrap(PersistentIdentity.fetch(in: readContext, identityId: ownerId))
        XCTAssertEqual(identity.mainDpnsName, "Bob")
        XCTAssertEqual(identity.dpnsName, "Bob")

        // An empty canonical snapshot removes Bob (cache-only), keeps Alice's
        // sold history, and clears stale scalar selections.
        applyIdentitySnapshot(id: ownerId, names: [])
        readContext = ModelContext(container)
        owned = try readContext.fetch(
            FetchDescriptor<PersistentDPNSName>(
                predicate: PersistentDPNSName.predicate(identityId: ownerId)
            )
        )
        XCTAssertTrue(owned.isEmpty)
        let allRows = try readContext.fetch(FetchDescriptor<PersistentDPNSName>())
        XCTAssertEqual(allRows.count, 1)
        XCTAssertEqual(allRows.first?.label, "Alice")
        XCTAssertEqual(allRows.first?.isOwned, false)
        XCTAssertEqual(allRows.first?.saleStatusRaw, 1)
        identity = try XCTUnwrap(PersistentIdentity.fetch(in: readContext, identityId: ownerId))
        XCTAssertNil(identity.mainDpnsName)
        XCTAssertNil(identity.dpnsName)
    }

    func testCanonicalSnapshotRebindsUniqueNameToNewOwner() throws {
        let context = ModelContext(container)
        let oldOwner = PersistentIdentity(identityId: ownerId, isLocal: false, network: .testnet)
        let nextOwner = PersistentIdentity(
            identityId: nextOwnerId,
            isLocal: false,
            network: .testnet
        )
        let row = PersistentDPNSName(identity: oldOwner, label: "Alice")
        row.isOwned = false
        context.insert(oldOwner)
        context.insert(nextOwner)
        context.insert(row)
        try context.save()

        applyIdentitySnapshot(id: nextOwnerId, names: [("Alice", 30)])

        let readContext = ModelContext(container)
        let rows = try readContext.fetch(FetchDescriptor<PersistentDPNSName>())
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows.first?.identity.identityId, nextOwnerId)
        XCTAssertEqual(rows.first?.isOwned, true)
        XCTAssertEqual(rows.first?.acquiredAt, 30)
    }

    func testOptionalIdentifiersRequireExactly32Bytes() throws {
        XCTAssertNil(try ManagedPlatformWallet.validatedOptionalIdentifier(nil, parameter: "id"))
        XCTAssertEqual(
            try ManagedPlatformWallet.validatedOptionalIdentifier(
                Data(repeating: 1, count: 32),
                parameter: "id"
            )?.count,
            32
        )
        XCTAssertThrowsError(
            try ManagedPlatformWallet.validatedOptionalIdentifier(
                Data(repeating: 1, count: 31),
                parameter: "id"
            )
        )
        XCTAssertThrowsError(
            try ManagedPlatformWallet.validatedOptionalIdentifier(
                Data(repeating: 1, count: 33),
                parameter: "id"
            )
        )
    }
}
