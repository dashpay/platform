import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Removing an identity from the device ("Remove from Device" on the
/// Identities tab, and the orphan delete in the key health sheet).
///
/// Deleting a `PersistentIdentity` that still has DPNS names or DashPay rows
/// in one save makes SwiftData fatal: those children have a non-optional
/// inverse to the identity. These tests build an identity with every such
/// child and check that `PersistentIdentity.deleteChildrenFirst` removes it
/// without a crash, leaves a sibling identity alone, and that
/// `IdentitiesContentView.removeIdentityLocally` only touches the Keychain
/// once the rows are gone.
@MainActor
final class IdentityLocalRemovalTests: XCTestCase {

    private let doomedId = Data(repeating: 0x11, count: 32)
    private let siblingId = Data(repeating: 0x22, count: 32)

    func testDeleteChildrenFirstRemovesIdentityAndEveryChildRow() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let doomed = makeIdentity(doomedId, label: "doomed", keySeed: 0x30, in: context)
        try context.save()

        // Precondition: the inverse collections are populated, so the
        // single-save delete would have to null them.
        XCTAssertEqual(doomed.dpnsNames.count, 1)
        XCTAssertNotNil(doomed.dashpayProfile)
        XCTAssertEqual(doomed.contactRequests.count, 1)
        XCTAssertEqual(doomed.contactProfiles.count, 1)
        XCTAssertEqual(doomed.dashpayPayments.count, 1)
        XCTAssertEqual(doomed.dashpayIgnoredSenders.count, 1)
        XCTAssertEqual(doomed.publicKeys.count, 2)

        try PersistentIdentity.deleteChildrenFirst(doomed, in: context)

        try assertNoRows(in: container)
    }

    func testDeleteChildrenFirstKeepsSiblingIdentityRows() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let doomed = makeIdentity(doomedId, label: "doomed", keySeed: 0x30, in: context)
        _ = makeIdentity(siblingId, label: "sibling", keySeed: 0x40, in: context)
        try context.save()

        try PersistentIdentity.deleteChildrenFirst(doomed, in: context)

        let fresh = ModelContext(container)
        let identities = try fresh.fetch(FetchDescriptor<PersistentIdentity>())
        XCTAssertEqual(identities.map(\.identityId), [siblingId])
        let sibling = try XCTUnwrap(identities.first)
        XCTAssertEqual(sibling.dpnsNames.map(\.label), ["sibling"])
        XCTAssertNotNil(sibling.dashpayProfile)
        XCTAssertEqual(sibling.contactRequests.count, 1)
        XCTAssertEqual(sibling.contactProfiles.count, 1)
        XCTAssertEqual(sibling.dashpayPayments.count, 1)
        XCTAssertEqual(sibling.dashpayIgnoredSenders.count, 1)
        XCTAssertEqual(sibling.publicKeys.count, 2)

        XCTAssertEqual(try fresh.fetch(FetchDescriptor<PersistentDPNSName>()).count, 1)
        XCTAssertEqual(try fresh.fetch(FetchDescriptor<PersistentDashpayProfile>()).count, 1)
        XCTAssertEqual(try fresh.fetch(FetchDescriptor<PersistentDashpayContactRequest>()).count, 1)
        XCTAssertEqual(try fresh.fetch(FetchDescriptor<PersistentDashpayContactProfile>()).count, 1)
        XCTAssertEqual(try fresh.fetch(FetchDescriptor<PersistentDashpayPayment>()).count, 1)
        XCTAssertEqual(try fresh.fetch(FetchDescriptor<PersistentDashpayIgnoredSender>()).count, 1)
        XCTAssertEqual(try fresh.fetch(FetchDescriptor<PersistentPublicKey>()).count, 2)
    }

    func testRemoveIdentityLocallyDeletesKeychainItemsOnlyAfterTheRowsAreGone() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let doomed = makeIdentity(doomedId, label: "doomed", keySeed: 0x30, in: context)
        doomed.votingPrivateKeyIdentifier = "specialkey_voting"
        doomed.ownerPrivateKeyIdentifier = "specialkey_owner"
        try context.save()

        var specialKeyDeletes: [SpecialKeyType] = []
        var identifierDeletes: [String] = []
        var rowsLeftAtKeychainCall: [Int] = []
        let identityRowCount = { [doomedId] () -> Int in
            let target = doomedId
            let rows = try? ModelContext(container).fetch(FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == target }
            ))
            return rows?.count ?? -1
        }

        let keysLeft = try IdentitiesContentView.removeIdentityLocally(
            doomed,
            modelContext: context,
            deleteSpecialKey: { identityId, keyType in
                XCTAssertEqual(identityId, self.doomedId)
                rowsLeftAtKeychainCall.append(identityRowCount())
                specialKeyDeletes.append(keyType)
                return true
            },
            deleteKeyData: { identifier in
                rowsLeftAtKeychainCall.append(identityRowCount())
                identifierDeletes.append(identifier)
                return true
            }
        )

        XCTAssertEqual(keysLeft, [])
        XCTAssertEqual(specialKeyDeletes, [.voting, .owner, .payout])
        XCTAssertEqual(
            identifierDeletes,
            ["privkey_key_0", "privkey_key_1", "specialkey_voting", "specialkey_owner"]
        )
        XCTAssertEqual(rowsLeftAtKeychainCall, Array(repeating: 0, count: 7))
        try assertNoRows(in: container)
    }

    func testRemoveIdentityLocallyReportsKeychainItemsItCouldNotDelete() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let doomed = makeIdentity(doomedId, label: "doomed", keySeed: 0x30, in: context)
        doomed.votingPrivateKeyIdentifier = "specialkey_voting"
        try context.save()

        let keysLeft = try IdentitiesContentView.removeIdentityLocally(
            doomed,
            modelContext: context,
            deleteSpecialKey: { _, keyType in keyType != .voting },
            deleteKeyData: { identifier in
                identifier != "privkey_key_1" && identifier != "specialkey_voting"
            }
        )

        // The refused voting slot and its row identifier are one key, named once.
        XCTAssertEqual(keysLeft, ["voting key", "key 1"])
        try assertNoRows(in: container)
    }

    // MARK: - Helpers

    /// Inserts an identity with a DPNS name, a DashPay profile, a contact
    /// request, a contact profile, a payment, an ignored sender and two
    /// public keys whose Keychain identifiers are `privkey_key_<keyId>`.
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

        for keyId: Int32 in [0, 1] {
            let key = PersistentPublicKey(
                keyId: keyId,
                purpose: .authentication,
                securityLevel: keyId == 0 ? .master : .high,
                keyType: .ecdsaSecp256k1,
                publicKeyData: Data(repeating: keySeed &+ UInt8(keyId), count: 33),
                identityId: identityId.toHexString()
            )
            key.privateKeyKeychainIdentifier = "privkey_key_\(keyId)"
            context.insert(key)
            identity.publicKeys.append(key)
        }
        return identity
    }

    /// No identity and none of its child rows are left in the store.
    private func assertNoRows(
        in container: ModelContainer,
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws {
        let fresh = ModelContext(container)
        XCTAssertEqual(try fresh.fetchCount(FetchDescriptor<PersistentIdentity>()), 0, file: file, line: line)
        XCTAssertEqual(try fresh.fetchCount(FetchDescriptor<PersistentDPNSName>()), 0, file: file, line: line)
        XCTAssertEqual(try fresh.fetchCount(FetchDescriptor<PersistentDashpayProfile>()), 0, file: file, line: line)
        XCTAssertEqual(
            try fresh.fetchCount(FetchDescriptor<PersistentDashpayContactRequest>()), 0, file: file, line: line
        )
        XCTAssertEqual(
            try fresh.fetchCount(FetchDescriptor<PersistentDashpayContactProfile>()), 0, file: file, line: line
        )
        XCTAssertEqual(try fresh.fetchCount(FetchDescriptor<PersistentDashpayPayment>()), 0, file: file, line: line)
        XCTAssertEqual(
            try fresh.fetchCount(FetchDescriptor<PersistentDashpayIgnoredSender>()), 0, file: file, line: line
        )
        XCTAssertEqual(try fresh.fetchCount(FetchDescriptor<PersistentPublicKey>()), 0, file: file, line: line)
    }
}
