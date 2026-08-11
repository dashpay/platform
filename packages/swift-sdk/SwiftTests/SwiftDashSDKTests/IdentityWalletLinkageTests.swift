import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

// MARK: - `PersistentIdentity.wallet` linkage on the persister path
//
// The `wallet` relationship is the single source of truth for
// identity ownership (`isWalletOwned` / `walletOwnedIdentitiesPredicate`
// are views over it). A stored `isLocal` flag used to shadow it and
// drifted — nothing ever wrote `true`, so a wallet's own identity
// persisted as "not local" (observed on a mainnet device 2026-08-12,
// where it hid the identity-key refresh affordances in dashwallet-ios).
// The flag is gone; these tests pin the linkage itself: who gets
// attached to the scope wallet, who must not, and how a mislinked row
// heals on re-emit.

final class IdentityWalletLinkageTests: XCTestCase {

    private var container: ModelContainer!
    private var handler: PlatformWalletPersistenceHandler!

    private let walletId = Data(repeating: 0xAA, count: 32)
    private let ownIdentityId = Data(repeating: 0x01, count: 32)
    private let observedIdentityId = Data(repeating: 0x02, count: 32)

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

    // MARK: Fixtures

    private func insertWalletRow() throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
    }

    private func makeEntry(
        identityId: Data,
        identityIndex: UInt32?,
        walletId: Data?
    ) -> PlatformWalletPersistenceHandler.IdentityEntrySnapshot {
        .init(
            identityId: identityId,
            balance: 100,
            revision: 1,
            identityIndex: identityIndex,
            label: nil,
            status: 0,
            walletId: walletId,
            dpnsNames: [],
            dashpayProfile: nil,
            contactProfiles: []
        )
    }

    /// Apply one identity persister round the way the FFI does —
    /// bracketed by `beginChangeset` / `endChangeset(success: true)`.
    private func applyIdentities(
        _ upserts: [PlatformWalletPersistenceHandler.IdentityEntrySnapshot]
    ) {
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(
            walletId: walletId,
            upserts: upserts,
            removed: []
        )
        handler.endChangeset(walletId: walletId, success: true)
    }

    private func fetchIdentity(_ identityId: Data) throws -> PersistentIdentity? {
        let context = ModelContext(container)
        return try context.fetch(
            FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
        ).first
    }

    // MARK: Linkage derivation

    /// A wallet-owned entry (Rust stamps `wallet_id` on every
    /// `add_identity` / load / discovery emission) links the wallet
    /// relationship, which `isWalletOwned` reflects.
    func testWalletOwnedEntryLinksWallet() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId)
        ])

        let row = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(row.wallet?.walletId, walletId)
        XCTAssertTrue(row.isWalletOwned)
    }

    /// A wallet-derived entry that arrives without its own
    /// `wallet_id` but WITH an identity index (create-flow corner
    /// case) falls back to the changeset's scope wallet.
    func testIndexOnlyEntryFallsBackToScopeWallet() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: nil)
        ])

        let row = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(row.wallet?.walletId, walletId)
    }

    /// An out-of-wallet (observed) entry — `identity_index == nil`
    /// and `wallet_id == nil`, the `add_out_of_wallet_identity`
    /// shape — must NOT be linked to the scope wallet by the
    /// fallback.
    func testObservedEntryDoesNotLinkScopeWallet() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil)
        ])

        let row = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertNil(
            row.wallet,
            "observed identities must not inherit the changeset's scope wallet"
        )
        XCTAssertFalse(row.isWalletOwned)
    }

    /// Re-emitting an entry heals linkage in place: a row mislinked
    /// by the old unconditional fallback is unlinked when its entry
    /// re-arrives as out-of-wallet, and a wallet-owned row keeps its
    /// linkage.
    func testUpsertRestampsLinkage() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let ownRowSeed = PersistentIdentity(
            identityId: ownIdentityId,
            network: .testnet
        )
        ownRowSeed.wallet = wallet
        context.insert(ownRowSeed)
        let mislinkedObserved = PersistentIdentity(
            identityId: observedIdentityId,
            network: .testnet
        )
        mislinkedObserved.wallet = wallet
        context.insert(mislinkedObserved)
        try context.save()

        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId),
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil),
        ])

        let ownRow = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(ownRow.wallet?.walletId, walletId)

        let observedRow = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertNil(observedRow.wallet)
        XCTAssertFalse(observedRow.isWalletOwned)
    }
}
