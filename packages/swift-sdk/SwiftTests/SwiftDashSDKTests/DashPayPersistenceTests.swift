import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

// MARK: - DashPay persister-bridge mapping
//
// These tests feed synthetic persister payloads — the same shapes the
// Rust `on_persist_contacts_fn` callback delivers — through
// `PlatformWalletPersistenceHandler` into an in-memory ModelContainer
// and assert the SwiftData effects. No network, no FFI handles: the
// seam under test is the Swift side of the persister bridge.

final class DashPayContactPersistenceTests: XCTestCase {

    private var container: ModelContainer!
    private var handler: PlatformWalletPersistenceHandler!

    // Fixed fixture ids.
    private let walletId = Data(repeating: 0xAA, count: 32)
    private let ownerId = Data(repeating: 0x01, count: 32)
    private let contactId = Data(repeating: 0x02, count: 32)
    private let otherSenderId = Data(repeating: 0x03, count: 32)

    override func setUpWithError() throws {
        try super.setUpWithError()
        container = try DashModelContainer.createInMemory()
        handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet
        )
        // The contact persister requires the owner identity row to
        // already exist (non-optional `owner` relationship + the
        // `networkRaw` read come off it) — mirror the production
        // ordering where identities apply before contacts.
        let context = ModelContext(container)
        let owner = PersistentIdentity(
            identityId: ownerId,
            isLocal: false,
            network: .testnet
        )
        context.insert(owner)
        try context.save()
    }

    override func tearDown() {
        handler = nil
        container = nil
        super.tearDown()
    }

    // MARK: Fixtures

    private func makeSnapshot(
        contactId: Data? = nil,
        isOutgoing: Bool,
        accountReference: UInt32 = 0,
        paymentChannelBroken: Bool = false,
        encryptedPublicKey: Data = Data(repeating: 0x11, count: 96),
        encryptedAccountLabel: Data? = nil,
        contactAcceptedAccounts: [UInt32] = []
    ) -> PlatformWalletPersistenceHandler.ContactRequestSnapshot {
        .init(
            ownerIdentityId: ownerId,
            contactIdentityId: contactId ?? self.contactId,
            isOutgoing: isOutgoing,
            senderKeyIndex: 2,
            recipientKeyIndex: 3,
            accountReference: accountReference,
            encryptedPublicKey: encryptedPublicKey,
            encryptedAccountLabel: encryptedAccountLabel,
            autoAcceptProof: nil,
            coreHeightCreatedAt: 1_234_567,
            createdAtMillis: 1_700_000_000_000,
            paymentChannelBroken: paymentChannelBroken,
            contactAlias: nil,
            contactNote: nil,
            contactHidden: false,
            contactAccountLabel: nil,
            contactAcceptedAccounts: contactAcceptedAccounts
        )
    }

    /// Apply one persister round the way the FFI does: bracketed by
    /// `beginChangeset` / `endChangeset(success: true)` so the writes
    /// land in the store atomically.
    private func applyContacts(
        upserts: [PlatformWalletPersistenceHandler.ContactRequestSnapshot] = [],
        removedSent: [PlatformWalletPersistenceHandler.ContactRequestRemovalSnapshot] = [],
        removedIncoming: [PlatformWalletPersistenceHandler.ContactRequestRemovalSnapshot] = [],
        ignored: [PlatformWalletPersistenceHandler.ContactIgnoredSenderSnapshot] = []
    ) {
        handler.beginChangeset(walletId: walletId)
        handler.persistContacts(
            walletId: walletId,
            upserts: upserts,
            removedSent: removedSent,
            removedIncoming: removedIncoming,
            ignored: ignored
        )
        handler.endChangeset(walletId: walletId, success: true)
    }

    /// Read every contact-request row back through a fresh context so
    /// the assertions only see committed state.
    private func fetchContactRows() throws -> [PersistentDashpayContactRequest] {
        let context = ModelContext(container)
        return try context.fetch(
            FetchDescriptor<PersistentDashpayContactRequest>()
        )
    }

    private func fetchContactProfileRows() throws -> [PersistentDashpayContactProfile] {
        let context = ModelContext(container)
        return try context.fetch(
            FetchDescriptor<PersistentDashpayContactProfile>()
        )
    }

    /// Apply one identity persister round carrying only the given contact
    /// profiles for the fixture owner — the seam `upsertDashpayContactProfiles`
    /// runs under.
    private func applyContactProfiles(
        _ profiles: [PlatformWalletPersistenceHandler.ContactProfileSnapshot]
    ) {
        // Bracket the round like the FFI does: `endChangeset` is the only
        // atomic `save()`, so a bare `persistIdentities` would stage the writes
        // without committing them.
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(
            walletId: walletId,
            upserts: [
                PlatformWalletPersistenceHandler.IdentityEntrySnapshot(
                    identityId: ownerId,
                    balance: 0,
                    revision: 0,
                    identityIndex: nil,
                    label: nil,
                    status: 0,
                    walletId: walletId,
                    dpnsNames: [],
                    dashpayProfile: nil,
                    contactProfiles: profiles
                )
            ],
            removed: []
        )
        handler.endChangeset(walletId: walletId, success: true)
    }

    // MARK: Contact-profile tombstone delete

    /// A present profile upserts a row; a later `isPresent == false` tombstone
    /// for the same contact DELETEs it — so a contact who removed their on-chain
    /// DashPay profile can't leave a stale name/avatar behind.
    func testContactProfileTombstoneDeletesPersistedRow() throws {
        applyContactProfiles([
            .init(
                contactIdentityId: contactId,
                isPresent: true,
                displayName: "Carol",
                bio: nil,
                publicMessage: nil,
                avatarUrl: nil,
                avatarHash: nil,
                avatarFingerprint: nil,
                checkedAtMs: 111
            )
        ])
        var rows = try fetchContactProfileRows()
        XCTAssertEqual(rows.count, 1, "a present profile persists one row")
        XCTAssertEqual(rows.first?.displayName, "Carol")

        applyContactProfiles([
            .init(
                contactIdentityId: contactId,
                isPresent: false,
                displayName: nil,
                bio: nil,
                publicMessage: nil,
                avatarUrl: nil,
                avatarHash: nil,
                avatarFingerprint: nil,
                checkedAtMs: 222
            )
        ])
        rows = try fetchContactProfileRows()
        XCTAssertEqual(rows.count, 0, "a tombstone deletes the contact's stale profile row")
    }

    /// A tombstone for a contact that was never persisted is a clean no-op.
    func testContactProfileTombstoneForUnknownContactIsNoop() throws {
        applyContactProfiles([
            .init(
                contactIdentityId: contactId,
                isPresent: false,
                displayName: nil,
                bio: nil,
                publicMessage: nil,
                avatarUrl: nil,
                avatarHash: nil,
                avatarFingerprint: nil,
                checkedAtMs: 111
            )
        ])
        let rows = try fetchContactProfileRows()
        XCTAssertEqual(rows.count, 0, "deleting a never-persisted profile is a no-op")
    }

    // MARK: Upsert mapping

    func testUpsertInsertsRowWithAllFieldsMapped() throws {
        let key = Data((0..<96).map { UInt8($0) })
        let label = Data([0xDE, 0xAD, 0xBE, 0xEF])
        applyContacts(upserts: [
            makeSnapshot(
                isOutgoing: true,
                accountReference: 42,
                encryptedPublicKey: key,
                encryptedAccountLabel: label,
                contactAcceptedAccounts: [7, 42]
            )
        ])

        let rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 1)
        let row = try XCTUnwrap(rows.first)
        XCTAssertEqual(row.ownerIdentityId, ownerId)
        XCTAssertEqual(row.contactIdentityId, contactId)
        XCTAssertTrue(row.isOutgoing)
        XCTAssertEqual(row.senderKeyIndex, 2)
        XCTAssertEqual(row.recipientKeyIndex, 3)
        XCTAssertEqual(row.accountReference, 42)
        XCTAssertEqual(row.encryptedPublicKey, key)
        XCTAssertEqual(row.encryptedAccountLabel, label)
        XCTAssertNil(row.autoAcceptProof)
        XCTAssertEqual(row.coreHeightCreatedAt, 1_234_567)
        XCTAssertEqual(row.createdAtMillis, 1_700_000_000_000)
        XCTAssertFalse(row.paymentChannelBroken)
        XCTAssertEqual(
            row.contactAcceptedAccounts, [7, 42],
            "DIP-15 accepted-account acceptances must round-trip through the persist path"
        )
        XCTAssertEqual(row.network, .testnet)
        XCTAssertEqual(row.owner.identityId, ownerId)
    }

    /// G1c: `payment_channel_broken` is a property of the established
    /// relationship, so the Rust projection stamps it on BOTH direction
    /// rows of the pair — the UI must be able to disable "Send Dash"
    /// regardless of which direction row it happens to read.
    func testPaymentChannelBrokenLandsOnBothDirectionRows() throws {
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: true, paymentChannelBroken: true),
            makeSnapshot(isOutgoing: false, paymentChannelBroken: true),
        ])

        let rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(Set(rows.map(\.isOutgoing)), [true, false])
        for row in rows {
            XCTAssertTrue(
                row.paymentChannelBroken,
                "broken-channel flag must land on the \(row.isOutgoing ? "outgoing" : "incoming") row too"
            )
        }
    }

    /// The `established` promotion re-uses the same
    /// `(network, owner, contact, direction)` unique key as the prior
    /// pending row — the upsert must refresh in place, not grow a
    /// duplicate, and a later flush can flip the broken flag on.
    func testReupsertPromotesPendingRowInPlaceWithoutDuplicate() throws {
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: false, paymentChannelBroken: false)
        ])
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: false, paymentChannelBroken: true)
        ])

        let rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 1, "re-upsert of the same direction row must not duplicate")
        XCTAssertTrue(try XCTUnwrap(rows.first).paymentChannelBroken)
    }

    /// A contact upsert whose owner identity Swift hasn't seen yet is
    /// skipped (next sync round replays it) — it must not crash or
    /// insert an orphan row.
    func testUpsertSkipsUnknownOwnerIdentity() throws {
        let unknownOwner = Data(repeating: 0x77, count: 32)
        var snapshot = makeSnapshot(isOutgoing: false)
        snapshot = .init(
            ownerIdentityId: unknownOwner,
            contactIdentityId: snapshot.contactIdentityId,
            isOutgoing: snapshot.isOutgoing,
            senderKeyIndex: snapshot.senderKeyIndex,
            recipientKeyIndex: snapshot.recipientKeyIndex,
            accountReference: snapshot.accountReference,
            encryptedPublicKey: snapshot.encryptedPublicKey,
            encryptedAccountLabel: snapshot.encryptedAccountLabel,
            autoAcceptProof: snapshot.autoAcceptProof,
            coreHeightCreatedAt: snapshot.coreHeightCreatedAt,
            createdAtMillis: snapshot.createdAtMillis,
            paymentChannelBroken: snapshot.paymentChannelBroken,
            contactAlias: snapshot.contactAlias,
            contactNote: snapshot.contactNote,
            contactHidden: snapshot.contactHidden,
            contactAccountLabel: snapshot.contactAccountLabel,
            contactAcceptedAccounts: snapshot.contactAcceptedAccounts
        )
        applyContacts(upserts: [snapshot])

        XCTAssertEqual(try fetchContactRows().count, 0)
    }

    // MARK: Ignored senders (per-sender mute, local-only)

    /// Ignore is **per-sender** — bare sender id, no accountReference. ALL
    /// of the ignored sender's incoming rows go (including a rotated,
    /// bumped-accountReference one), while a DIFFERENT sender's rows are
    /// never touched. (This is the deliberate semantic change from the old
    /// per-(sender, accountReference) reject.)
    func testIgnoreDeletesAllIncomingRowsFromTheSender() throws {
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: false, accountReference: 7),
            makeSnapshot(contactId: otherSenderId, isOutgoing: false, accountReference: 9),
        ])
        XCTAssertEqual(try fetchContactRows().count, 2)

        // Ignore the sender — its incoming row(s) go regardless of
        // accountReference; the OTHER sender's row stays. A durable
        // PersistentDashpayIgnoredSender row is written.
        applyContacts(ignored: [
            .init(ownerIdentityId: ownerId, senderIdentityId: contactId, isIgnored: true)
        ])
        let rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(try XCTUnwrap(rows.first).contactIdentityId, otherSenderId)
        XCTAssertEqual(try XCTUnwrap(rows.first).accountReference, 9)

        // The durable ignored-sender row exists for the ignored sender only.
        let ignoredRows = try fetchIgnoredRows()
        XCTAssertEqual(ignoredRows.count, 1)
        XCTAssertEqual(try XCTUnwrap(ignoredRows.first).ignoredSenderId, contactId)
    }

    /// Ignore only suppresses the *incoming* direction — an outgoing
    /// request the owner sent to the same identity is unrelated state and
    /// must survive.
    func testIgnoreLeavesOutgoingRowIntact() throws {
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: true, accountReference: 7),
            makeSnapshot(isOutgoing: false, accountReference: 7),
        ])

        applyContacts(ignored: [
            .init(ownerIdentityId: ownerId, senderIdentityId: contactId, isIgnored: true)
        ])

        let rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 1)
        XCTAssertTrue(
            try XCTUnwrap(rows.first).isOutgoing,
            "ignore must delete the incoming row only"
        )
    }

    /// Un-ignore (an `ignored` row with `isIgnored == false`) deletes the
    /// durable ignored-sender row so the sender resurfaces on the next
    /// sweep.
    func testUnignoreDeletesTheIgnoredSenderRow() throws {
        applyContacts(ignored: [
            .init(ownerIdentityId: ownerId, senderIdentityId: contactId, isIgnored: true)
        ])
        XCTAssertEqual(try fetchIgnoredRows().count, 1)

        applyContacts(ignored: [
            .init(ownerIdentityId: ownerId, senderIdentityId: contactId, isIgnored: false)
        ])
        XCTAssertEqual(
            try fetchIgnoredRows().count, 0,
            "un-ignore must delete the durable ignored-sender row"
        )
    }

    /// Read every ignored-sender row back through a fresh context.
    private func fetchIgnoredRows() throws -> [PersistentDashpayIgnoredSender] {
        let context = ModelContext(container)
        return try context.fetch(
            FetchDescriptor<PersistentDashpayIgnoredSender>()
        )
    }

    // MARK: Removal tombstones

    /// `removed_sent` / `removed_incoming` arrive in separate FFI
    /// arrays and each must delete only its own direction row.
    func testRemovalTombstonesAreDirectionScoped() throws {
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: true),
            makeSnapshot(isOutgoing: false),
        ])

        applyContacts(removedSent: [
            .init(ownerIdentityId: ownerId, contactIdentityId: contactId)
        ])
        var rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 1)
        XCTAssertFalse(try XCTUnwrap(rows.first).isOutgoing)

        applyContacts(removedIncoming: [
            .init(ownerIdentityId: ownerId, contactIdentityId: contactId)
        ])
        rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 0)
    }

    // MARK: Full 10-arg C callback round-trip

    /// Drives the *real* `on_persist_contacts_fn` C trampoline (the
    /// 10-argument callback the Rust persister invokes) with synthetic
    /// `ContactRequestFFI` / `ContactIgnoredSenderFFI` payloads —
    /// pinning the FFI-struct marshalling layer (32-byte tuple copies,
    /// heap byte-buffer copies, `payment_channel_broken` projection)
    /// on top of the snapshot path the other tests exercise.
    func testPersistContactsCallbackMarshalsTenArgPayload() throws {
        let callbacks = handler.makeCallbacks()
        let beginFn = try XCTUnwrap(callbacks.on_changeset_begin_fn)
        let contactsFn = try XCTUnwrap(callbacks.on_persist_contacts_fn)
        let endFn = try XCTUnwrap(callbacks.on_changeset_end_fn)

        let encryptedKey = Data((0..<96).map { UInt8($0 ^ 0x5A) })
        let label = Data([0x01, 0x02, 0x03])
        let accepted: [UInt32] = [7, 42]

        walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) in
            guard let wid = widRaw.bindMemory(to: UInt8.self).baseAddress else {
                XCTFail("wallet-id buffer must bind")
                return
            }
            _ = beginFn(callbacks.context, wid)

            encryptedKey.withUnsafeBytes { (keyRaw: UnsafeRawBufferPointer) in
                label.withUnsafeBytes { (labelRaw: UnsafeRawBufferPointer) in
                    // Keep the accepted-accounts buffer alive for the whole
                    // call — Rust owns it in production, but here the test
                    // owns it and must outlive `contactsFn`.
                    accepted.withUnsafeBufferPointer { acceptedPtr in
                    let keyPtr = keyRaw.bindMemory(to: UInt8.self).baseAddress
                    let labelPtr = labelRaw.bindMemory(to: UInt8.self).baseAddress

                    var outgoing = ContactRequestFFI()
                    outgoing.owner_id = Self.tuple32(ownerId)
                    outgoing.contact_id = Self.tuple32(contactId)
                    outgoing.is_outgoing = true
                    outgoing.sender_key_index = 5
                    outgoing.recipient_key_index = 6
                    outgoing.account_reference = 11
                    outgoing.encrypted_public_key = keyPtr
                    outgoing.encrypted_public_key_len = UInt(encryptedKey.count)
                    outgoing.encrypted_account_label = labelPtr
                    outgoing.encrypted_account_label_len = UInt(label.count)
                    outgoing.core_height_created_at = 99
                    outgoing.created_at = 1_700_000_000_123
                    outgoing.payment_channel_broken = true
                    outgoing.accepted_accounts = acceptedPtr.baseAddress
                    outgoing.accepted_accounts_len = UInt(accepted.count)

                    var incoming = outgoing
                    incoming.is_outgoing = false
                    incoming.encrypted_account_label = nil
                    incoming.encrypted_account_label_len = 0
                    // A null/0 accepted-accounts pointer must map to an
                    // empty array (not a crash).
                    incoming.accepted_accounts = nil
                    incoming.accepted_accounts_len = 0

                    let rows = [outgoing, incoming]
                    rows.withUnsafeBufferPointer { rowsPtr in
                        let rc = contactsFn(
                            callbacks.context,
                            wid,
                            rowsPtr.baseAddress,
                            UInt(rows.count),
                            nil, 0,
                            nil, 0,
                            nil, 0
                        )
                        XCTAssertEqual(rc, 0)
                    }
                    }
                }
            }

            _ = endFn(callbacks.context, wid, true)
        }

        let rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 2)
        for row in rows {
            XCTAssertEqual(row.ownerIdentityId, ownerId)
            XCTAssertEqual(row.contactIdentityId, contactId)
            XCTAssertEqual(row.senderKeyIndex, 5)
            XCTAssertEqual(row.recipientKeyIndex, 6)
            XCTAssertEqual(row.accountReference, 11)
            XCTAssertEqual(row.encryptedPublicKey, encryptedKey)
            XCTAssertEqual(row.coreHeightCreatedAt, 99)
            XCTAssertEqual(row.createdAtMillis, 1_700_000_000_123)
            XCTAssertTrue(row.paymentChannelBroken)
        }
        let outgoingRow = try XCTUnwrap(rows.first(where: \.isOutgoing))
        let incomingRow = try XCTUnwrap(rows.first(where: { !$0.isOutgoing }))
        XCTAssertEqual(outgoingRow.encryptedAccountLabel, label)
        XCTAssertNil(
            incomingRow.encryptedAccountLabel,
            "null label pointer must map to nil, not empty Data"
        )
        XCTAssertEqual(
            outgoingRow.contactAcceptedAccounts, [7, 42],
            "non-null accepted-accounts pointer must marshal into an owned [UInt32]"
        )
        XCTAssertEqual(
            incomingRow.contactAcceptedAccounts, [],
            "null/0 accepted-accounts pointer must map to an empty array"
        )

        // Ignore leg of the same callback: ignore the sender (drop the
        // incoming row + write the durable ignored-sender row) through the
        // C signature too.
        walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) in
            guard let wid = widRaw.bindMemory(to: UInt8.self).baseAddress else {
                XCTFail("wallet-id buffer must bind")
                return
            }
            _ = beginFn(callbacks.context, wid)
            var ignore = ContactIgnoredSenderFFI()
            ignore.owner_id = Self.tuple32(ownerId)
            ignore.sender_id = Self.tuple32(contactId)
            ignore.is_ignored = true
            withUnsafePointer(to: &ignore) { ignPtr in
                let rc = contactsFn(
                    callbacks.context,
                    wid,
                    nil, 0,
                    nil, 0,
                    nil, 0,
                    ignPtr, 1
                )
                XCTAssertEqual(rc, 0)
            }
            _ = endFn(callbacks.context, wid, true)
        }

        let afterIgnore = try fetchContactRows()
        XCTAssertEqual(afterIgnore.count, 1)
        XCTAssertTrue(try XCTUnwrap(afterIgnore.first).isOutgoing)
    }

    // MARK: accepted_accounts round-trip (F11)

    /// An established contact's DIP-15 `accepted_accounts` must survive
    /// the persist path — the FFI carries them as a `(u32*, len)` pair
    /// replicated onto both direction rows, and the handler stores them
    /// on `contactAcceptedAccounts`. Against the unfixed handler (which
    /// ignored the field) the persisted rows come back empty.
    func testPersistPreservesAcceptedAccounts() throws {
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: true, contactAcceptedAccounts: [7, 42]),
            makeSnapshot(isOutgoing: false, contactAcceptedAccounts: [7, 42]),
        ])

        let rows = try fetchContactRows()
        XCTAssertEqual(rows.count, 2)
        for row in rows {
            XCTAssertEqual(
                row.contactAcceptedAccounts, [7, 42],
                "both direction rows must carry the relationship's accepted accounts"
            )
        }
    }

    /// The full relaunch restore: persist an established contact with
    /// `accepted_accounts = [7, 42]`, then drive `loadWalletList()` (the
    /// FFI load path the app runs on cold start) and assert the rebuilt
    /// `ContactRequestFFI` rows carry them back. Against the unfixed
    /// restore path (which rebuilt rows via `EstablishedContact::new`
    /// and never set `accepted_accounts`) the rebuilt rows come back
    /// empty, silently resetting the value every launch.
    func testRestoreRebuildsAcceptedAccounts() throws {
        // A restorable wallet needs at least one account carrying a
        // non-empty extended pubkey, plus the owner identity linked to
        // the wallet so `loadWalletList` walks its contact rows.
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet,
            accountType: 0,
            accountIndex: 0,
            accountTypeName: "standard"
        )
        account.accountExtendedPubKeyBytes = Data(repeating: 0xEE, count: 78)
        context.insert(account)
        let target = ownerId
        let ownerDescriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == target }
        )
        let owner = try XCTUnwrap(try context.fetch(ownerDescriptor).first)
        owner.wallet = wallet
        try context.save()

        // Persist an established contact carrying the accepted accounts.
        applyContacts(upserts: [
            makeSnapshot(isOutgoing: true, contactAcceptedAccounts: [7, 42]),
            makeSnapshot(isOutgoing: false, contactAcceptedAccounts: [7, 42]),
        ])

        let (entries, count, errored) = handler.loadWalletList()
        XCTAssertFalse(errored)
        XCTAssertEqual(count, 1)
        let entriesPtr = try XCTUnwrap(entries)
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entriesPtr)) }

        var seen: [[UInt32]] = []
        let walletEntry = entriesPtr[0]
        XCTAssertGreaterThan(walletEntry.identities_count, 0)
        for iIdx in 0..<Int(walletEntry.identities_count) {
            let identity = walletEntry.identities![iIdx]
            for cIdx in 0..<Int(identity.contacts_count) {
                let contact = identity.contacts![cIdx]
                if let acceptedPtr = contact.accepted_accounts,
                   contact.accepted_accounts_len > 0 {
                    seen.append(Array(
                        UnsafeBufferPointer(
                            start: acceptedPtr,
                            count: Int(contact.accepted_accounts_len)
                        )
                    ))
                } else {
                    seen.append([])
                }
            }
        }

        XCTAssertEqual(seen.count, 2, "both direction rows must be rebuilt")
        for accepted in seen {
            XCTAssertEqual(
                accepted, [7, 42],
                "restore must rebuild accepted_accounts, not reset them to empty"
            )
        }
    }

    // MARK: Changeset atomicity vs app-facing writers

    /// Regression: an app-facing `persistDashpayPayments` refresh that
    /// lands while a Rust persister round is open (between
    /// `beginChangeset` and `endChangeset`) must NOT commit the
    /// round's half-applied writes early. Every other app-facing
    /// writer in the handler guards its immediate save with
    /// `if !inChangeset`; without that guard here, a payments
    /// pull-to-refresh racing a sync round breaks the documented
    /// "each Rust store() is one atomic transaction" invariant — a
    /// failed round could no longer roll back cleanly.
    func testPaymentRefreshDoesNotCommitAnOpenChangesetRound() throws {
        // Open a round and stage an (uncommitted) contact write.
        handler.beginChangeset(walletId: walletId)
        handler.persistContacts(
            walletId: walletId,
            upserts: [makeSnapshot(isOutgoing: false)],
            removedSent: [],
            removedIncoming: [],
            ignored: []
        )

        // App-facing payment refresh lands mid-round.
        handler.persistDashpayPayments(
            ownerIdentityId: ownerId,
            payments: [
                DashPayPayment(
                    counterpartyId: contactId,
                    amountDuffs: 1_000,
                    direction: .sent,
                    status: .pending,
                    txid: "0011223344556677"
                )
            ]
        )

        // The open round's writes must not be visible to other
        // contexts yet.
        XCTAssertEqual(
            try fetchContactRows().count, 0,
            "a mid-round payment refresh must not flush the open changeset early"
        )

        // Fail the round — everything staged since begin (contact row
        // AND the payment row that rode the round) must roll back.
        handler.endChangeset(walletId: walletId, success: false)
        XCTAssertEqual(try fetchContactRows().count, 0)
    }

    /// Copy a 32-byte `Data` into the C fixed-array tuple shape the
    /// FFI structs use for ids.
    private static func tuple32(_ data: Data) -> FFIByteTuple32 {
        precondition(data.count == 32)
        var tuple: FFIByteTuple32 = (
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        )
        withUnsafeMutableBytes(of: &tuple) { $0.copyBytes(from: data) }
        return tuple
    }
}

// MARK: - DashPay payment-history persistence

final class DashPayPaymentPersistenceTests: XCTestCase {

    private var container: ModelContainer!
    private var handler: PlatformWalletPersistenceHandler!

    private let ownerId = Data(repeating: 0x01, count: 32)
    private let secondOwnerId = Data(repeating: 0x04, count: 32)
    private let counterpartyId = Data(repeating: 0x02, count: 32)
    private let txid = "f0e1d2c3b4a5968778695a4b3c2d1e0f00112233445566778899aabbccddeeff"

    override func setUpWithError() throws {
        try super.setUpWithError()
        container = try DashModelContainer.createInMemory()
        handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet
        )
        let context = ModelContext(container)
        context.insert(
            PersistentIdentity(identityId: ownerId, isLocal: false, network: .testnet)
        )
        context.insert(
            PersistentIdentity(identityId: secondOwnerId, isLocal: false, network: .testnet)
        )
        try context.save()
    }

    override func tearDown() {
        handler = nil
        container = nil
        super.tearDown()
    }

    private func fetchPaymentRows() throws -> [PersistentDashpayPayment] {
        let context = ModelContext(container)
        return try context.fetch(FetchDescriptor<PersistentDashpayPayment>())
    }

    private func makePayment(
        status: DashPayPaymentStatus = .pending,
        direction: DashPayPaymentDirection = .sent,
        memo: String? = nil
    ) -> DashPayPayment {
        DashPayPayment(
            counterpartyId: counterpartyId,
            amountDuffs: 250_000,
            direction: direction,
            status: status,
            txid: txid,
            memo: memo
        )
    }

    /// The refresh path re-reads the whole Rust `dashpay_payments` map
    /// on every call — re-upserting the same txid must refresh the row
    /// in place (status is the field that actually moves) and never
    /// grow a duplicate.
    func testReupsertSameTxidUpdatesInPlaceWithoutDuplicate() throws {
        handler.persistDashpayPayments(
            ownerIdentityId: ownerId,
            payments: [makePayment(status: .pending)]
        )
        handler.persistDashpayPayments(
            ownerIdentityId: ownerId,
            payments: [makePayment(status: .confirmed, memo: "lunch")]
        )

        let rows = try fetchPaymentRows()
        XCTAssertEqual(rows.count, 1, "same (owner, txid) must upsert, not duplicate")
        let row = try XCTUnwrap(rows.first)
        XCTAssertEqual(row.status, .confirmed)
        XCTAssertEqual(row.memo, "lunch")
        XCTAssertEqual(row.amountDuffs, 250_000)
        XCTAssertEqual(row.direction, .sent)
        XCTAssertEqual(row.txid, txid)
        XCTAssertEqual(row.counterpartyIdentityId, counterpartyId)
        XCTAssertEqual(row.ownerIdentityId, ownerId)
        XCTAssertEqual(row.network, .testnet)
    }

    /// The unique key is `(network, owner, txid)` — the same txid seen
    /// from two wallet-managed identities (e.g. an in-wallet transfer
    /// between own identities) is two distinct history rows.
    func testSameTxidAcrossDifferentOwnersCreatesSeparateRows() throws {
        handler.persistDashpayPayments(
            ownerIdentityId: ownerId,
            payments: [makePayment(direction: .sent)]
        )
        handler.persistDashpayPayments(
            ownerIdentityId: secondOwnerId,
            payments: [makePayment(direction: .received)]
        )

        let rows = try fetchPaymentRows()
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(
            Set(rows.map(\.ownerIdentityId)),
            [ownerId, secondOwnerId]
        )

        // The owner-scoped predicate the views query through must
        // partition the rows.
        let context = ModelContext(container)
        let ownerRows = try context.fetch(
            FetchDescriptor<PersistentDashpayPayment>(
                predicate: PersistentDashpayPayment.predicate(ownerIdentityId: ownerId)
            )
        )
        XCTAssertEqual(ownerRows.count, 1)
        XCTAssertEqual(try XCTUnwrap(ownerRows.first).direction, .sent)
    }

    /// Defensive paths: an empty txid (degraded FFI row) is skipped,
    /// and an owner identity Swift doesn't know yet means the whole
    /// batch is deferred to the next refresh — neither may crash or
    /// write partial rows.
    func testSkipsEmptyTxidAndUnknownOwner() throws {
        let emptyTxid = DashPayPayment(
            counterpartyId: counterpartyId,
            amountDuffs: 1,
            direction: .sent,
            status: .pending,
            txid: "",
            memo: nil
        )
        handler.persistDashpayPayments(ownerIdentityId: ownerId, payments: [emptyTxid])
        XCTAssertEqual(try fetchPaymentRows().count, 0)

        let unknownOwner = Data(repeating: 0x99, count: 32)
        handler.persistDashpayPayments(
            ownerIdentityId: unknownOwner,
            payments: [makePayment()]
        )
        XCTAssertEqual(try fetchPaymentRows().count, 0)
    }

    // MARK: Changeset persister-callback path (event-driven durability)

    /// The event-driven write half of the payment durability loop: a
    /// batch delivered by the `on_persist_dashpay_payments_fn` round —
    /// no UI refresh ever running — must commit with the round and
    /// round-trip through the cold-start restore buffer with the Sent
    /// entry's memo intact. Against the getter-only era this exact
    /// flow lost the row on relaunch unless `ContactDetailView`
    /// happened to appear first.
    func testChangesetRoundPersistsSentEntryAndRestoreBufferRoundTripsIt() throws {
        // Restorable-wallet scaffolding for the load half (mirrors
        // testRestoreRebuildsAcceptedAccounts): loadWalletList only
        // walks identities linked to a wallet with a restorable
        // account.
        let walletId = Data(repeating: 0xAA, count: 32)
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet,
            accountType: 0,
            accountIndex: 0,
            accountTypeName: "standard"
        )
        account.accountExtendedPubKeyBytes = Data(repeating: 0xEE, count: 78)
        context.insert(account)
        let target = ownerId
        let ownerDescriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == target }
        )
        let owner = try XCTUnwrap(try context.fetch(ownerDescriptor).first)
        owner.wallet = wallet
        try context.save()

        // The persister round: begin → payments batch → end.
        handler.beginChangeset(walletId: walletId)
        handler.persistDashpayPayments(
            walletId: walletId,
            entriesByOwner: [ownerId: [makePayment(status: .pending, memo: "rent + utilities")]]
        )
        // Mid-round: staged, not committed.
        XCTAssertEqual(
            try fetchPaymentRows().count, 0,
            "the callback must ride the round's atomic commit, not flush early"
        )
        handler.endChangeset(walletId: walletId, success: true)

        let rows = try fetchPaymentRows()
        XCTAssertEqual(rows.count, 1)
        let row = try XCTUnwrap(rows.first)
        XCTAssertEqual(row.memo, "rent + utilities")
        XCTAssertEqual(row.status, .pending)
        XCTAssertEqual(row.direction, .sent)
        XCTAssertEqual(row.ownerIdentityId, ownerId)
        XCTAssertEqual(row.txid, txid)

        // Cold-start restore: the row must ride the identity restore
        // buffer's payments array back into Rust.
        let (entries, count, errored) = handler.loadWalletList()
        XCTAssertFalse(errored)
        XCTAssertEqual(count, 1)
        let entriesPtr = try XCTUnwrap(entries)
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entriesPtr)) }

        var restored: [(txid: String, memo: String?)] = []
        let walletEntry = entriesPtr[0]
        for iIdx in 0..<Int(walletEntry.identities_count) {
            let identity = walletEntry.identities![iIdx]
            guard let paymentsPtr = identity.payments, identity.payments_count > 0 else {
                continue
            }
            for pIdx in 0..<Int(identity.payments_count) {
                let p = paymentsPtr[pIdx]
                restored.append((
                    txid: p.txid.map { String(cString: $0) } ?? "",
                    memo: p.memo.map { String(cString: $0) }
                ))
            }
        }
        XCTAssertEqual(restored.count, 1, "the persisted payment must rehydrate at load")
        XCTAssertEqual(restored.first?.txid, txid)
        XCTAssertEqual(restored.first?.memo, "rent + utilities")
    }

    /// The confirm sweep re-records the same `(owner, txid)` with
    /// status Confirmed and the next round re-emits the row — the flip
    /// must persist in place through the callback path, never
    /// duplicate.
    func testChangesetRoundStatusFlipRepersistsTheSameRow() throws {
        let walletId = Data(repeating: 0xAA, count: 32)
        handler.beginChangeset(walletId: walletId)
        handler.persistDashpayPayments(
            walletId: walletId,
            entriesByOwner: [ownerId: [makePayment(status: .pending)]]
        )
        handler.endChangeset(walletId: walletId, success: true)
        XCTAssertEqual(try fetchPaymentRows().first?.status, .pending)

        handler.beginChangeset(walletId: walletId)
        handler.persistDashpayPayments(
            walletId: walletId,
            entriesByOwner: [ownerId: [makePayment(status: .confirmed)]]
        )
        handler.endChangeset(walletId: walletId, success: true)

        let rows = try fetchPaymentRows()
        XCTAssertEqual(rows.count, 1, "a status flip must upsert in place, not duplicate")
        XCTAssertEqual(rows.first?.status, .confirmed)
    }

    /// A payments batch whose owner identity is staged LATER in the
    /// same round parks mid-round and is staged again by `endChangeset`
    /// BEFORE the round's single save — so the payment rows commit in
    /// the same atomic transaction as the owner identity, never in a
    /// second post-commit save a process kill could separate from the
    /// round.
    func testRowsForAnOwnerStagedLaterInTheRoundCommitAtomically() throws {
        let walletId = Data(repeating: 0xAA, count: 32)
        let lateOwner = Data(repeating: 0x33, count: 32)
        // Wallet row so `persistIdentities` can resolve the network
        // for the brand-new identity.
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()

        handler.beginChangeset(walletId: walletId)
        // Payments land before the owner's identity row this round.
        handler.persistDashpayPayments(
            walletId: walletId,
            entriesByOwner: [lateOwner: [makePayment(memo: "parked")]]
        )
        handler.persistIdentities(
            walletId: walletId,
            upserts: [
                PlatformWalletPersistenceHandler.IdentityEntrySnapshot(
                    identityId: lateOwner,
                    balance: 0,
                    revision: 0,
                    identityIndex: nil,
                    label: nil,
                    status: 0,
                    walletId: walletId,
                    dpnsNames: [],
                    dashpayProfile: nil,
                    contactProfiles: []
                )
            ],
            removed: []
        )
        let committed = handler.endChangeset(walletId: walletId, success: true)
        XCTAssertTrue(committed, "a resolvable parked owner must not fail the round")

        let rows = try fetchPaymentRows()
        XCTAssertEqual(
            rows.count, 1,
            "parked rows must commit with the round's single save"
        )
        XCTAssertEqual(rows.first?.ownerIdentityId, lateOwner)
        XCTAssertEqual(rows.first?.memo, "parked")
    }

    /// A payments batch whose owner identity never appears — neither
    /// pre-existing nor staged by the round — must FAIL the round:
    /// `endChangeset` rolls back and reports failure to Rust (which
    /// then rolls its in-memory entry back), instead of committing the
    /// rest of the round while silently dropping payment rows.
    func testUnresolvableOwnerFailsTheRoundInsteadOfDroppingRows() throws {
        let walletId = Data(repeating: 0xAA, count: 32)
        let ghostOwner = Data(repeating: 0x55, count: 32)

        handler.beginChangeset(walletId: walletId)
        handler.persistDashpayPayments(
            walletId: walletId,
            entriesByOwner: [
                ownerId: [makePayment()],
                ghostOwner: [makePayment(memo: "orphaned")],
            ]
        )
        let committed = handler.endChangeset(walletId: walletId, success: true)

        XCTAssertFalse(
            committed,
            "an unresolvable payment owner must fail the round, not drop the rows"
        )
        XCTAssertEqual(
            try fetchPaymentRows().count, 0,
            "the failed round must roll back everything it staged"
        )
    }

    /// A failed round discards BOTH staged and parked payment rows —
    /// the Rust side rolled the entries back out of its in-memory map,
    /// so persisting them later would fabricate history.
    func testFailedRoundRollsBackStagedAndParkedPaymentRows() throws {
        let walletId = Data(repeating: 0xAA, count: 32)
        let unknownOwner = Data(repeating: 0x44, count: 32)
        handler.beginChangeset(walletId: walletId)
        handler.persistDashpayPayments(
            walletId: walletId,
            entriesByOwner: [
                ownerId: [makePayment()],
                unknownOwner: [makePayment(memo: "will park")],
            ]
        )
        handler.endChangeset(walletId: walletId, success: false)
        XCTAssertEqual(try fetchPaymentRows().count, 0)

        // Make the parked group's owner appear and run a healthy round:
        // the failed round's parked rows must NOT resurrect.
        let context = ModelContext(container)
        context.insert(
            PersistentIdentity(identityId: unknownOwner, isLocal: false, network: .testnet)
        )
        try context.save()
        handler.beginChangeset(walletId: walletId)
        handler.endChangeset(walletId: walletId, success: true)
        XCTAssertEqual(
            try fetchPaymentRows().count, 0,
            "rolled-back parked rows must be discarded, not replayed"
        )
    }
}

// MARK: - DashPayPayment FFI value-struct marshalling

final class DashPayPaymentFFIMarshallingTests: XCTestCase {

    private let counterpartyId = Data((0..<32).map { UInt8($0 + 1) })

    private static func tuple32(_ data: Data) -> FFIByteTuple32 {
        precondition(data.count == 32)
        var tuple: FFIByteTuple32 = (
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        )
        withUnsafeMutableBytes(of: &tuple) { $0.copyBytes(from: data) }
        return tuple
    }

    func testInitFromFFICopiesAllFields() throws {
        let txidCString = strdup("ab12cd34")
        let memoCString = strdup("coffee ☕")
        defer {
            free(txidCString)
            free(memoCString)
        }

        var ffi = DashpayPaymentFFI()
        ffi.counterparty_id = Self.tuple32(counterpartyId)
        ffi.amount_duffs = 123_456_789
        ffi.direction = DashPayPaymentDirection.received.rawValue
        ffi.status = DashPayPaymentStatus.confirmed.rawValue
        ffi.txid = txidCString
        ffi.memo = memoCString

        let payment = DashPayPayment(ffi: ffi)
        XCTAssertEqual(payment.counterpartyId, counterpartyId)
        XCTAssertEqual(payment.amountDuffs, 123_456_789)
        XCTAssertEqual(payment.direction, .received)
        XCTAssertEqual(payment.status, .confirmed)
        XCTAssertEqual(payment.txid, "ab12cd34")
        XCTAssertEqual(payment.memo, "coffee ☕")
    }

    /// Optional memo: a null pointer mirrors the Rust `Option::None`
    /// and must come through as `nil`, not an empty string.
    func testNullMemoMapsToNil() throws {
        let txidCString = strdup("ff00")
        defer { free(txidCString) }

        var ffi = DashpayPaymentFFI()
        ffi.counterparty_id = Self.tuple32(counterpartyId)
        ffi.amount_duffs = 1
        ffi.direction = DashPayPaymentDirection.sent.rawValue
        ffi.status = DashPayPaymentStatus.pending.rawValue
        ffi.txid = txidCString
        ffi.memo = nil

        let payment = DashPayPayment(ffi: ffi)
        XCTAssertNil(payment.memo)
        XCTAssertEqual(payment.txid, "ff00")
    }

    /// Forward compatibility: unknown direction / status discriminants
    /// from a newer Rust enum must degrade to the documented fallbacks
    /// (`.sent` / `.pending`) instead of making history unreadable; a
    /// (contract-violating) null txid degrades to "" instead of
    /// trapping.
    func testUnknownDiscriminantsAndNullTxidDegradeGracefully() throws {
        var ffi = DashpayPaymentFFI()
        ffi.counterparty_id = Self.tuple32(counterpartyId)
        ffi.amount_duffs = 42
        ffi.direction = 99
        ffi.status = 99
        ffi.txid = nil
        ffi.memo = nil

        let payment = DashPayPayment(ffi: ffi)
        XCTAssertEqual(payment.direction, .sent)
        XCTAssertEqual(payment.status, .pending)
        XCTAssertEqual(payment.txid, "")
    }
}
