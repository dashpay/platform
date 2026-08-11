import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

// MARK: - `PersistentIdentity.isLocal` promotion invariants
//
// `isLocal` = the user can act as this identity from this device.
// Wallet linkage implies local (`wallet != nil` ⟹ `isLocal`), but not
// the converse — an identity can be local via imported key material
// (masternode voting/owner/payout keys, pasted user keys) with no
// wallet row. Writers therefore PROMOTE and never demote.
//
// Historically the persister wrote a constant `false`, so a wallet's
// own identity (correct `wallet` relationship, identityIndex 0)
// persisted as `isLocal == false` — field-observed on a mainnet
// device, where it hid the identity-key refresh affordances in
// dashwallet-ios. These tests pin the promotion on the upsert path,
// the gating of the scope-wallet fallback, the preservation of
// walletless-local rows, and the load-time heal.

final class IdentityIsLocalPersistenceTests: XCTestCase {

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
        // The real probe reads the Keychain-backed WalletStorage,
        // which the simulator test host can't reach. Fixture wallets
        // hold signing material unless a test says otherwise.
        handler.walletSigningMaterialProbe = { _ in true }
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

    /// A persisted key row. With `walletId` set this is the derivation
    /// stamp `persistIdentityKeys` writes for DIP-9-derived keys — the
    /// only accepted ownership evidence. With `walletId == nil` it
    /// models an UNSTAMPED row (pre-breadcrumb era, or a by-id
    /// import / key refresh on an observed identity), which is
    /// deliberately NOT evidence.
    private func attachKeyRow(
        to identity: PersistentIdentity,
        stampedWalletId: Data?
    ) {
        let key = PersistentPublicKey(
            keyId: Int32(identity.publicKeys.count),
            purpose: .authentication,
            securityLevel: .master,
            keyType: .ecdsaSecp256k1,
            publicKeyData: Data(repeating: 0x11, count: 33),
            identityId: identity.identityIdString
        )
        key.walletId = stampedWalletId
        identity.publicKeys.append(key)
    }

    // MARK: Upsert derivation

    /// A wallet-owned entry (Rust stamps `wallet_id` on every
    /// `add_identity` / load / discovery emission) links the wallet
    /// relationship AND persists `isLocal == true`.
    func testWalletOwnedEntryPersistsIsLocalTrue() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId)
        ])

        let row = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(row.wallet?.walletId, walletId)
        XCTAssertTrue(
            row.isLocal,
            "the wallet's own identity must persist isLocal == true (the mainnet field bug)"
        )
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
        XCTAssertTrue(row.isLocal)
    }

    /// An out-of-wallet (observed) entry — `identity_index == nil`
    /// and `wallet_id == nil`, the `add_out_of_wallet_identity`
    /// shape — must NOT be linked to the scope wallet by the
    /// fallback, and stays `isLocal == false`.
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
        XCTAssertFalse(row.isLocal)
    }

    /// Re-emitting an entry heals a stale row in place: a
    /// wallet-linked row persisted with `isLocal == false` (written
    /// before the flag tracked linkage) flips to `true`, and a row
    /// mislinked by the old unconditional fallback is unlinked when
    /// its entry re-arrives as out-of-wallet. The unlink must NOT
    /// demote `isLocal` — the flag may be `true` because the user
    /// imported key material (masternode voting keys, pasted user
    /// keys), which the persister cannot see.
    func testUpsertPromotesStaleRowsAndPreservesWalletlessLocal() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let staleOwn = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: false,
            network: .testnet
        )
        staleOwn.wallet = wallet
        context.insert(staleOwn)
        // Mislinked by the old fallback AND marked local (as an
        // imported-keys identity would be).
        let mislinkedLocal = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        mislinkedLocal.wallet = wallet
        context.insert(mislinkedLocal)
        try context.save()

        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId),
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil),
        ])

        let ownRow = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertTrue(ownRow.isLocal)
        XCTAssertEqual(ownRow.wallet?.walletId, walletId)

        let observedRow = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertNil(observedRow.wallet, "the wallet mislink is cleared")
        XCTAssertTrue(
            observedRow.isLocal,
            "losing the wallet link must not demote isLocal — imported-key identities are local without a wallet"
        )
    }

    /// An entry that DECLARES an owner whose wallet row fails to
    /// resolve keeps the existing link ONLY when that link already
    /// points at the declared owner (the network-scoped fetch-miss
    /// case — here the owner's row exists on another network). A
    /// matching link surviving the miss is correct; anything else
    /// would codify a contradiction.
    func testDeclaredOwnerFetchMissKeepsMatchingLink() throws {
        // Owner wallet row exists but on .mainnet — the .testnet
        // scoped `fetchWalletForLink` misses it, while the
        // relationship can still point at the row.
        let otherWalletId = Data(repeating: 0xCC, count: 32)
        let context = ModelContext(container)
        let mainnetWallet = PersistentWallet(walletId: otherWalletId, network: .mainnet)
        context.insert(mainnetWallet)
        let row = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: true,
            network: .testnet
        )
        row.wallet = mainnetWallet
        context.insert(row)
        try context.save()

        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: otherWalletId)
        ])

        let fetched = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(
            fetched.wallet?.walletId,
            otherWalletId,
            "a link matching the declared owner survives a network-scoped fetch miss"
        )
    }

    /// When the declared owner doesn't resolve AND the existing link
    /// points at a DIFFERENT wallet, the link is cleared — the
    /// entry's declaration is Rust's current truth, and keeping the
    /// contradicting relationship would preserve exactly the kind of
    /// stale ownership this PR unwinds.
    func testDeclaredOwnerMismatchClearsContradictingLink() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let scopeWallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let row = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: true,
            network: .testnet
        )
        row.wallet = scopeWallet
        context.insert(row)
        try context.save()

        // Entry declares an owner with no wallet row anywhere; the
        // existing link points at the scope wallet instead.
        let unresolvedOwnerId = Data(repeating: 0xDD, count: 32)
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: unresolvedOwnerId)
        ])

        let fetched = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertNil(
            fetched.wallet,
            "a link contradicting the entry's declared owner is cleared"
        )
        XCTAssertTrue(fetched.isLocal, "clearing the link never demotes isLocal")
    }

    /// Wallet linkage is not signing capability: a watch-only wallet
    /// (no mnemonic — the probe reports no signing material) links
    /// its identities but must NOT promote them to "Local", which
    /// gates mutation controls the wallet cannot sign for.
    func testWatchOnlyWalletLinksWithoutPromotingIsLocal() throws {
        try insertWalletRow()
        handler.walletSigningMaterialProbe = { _ in false }
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId)
        ])

        let row = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(row.wallet?.walletId, walletId, "ownership is still recorded")
        XCTAssertFalse(
            row.isLocal,
            "a wallet without signing material must not present its identities as Local"
        )
    }

    /// "Out-of-wallet" is relative to the EMITTING manager: wallet A
    /// can resolve wallet B's identity via
    /// `load_identity_by_dpns_name`, whose `add_out_of_wallet_identity`
    /// emits the nil/nil shape from A's manager. The globally-keyed
    /// row must keep wallet B's valid relationship — the unlink only
    /// applies to a link to the SCOPE wallet (the one the old
    /// fallback could have fabricated).
    func testObservedEntryPreservesAnotherWalletsLinkage() throws {
        try insertWalletRow()
        let otherWalletId = Data(repeating: 0xBB, count: 32)
        let context = ModelContext(container)
        let otherWallet = PersistentWallet(walletId: otherWalletId, network: .testnet)
        context.insert(otherWallet)
        let ownedByOther = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        ownedByOther.wallet = otherWallet
        context.insert(ownedByOther)
        try context.save()

        // Scope wallet A emits B's identity as observed.
        applyIdentities([
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil)
        ])

        let row = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertEqual(
            row.wallet?.walletId,
            otherWalletId,
            "an observed emission from wallet A must not strip wallet B's ownership"
        )
        XCTAssertTrue(row.isLocal)
    }

    // MARK: Restore-slice collision guard

    /// Only rows carrying THIS wallet's derivation stamp reach the
    /// restore slice: a legacy mislink at the placeholder index `0`
    /// (no stamp) is quarantined and cannot displace the genuine
    /// index-0 identity in Rust's per-index BTreeMap, and every
    /// stamped row at a unique index passes through.
    func testRestorableIdentitiesPrefersKeyEvidenceOnIndexCollision() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )

        // Genuine index-0 identity with persister-written evidence.
        // Its id sorts AFTER the mislink's so ordering alone would
        // pick the wrong row.
        let genuine = PersistentIdentity(
            identityId: Data(repeating: 0x0F, count: 32),
            isLocal: true,
            network: .testnet
        )
        genuine.wallet = wallet
        context.insert(genuine)
        attachKeyRow(to: genuine, stampedWalletId: walletId)

        // Legacy mislink: linked to the same wallet, placeholder
        // index 0, no key evidence, lexicographically-smaller id.
        let mislinked = PersistentIdentity(
            identityId: Data(repeating: 0x01, count: 32),
            isLocal: false,
            network: .testnet
        )
        mislinked.wallet = wallet
        context.insert(mislinked)

        // A stamped identity at a unique index passes through.
        let second = PersistentIdentity(
            identityId: Data(repeating: 0x02, count: 32),
            isLocal: true,
            network: .testnet,
            identityIndex: 1
        )
        second.wallet = wallet
        context.insert(second)
        attachKeyRow(to: second, stampedWalletId: walletId)
        try context.save()

        let slice = PlatformWalletPersistenceHandler.restorableIdentities(
            [mislinked, second, genuine],
            walletId: walletId
        )

        XCTAssertEqual(slice.map(\.identityIndex), [0, 1])
        XCTAssertEqual(
            slice[0].identityId,
            genuine.identityId,
            "the key-less mislink is quarantined; the stamped index-0 row survives"
        )
        XCTAssertEqual(slice[1].identityId, second.identityId)
    }

    /// A SOLE mislinked row with no genuine competitor must not be
    /// restored as wallet-owned either: Rust would stamp it
    /// `wallet_id = Some(wallet)` and the corruption would become
    /// durable. Crucially this holds even when the row CARRIES key
    /// rows — an unstamped one (by-id import / key refresh persist
    /// those for observed identities too) and one stamped for a
    /// DIFFERENT wallet — neither of which is evidence for THIS
    /// wallet. Quarantine leaves the store row untouched.
    func testRestorableIdentitiesQuarantinesSoleMislinkWithForeignKeys() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let mislinked = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: false,
            network: .testnet
        )
        mislinked.wallet = wallet
        context.insert(mislinked)
        attachKeyRow(to: mislinked, stampedWalletId: nil)
        attachKeyRow(to: mislinked, stampedWalletId: Data(repeating: 0xBB, count: 32))
        try context.save()

        let slice = PlatformWalletPersistenceHandler.restorableIdentities(
            [mislinked],
            walletId: walletId
        )

        XCTAssertTrue(
            slice.isEmpty,
            "unstamped or foreign-stamped keys must not qualify a sole mislink for wallet-owned restore"
        )
    }

    /// Refresh-then-restore regression: refresh flows rebuild the
    /// public-key rows from freshly fetched (unstamped) data, and
    /// MUST carry the wallet-derivation breadcrumb forward — dropping
    /// it would strip a genuine identity of the evidence
    /// `restorableIdentities` requires and quarantine it from the
    /// next Rust restore. Provenance transfers only on a full
    /// (`keyId`, `publicKeyData`) match; different key material must
    /// not inherit a breadcrumb.
    func testKeyReplacePreservesProvenanceAndSurvivesRestore() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let identity = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: true,
            network: .testnet
        )
        identity.wallet = wallet
        context.insert(identity)
        attachKeyRow(to: identity, stampedWalletId: walletId)
        identity.publicKeys[0].identityDerivationPath = "m/9'/1'/5'/0'/0'"
        identity.publicKeys[0].privateKeyKeychainIdentifier = "kc-0"
        try context.save()

        // A refresh delivers the same key unstamped (network data
        // carries no local provenance) plus a brand-new key.
        let refreshedSame = PersistentPublicKey(
            keyId: 0,
            purpose: .authentication,
            securityLevel: .master,
            keyType: .ecdsaSecp256k1,
            publicKeyData: Data(repeating: 0x11, count: 33),
            identityId: identity.identityIdString
        )
        let brandNew = PersistentPublicKey(
            keyId: 1,
            purpose: .authentication,
            securityLevel: .critical,
            keyType: .ecdsaSecp256k1,
            publicKeyData: Data(repeating: 0x22, count: 33),
            identityId: identity.identityIdString
        )
        identity.replacePublicKeysPreservingProvenance(
            with: [refreshedSame, brandNew]
        )
        try context.save()

        let carried = try XCTUnwrap(
            identity.publicKeys.first { $0.keyId == 0 }
        )
        XCTAssertEqual(carried.walletId, walletId, "the derivation stamp survives the refresh")
        XCTAssertEqual(carried.identityDerivationPath, "m/9'/1'/5'/0'/0'")
        XCTAssertEqual(carried.privateKeyKeychainIdentifier, "kc-0")
        let fresh = try XCTUnwrap(
            identity.publicKeys.first { $0.keyId == 1 }
        )
        XCTAssertNil(fresh.walletId, "new key material must not inherit a breadcrumb")

        let slice = PlatformWalletPersistenceHandler.restorableIdentities(
            [identity],
            walletId: walletId
        )
        XCTAssertEqual(
            slice.map(\.identityId),
            [identity.identityId],
            "a refreshed genuine identity keeps restoring"
        )
    }

    // MARK: Load-time heal

    /// `loadWalletList()` promotes wallet-linked stale-`false` rows
    /// to `isLocal == true` — but only with wallet-derivation key
    /// evidence, so a legacy mislink (linked, no key rows) is never
    /// promoted to "signable". UPWARD ONLY: a walletless row carrying
    /// `true` (imported-key identity — masternode voting keys, pasted
    /// user keys) is left alone.
    func testLoadWalletListPromotesOnlyEvidencedRowsAndNeverDemotes() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let staleOwn = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: false,
            network: .testnet
        )
        staleOwn.wallet = wallet
        context.insert(staleOwn)
        attachKeyRow(to: staleOwn, stampedWalletId: walletId)
        let walletlessLocal = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        context.insert(walletlessLocal)
        let mislinkedNoEvidence = PersistentIdentity(
            identityId: Data(repeating: 0x03, count: 32),
            isLocal: false,
            network: .testnet
        )
        mislinkedNoEvidence.wallet = wallet
        context.insert(mislinkedNoEvidence)
        // Linked row with only an UNSTAMPED key: could be a
        // pre-breadcrumb genuine row OR a mislinked observed identity
        // whose keys came from a by-id import / refresh — the heal
        // cannot tell them apart, so it must defer (the breadcrumb
        // backfill / next owned re-emit stamps the genuine case).
        let unstamped = PersistentIdentity(
            identityId: Data(repeating: 0x04, count: 32),
            isLocal: false,
            network: .testnet,
            identityIndex: 1
        )
        unstamped.wallet = wallet
        context.insert(unstamped)
        attachKeyRow(to: unstamped, stampedWalletId: nil)
        try context.save()

        // The fixture wallet has no restorable accounts, so the
        // returned list is empty — the heal runs regardless.
        let result = handler.loadWalletList()
        XCTAssertNil(result.entries)
        XCTAssertFalse(result.errored)

        let ownRow = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertTrue(
            ownRow.isLocal,
            "wallet-linked row with key evidence heals to isLocal == true"
        )

        let walletlessRow = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertTrue(
            walletlessRow.isLocal,
            "walletless local row (imported keys) must survive the heal"
        )

        let mislinkedRow = try XCTUnwrap(
            try fetchIdentity(Data(repeating: 0x03, count: 32))
        )
        XCTAssertFalse(
            mislinkedRow.isLocal,
            "a key-less legacy mislink must not be promoted to signable"
        )

        let unstampedRow = try XCTUnwrap(
            try fetchIdentity(Data(repeating: 0x04, count: 32))
        )
        XCTAssertFalse(
            unstampedRow.isLocal,
            "unstamped keys are not ownership evidence — promotion defers to the backfill / next owned re-emit"
        )
    }
}
