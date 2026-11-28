import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// An identity the Rust wallet removed
/// (`platform_wallet_manager_remove_identity_from_device`) reaches the
/// persistence handler as a `removed` id inside a changeset round.
///
/// Deleting a `PersistentIdentity` that still has DPNS names or DashPay rows
/// in one save makes SwiftData fatal: those children have a non-optional
/// inverse to the identity. A round has one save, so the handler deletes the
/// identity, children first, right after the round commits. These tests
/// build an identity with every such child and check that the round removes
/// it without a crash, leaves a sibling identity alone, and that a failed
/// round removes nothing.
final class IdentityRemovalPersistenceTests: XCTestCase {

    private var container: ModelContainer!
    private var handler: PlatformWalletPersistenceHandler!

    private let walletId = Data(repeating: 0xAA, count: 32)
    private let doomedId = Data(repeating: 0x11, count: 32)
    private let siblingId = Data(repeating: 0x22, count: 32)

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

    func testRemovedIdentityWithChildrenIsDeletedByACommittedRound() throws {
        let context = ModelContext(container)
        makeIdentity(doomedId, label: "doomed", keySeed: 0x30, in: context)
        makeIdentity(siblingId, label: "sibling", keySeed: 0x40, in: context)
        try context.save()

        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistIdentities(walletId: walletId, upserts: [], removed: [doomedId]))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        let fresh = ModelContext(container)
        let identities = try fresh.fetch(FetchDescriptor<PersistentIdentity>())
        XCTAssertEqual(identities.map(\.identityId), [siblingId])
        let sibling = try XCTUnwrap(identities.first)
        XCTAssertEqual(sibling.dpnsNames.map(\.label), ["sibling"])
        XCTAssertNotNil(sibling.dashpayProfile)
        try assertChildRowCounts(1, publicKeys: 2, in: fresh)
        let addresses = try fresh.fetch(FetchDescriptor<PersistentDashpayPaymentAddresses>())
        XCTAssertEqual(addresses.map(\.ownerIdentityId), [siblingId])
    }

    func testRemovedIdentityIsKeptWholeByAFailedRound() throws {
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        makeIdentity(doomedId, label: "doomed", keySeed: 0x30, in: context).wallet = wallet
        try context.save()

        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistIdentities(walletId: walletId, upserts: [], removed: [doomedId]))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))

        // A later round that commits must not delete the row the failed
        // round parked.
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        let fresh = ModelContext(container)
        let identities = try fresh.fetch(FetchDescriptor<PersistentIdentity>())
        XCTAssertEqual(identities.map(\.identityId), [doomedId])
        // The round unlinks a removed identity from its wallet, which is what
        // keeps a leftover row from being restored; a failed round must put
        // the link back.
        XCTAssertEqual(identities.first?.wallet?.walletId, walletId)
        try assertChildRowCounts(1, publicKeys: 2, in: fresh)
        XCTAssertEqual(try fresh.fetchCount(FetchDescriptor<PersistentDashpayPaymentAddresses>()), 1)
    }

    func testRemovingAnIdentityWithNoRowCommitsTheRound() throws {
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistIdentities(walletId: walletId, upserts: [], removed: [doomedId]))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    func testRemoveIdentityFromLiveWalletsRefusesAnUnconfiguredManager() async throws {
        let manager = await PlatformWalletManager()
        do {
            _ = try await manager.removeIdentityFromLiveWallets(identityId: doomedId)
            XCTFail("expected invalidHandle, call succeeded")
        } catch PlatformWalletError.invalidHandle {
            // Expected: no native manager to ask.
        } catch {
            XCTFail("expected invalidHandle, got \(error)")
        }
    }

    /// While an async native operation is admitted (a wallet restore, for
    /// one) Rust may not hold the identity yet and would answer "not
    /// managed"; the host would then delete rows and keys the restore brings
    /// back. The wrapper refuses before it asks.
    @MainActor
    func testRemoveIdentityFromLiveWalletsRefusesWhileAnAsyncNativeOpIsAdmitted() throws {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let manager = PlatformWalletManager.makeForTesting(
            handle: 97,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
                dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok))
        try manager.admitNativeOp("loadFromPersistor")
        defer { manager.finishNativeOp() }

        XCTAssertThrowsError(try manager.removeIdentityFromLiveWallets(identityId: doomedId)) { error in
            guard case PlatformWalletError.invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        }
    }

    func testDeleteChildrenFirstByIdReportsWhetherThereWasARow() throws {
        let context = ModelContext(container)
        makeIdentity(doomedId, label: "doomed", keySeed: 0x30, in: context)
        try context.save()

        XCTAssertTrue(try PersistentIdentity.deleteChildrenFirst(identityId: doomedId, in: context))
        XCTAssertFalse(try PersistentIdentity.deleteChildrenFirst(identityId: doomedId, in: context))
        try assertChildRowCounts(0, publicKeys: 0, in: ModelContext(container))
    }

    // MARK: - Helpers

    /// Inserts an identity with a DPNS name, a DashPay profile, a contact
    /// request, a contact profile, a payment, an ignored sender, a
    /// payment-address row and two public keys.
    @discardableResult
    private func makeIdentity(
        _ identityId: Data,
        label: String,
        keySeed: UInt8,
        in context: ModelContext
    ) -> PersistentIdentity {
        let identity = PersistentIdentity(identityId: identityId, network: .testnet)
        context.insert(identity)

        context.insert(PersistentDPNSName(identity: identity, label: label))
        context.insert(PersistentDashpayProfile(identity: identity, displayName: label))
        let contactId = Data(repeating: keySeed &+ 1, count: 32)
        context.insert(PersistentDashpayContactRequest(
            owner: identity,
            contactIdentityId: contactId,
            isOutgoing: true,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: Data(repeating: keySeed, count: 96),
            coreHeightCreatedAt: 1,
            createdAtMillis: 1
        ))
        context.insert(PersistentDashpayContactProfile(
            owner: identity,
            contactIdentityId: contactId,
            checkedAtMs: 1,
            displayName: "contact of \(label)"
        ))
        context.insert(PersistentDashpayPayment(
            owner: identity,
            counterpartyIdentityId: contactId,
            amountDuffs: 1_000,
            direction: .sent,
            status: .confirmed,
            txid: String(repeating: "a", count: 64)
        ))
        context.insert(PersistentDashpayIgnoredSender(
            owner: identity,
            ignoredSenderId: Data(repeating: keySeed &+ 2, count: 32)
        ))
        context.insert(PersistentDashpayPaymentAddresses(
            networkRaw: identity.networkRaw, ownerIdentityId: identityId, profileIdentityId: identityId,
            shieldedAddress: Data(repeating: 3, count: 43)))

        for keyId: Int32 in [0, 1] {
            let key = PersistentPublicKey(
                keyId: keyId,
                purpose: .authentication,
                securityLevel: keyId == 0 ? .master : .high,
                keyType: .ecdsaSecp256k1,
                publicKeyData: Data(repeating: keySeed &+ UInt8(keyId), count: 33),
                identityId: identityId.toHexString()
            )
            context.insert(key)
            identity.publicKeys.append(key)
        }
        return identity
    }

    /// Every child table with a non-optional inverse holds `count` rows.
    private func assertChildRowCounts(
        _ count: Int,
        publicKeys: Int,
        in context: ModelContext,
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws {
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentDPNSName>()), count, file: file, line: line)
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentDashpayProfile>()), count, file: file, line: line)
        XCTAssertEqual(
            try context.fetchCount(FetchDescriptor<PersistentDashpayContactRequest>()), count, file: file, line: line
        )
        XCTAssertEqual(
            try context.fetchCount(FetchDescriptor<PersistentDashpayContactProfile>()), count, file: file, line: line
        )
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentDashpayPayment>()), count, file: file, line: line)
        XCTAssertEqual(
            try context.fetchCount(FetchDescriptor<PersistentDashpayIgnoredSender>()), count, file: file, line: line
        )
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentPublicKey>()), publicKeys, file: file, line: line)
    }
}
