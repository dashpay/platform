import XCTest
import SwiftData
import DashSDKFFI
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

    /// A cold restore hands Rust every OWNED name the store knows, so the
    /// in-memory list is never empty/truncated before the first complete
    /// fetch: a capped fetch can then only add, and no snapshot reads an
    /// owned pick outside the fetched prefix as departed. Rows retained as
    /// not owned stay out of the restored list.
    func testColdRestoreCarriesEveryOwnedNameToRust() throws {
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "Standard")
        account.accountExtendedPubKeyBytes = Data(repeating: 0x30, count: 78)
        context.insert(account)
        let identity = PersistentIdentity(
            identityId: ownerId, mainDpnsName: "Carol", network: .testnet)
        identity.wallet = wallet
        context.insert(identity)
        for (label, acquiredAt, owned) in [
            ("Bob", UInt64(20), true), ("Alice", 10, true), ("Carol", 30, true), ("Dave", 5, false),
        ] {
            let row = PersistentDPNSName(identity: identity, label: label, acquiredAt: acquiredAt)
            row.isOwned = owned
            context.insert(row)
        }
        try context.save()

        let loaded = handler.loadWalletList()
        XCTAssertFalse(loaded.errored)
        let entries = try XCTUnwrap(loaded.entries)
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entries)) }
        let restored = try XCTUnwrap(entries[0].identities)[0]
        let names = try XCTUnwrap(restored.dpns_names)
        let labels = (0..<Int(restored.dpns_names_count)).map { String(cString: names[$0]!) }
        XCTAssertEqual(labels, ["Alice", "Bob", "Carol"])
    }

    func testMarketplaceColumnsHaveMigrationSafeDefaults() {
        let identity = PersistentIdentity(
            identityId: ownerId,
            isLocal: false,
            network: .testnet
        )
        let row = PersistentDPNSName(identity: identity, label: "Alice")

        XCTAssertNil(row.documentIdBase58)
        XCTAssertNil(row.priceCredits)
        XCTAssertEqual(row.saleStatusRaw, 0)
        XCTAssertNil(row.counterpartyIdBase58)
        XCTAssertNil(row.documentCreatedAtMs)
        XCTAssertNil(row.documentUpdatedAtMs)
        XCTAssertNil(row.documentTransferredAtMs)
        XCTAssertEqual(row.marketplaceUpdatedAt, 0)
        XCTAssertNil(row.saleStatus)
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
        // while Bob becomes the fallback display name. The main-name pick
        // stays as the user wrote it; readers skip it once it is not owned.
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
                    createdAtMs: 11,
                    updatedAtMs: 12,
                    transferredAtMs: 13,
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
        XCTAssertEqual(identity.mainDpnsName, "Alice")
        XCTAssertNil(identity.ownedMainDpnsName)
        XCTAssertEqual(identity.dpnsName, "Bob")
        XCTAssertEqual(identity.displayName, "Bob")

        // An empty canonical snapshot removes Bob (cache-only), keeps Alice's
        // sold history, and clears the stale display scalar.
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
        XCTAssertEqual(allRows.first?.documentCreatedAtMs, 11)
        XCTAssertEqual(allRows.first?.documentUpdatedAtMs, 12)
        XCTAssertEqual(allRows.first?.documentTransferredAtMs, 13)
        identity = try XCTUnwrap(PersistentIdentity.fetch(in: readContext, identityId: ownerId))
        XCTAssertNil(identity.ownedMainDpnsName)
        XCTAssertNil(identity.dpnsName)
    }

    /// A pick with no marketplace history that leaves the owned set keeps its
    /// row (not owned), so an identity left with no other names does not read
    /// as unhydrated and resurface the departed pick.
    func testDepartedCacheOnlyPickIsNotDisplayed() throws {
        applyIdentitySnapshot(id: ownerId, names: [("Alice", 10)])
        let context = ModelContext(container)
        XCTAssertTrue(PersistentIdentity.updateMainDpnsName(
            in: context, identityId: ownerId, mainDpnsName: "Alice"))
        try context.save()

        applyIdentitySnapshot(id: ownerId, names: [])

        let readContext = ModelContext(container)
        let identity = try XCTUnwrap(PersistentIdentity.fetch(in: readContext, identityId: ownerId))
        XCTAssertEqual(identity.mainDpnsName, "Alice")
        XCTAssertEqual(identity.dpnsNames.map(\.label), ["Alice"])
        XCTAssertEqual(identity.dpnsNames.first?.isOwned, false)
        XCTAssertNil(identity.ownedMainDpnsName)
        XCTAssertNotEqual(identity.displayName, "Alice")

        // Owning it again brings the pick back.
        applyIdentitySnapshot(id: ownerId, names: [("Alice", 10)])
        let reread = try XCTUnwrap(PersistentIdentity.fetch(in: ModelContext(container), identityId: ownerId))
        XCTAssertEqual(reread.ownedMainDpnsName, "Alice")
    }

    /// A departed pick with no display cache (`dpnsName` never set — a pick
    /// alone does not fill it) still falls back to another owned name.
    func testDisplayFallsBackToAnOwnedNameWhenTheDisplayCacheIsNil() throws {
        applyIdentitySnapshot(id: ownerId, names: [("Alice", 10), ("Bob", 20)])
        let context = ModelContext(container)
        XCTAssertTrue(PersistentIdentity.updateMainDpnsName(
            in: context, identityId: ownerId, mainDpnsName: "Alice"))
        try XCTUnwrap(PersistentIdentity.fetch(in: context, identityId: ownerId)).dpnsName = nil
        try context.save()

        applyIdentitySnapshot(id: ownerId, names: [("Bob", 20)])

        let identity = try XCTUnwrap(PersistentIdentity.fetch(in: ModelContext(container), identityId: ownerId))
        XCTAssertEqual(identity.mainDpnsName, "Alice")
        XCTAssertNil(identity.ownedMainDpnsName)
        XCTAssertEqual(identity.dpnsName, "Bob")
        XCTAssertEqual(identity.displayName, "Bob")
    }

    /// A pick stored without any name row (older data) is not trusted as
    /// unhydrated once an authoritative snapshot omits it, and comes back
    /// when a snapshot carries it again.
    func testScalarOnlyPickOmittedBySnapshotIsNotDisplayed() throws {
        let context = ModelContext(container)
        context.insert(PersistentIdentity(
            identityId: ownerId,
            isLocal: false,
            mainDpnsName: "Alice",
            network: .testnet
        ))
        try context.save()

        applyIdentitySnapshot(id: ownerId, names: [])
        var identity = try XCTUnwrap(PersistentIdentity.fetch(in: ModelContext(container), identityId: ownerId))
        XCTAssertEqual(identity.mainDpnsName, "Alice")
        XCTAssertNil(identity.ownedMainDpnsName)
        XCTAssertNotEqual(identity.displayName, "Alice")

        applyIdentitySnapshot(id: ownerId, names: [("Alice", 10)])
        identity = try XCTUnwrap(PersistentIdentity.fetch(in: ModelContext(container), identityId: ownerId))
        XCTAssertEqual(identity.ownedMainDpnsName, "Alice")
        XCTAssertEqual(identity.displayName, "Alice")
    }

    /// A snapshot that momentarily lacks the picked name (a cold start adds
    /// names before the in-memory list is whole) must not replace the pick.
    func testMainNamePickSurvivesAnIncompleteSnapshot() throws {
        applyIdentitySnapshot(id: ownerId, names: [("Alice", 10), ("Bob", 20)])
        let context = ModelContext(container)
        XCTAssertTrue(PersistentIdentity.updateMainDpnsName(
            in: context, identityId: ownerId, mainDpnsName: "Bob"))
        try context.save()

        applyIdentitySnapshot(id: ownerId, names: [("Alice", 30)])
        applyIdentitySnapshot(id: ownerId, names: [("Alice", 30), ("Bob", 30)])

        let readContext = ModelContext(container)
        let identity = try XCTUnwrap(PersistentIdentity.fetch(in: readContext, identityId: ownerId))
        XCTAssertEqual(identity.mainDpnsName, "Bob")
        XCTAssertEqual(identity.ownedMainDpnsName, "Bob")
        XCTAssertEqual(identity.displayName, "Bob")
    }

    func testMarketplaceCallbackCannotRestoreOwnershipRemovedByCanonicalSnapshot() throws {
        let documentId = Data(repeating: 0x35, count: 32).toBase58String()
        let context = ModelContext(container)
        let owner = PersistentIdentity(identityId: ownerId, isLocal: false, network: .testnet)
        let row = PersistentDPNSName(identity: owner, label: "Alice")
        row.documentIdBase58 = documentId
        context.insert(owner)
        context.insert(row)
        try context.save()

        // Commit the canonical ownership removal first. The later marketplace
        // update is a separate persistence round, matching independent sync
        // callbacks that can arrive well after the identity snapshot.
        applyIdentitySnapshot(id: ownerId, names: [])

        handler.beginChangeset(walletId: walletId)
        // Deliberately disagree with the already-committed identity snapshot.
        // Marketplace status is metadata relative to its tracked identity, not
        // ownership authority for the label cache, so this must not flip
        // `isOwned` back to true.
        XCTAssertTrue(handler.persistDpnsNameStates(
            walletId: walletId,
            upserts: [
                .init(
                    documentIdBase58: documentId,
                    walletIdentityId: ownerId,
                    label: "Alice",
                    normalizedLabel: PersistentDPNSName.normalize("Alice"),
                    normalizedParentDomainName: PersistentDPNSName.normalize("dash"),
                    priceCredits: nil,
                    statusRaw: 0,
                    counterpartyIdBase58: nil,
                    createdAtMs: nil,
                    updatedAtMs: nil,
                    transferredAtMs: nil,
                    lastSyncedAtMs: 200
                )
            ],
            removed: []
        ))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        let readContext = ModelContext(container)
        let persisted = try XCTUnwrap(
            readContext.fetch(FetchDescriptor<PersistentDPNSName>()).first
        )
        XCTAssertFalse(persisted.isOwned)
        XCTAssertEqual(persisted.saleStatusRaw, 0)
        XCTAssertTrue(try readContext.fetch(
            FetchDescriptor<PersistentDPNSName>(
                predicate: PersistentDPNSName.predicate(identityId: ownerId)
            )
        ).isEmpty)
    }

    func testSameWalletTransferRebindsSingleCanonicalRowToNewOwner() throws {
        let documentId = Data(repeating: 0x36, count: 32).toBase58String()
        let context = ModelContext(container)
        let oldOwner = PersistentIdentity(
            identityId: ownerId,
            isLocal: false,
            mainDpnsName: "Alice",
            network: .testnet
        )
        let nextOwner = PersistentIdentity(
            identityId: nextOwnerId,
            isLocal: false,
            network: .testnet
        )
        let row = PersistentDPNSName(identity: oldOwner, label: "Alice")
        row.isOwned = false
        row.documentIdBase58 = documentId
        row.saleStatusRaw = 2
        row.counterpartyIdBase58 = nextOwnerId.toBase58String()
        context.insert(oldOwner)
        context.insert(nextOwner)
        context.insert(row)
        try context.save()

        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(
            walletId: walletId,
            upserts: [identitySnapshot(id: nextOwnerId, names: [("Alice", 30)])],
            removed: []
        )
        XCTAssertTrue(handler.persistDpnsNameStates(
            walletId: walletId,
            upserts: [
                .init(
                    documentIdBase58: documentId,
                    walletIdentityId: nextOwnerId,
                    label: "Alice",
                    normalizedLabel: PersistentDPNSName.normalize("Alice"),
                    normalizedParentDomainName: PersistentDPNSName.normalize("dash"),
                    priceCredits: nil,
                    statusRaw: 0,
                    counterpartyIdBase58: nil,
                    createdAtMs: 21,
                    updatedAtMs: 22,
                    transferredAtMs: 23,
                    lastSyncedAtMs: 300
                )
            ],
            removed: []
        ))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        let readContext = ModelContext(container)
        let rows = try readContext.fetch(FetchDescriptor<PersistentDPNSName>())
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows.first?.identity.identityId, nextOwnerId)
        XCTAssertEqual(rows.first?.isOwned, true)
        XCTAssertEqual(rows.first?.acquiredAt, 30)
        XCTAssertEqual(rows.first?.saleStatusRaw, 0)
        XCTAssertNil(rows.first?.counterpartyIdBase58)
        XCTAssertTrue(try readContext.fetch(
            FetchDescriptor<PersistentDPNSName>(
                predicate: PersistentDPNSName.predicate(identityId: ownerId)
            )
        ).isEmpty)
        XCTAssertEqual(try readContext.fetch(
            FetchDescriptor<PersistentDPNSName>(
                predicate: PersistentDPNSName.predicate(identityId: nextOwnerId)
            )
        ).map(\.label), ["Alice"])

        // The rebind clears the old owner's pick of the transferred name.
        let previous = try XCTUnwrap(PersistentIdentity.fetch(in: readContext, identityId: ownerId))
        XCTAssertNil(previous.mainDpnsName)
        XCTAssertTrue(previous.dpnsNames.isEmpty)
        XCTAssertNil(previous.ownedMainDpnsName)
        XCTAssertNotEqual(previous.displayName, "Alice")

        // Even once the recipient's row is gone (its wallet deleted), the old
        // owner has no stale pick left to resurface.
        let deleteContext = ModelContext(container)
        for row in try deleteContext.fetch(FetchDescriptor<PersistentDPNSName>()) {
            deleteContext.delete(row)
        }
        try deleteContext.save()
        let afterDelete = try XCTUnwrap(
            PersistentIdentity.fetch(in: ModelContext(container), identityId: ownerId))
        XCTAssertNil(afterDelete.ownedMainDpnsName)
        XCTAssertNotEqual(afterDelete.displayName, "Alice")
    }

    func testMarketplaceRemovalClearsOnlyMarketplaceColumns() throws {
        let documentId = Data(repeating: 0x37, count: 32).toBase58String()
        let context = ModelContext(container)
        let owner = PersistentIdentity(identityId: ownerId, isLocal: false, network: .testnet)
        let row = PersistentDPNSName(identity: owner, label: "Alice", acquiredAt: 40)
        row.documentIdBase58 = documentId
        row.priceCredits = 12_345
        row.saleStatusRaw = 2
        row.counterpartyIdBase58 = nextOwnerId.toBase58String()
        row.documentCreatedAtMs = 41
        row.documentUpdatedAtMs = 42
        row.documentTransferredAtMs = 43
        row.marketplaceUpdatedAt = 400
        context.insert(owner)
        context.insert(row)
        try context.save()

        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistDpnsNameStates(
            walletId: walletId,
            upserts: [],
            removed: [documentId]
        ))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        let readContext = ModelContext(container)
        let rows = try readContext.fetch(FetchDescriptor<PersistentDPNSName>())
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows.first?.label, "Alice")
        XCTAssertEqual(rows.first?.acquiredAt, 40)
        XCTAssertEqual(rows.first?.isOwned, true)
        XCTAssertNil(rows.first?.documentIdBase58)
        XCTAssertNil(rows.first?.priceCredits)
        XCTAssertEqual(rows.first?.saleStatusRaw, 0)
        XCTAssertNil(rows.first?.counterpartyIdBase58)
        XCTAssertNil(rows.first?.documentCreatedAtMs)
        XCTAssertNil(rows.first?.documentUpdatedAtMs)
        XCTAssertNil(rows.first?.documentTransferredAtMs)
        XCTAssertEqual(rows.first?.marketplaceUpdatedAt, 0)
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
