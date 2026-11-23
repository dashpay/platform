import DashSDKFFI
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Which identity a masternode loads as is rs-sdk's rule
/// (`load_masternode_for_voting`) and tested there. These show that the row
/// keeps the masternode's pro_tx_hash apart from its identity id, and that the
/// decision crosses the FFI as Rust writes it.
@MainActor
final class MasternodeVotingLoadTests: XCTestCase {
    /// A pro_tx_hash in wire order whose two byte orders differ.
    private let wireOrderProTxHash = Data((1...32).map { UInt8($0) })

    /// A masternode stored under its voter identity keeps the pro_tx_hash it
    /// was loaded with, and votes name that, not anything derived from the
    /// identity id.
    func testShouldRoundTripTheProTxHashOfARowStoredUnderItsVoterIdentity() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let voterIdentityId = Data(repeating: 0x5a, count: 32)
        let identity = PersistentIdentity(
            identityId: voterIdentityId,
            identityType: .masternode,
            network: .testnet
        )
        identity.masternodeProTxHash = wireOrderProTxHash
        context.insert(identity)
        try context.save()

        let fresh = ModelContext(container)
        let fetched = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentIdentity>()).first)
        XCTAssertFalse(fetched === identity)
        XCTAssertEqual(fetched.identityId, voterIdentityId)
        XCTAssertEqual(fetched.masternodeProTxHash, wireOrderProTxHash)
        XCTAssertEqual(fetched.votingProTxHash, wireOrderProTxHash)
    }

    /// A row stored before the pro_tx_hash was recorded was loaded at its
    /// owner identity, whose id is the pro_tx_hash in Platform's order: its
    /// votes name the wire order, the reverse, as they did before.
    func testShouldNameTheOwnerIdentitysProTxHashForARowStoredBeforeItWasRecorded() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let ownerIdentityId = Data(wireOrderProTxHash.reversed())
        let identity = PersistentIdentity(
            identityId: ownerIdentityId,
            identityType: .evonode,
            network: .testnet
        )
        context.insert(identity)
        try context.save()

        let fresh = ModelContext(container)
        let fetched = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentIdentity>()).first)
        XCTAssertNil(fetched.masternodeProTxHash)
        XCTAssertEqual(fetched.votingProTxHash, wireOrderProTxHash)
    }

    func testShouldReadARefusalWithoutAMessageAsAnEmptyReason() throws {
        var load = DashSDKMasternodeVotingLoad()
        load.kind = DashSDKMasternodeVotingLoadKind_Refused
        guard case .refused(let message) = try MasternodeVotingLoadFFI.consume(&load) else {
            return XCTFail("expected a refusal")
        }
        XCTAssertEqual(message, "")
    }

    /// A loaded identity always carries its JSON; a load without it is an
    /// FFI fault, never an empty identity to store.
    func testShouldRefuseALoadThatCarriesNoIdentity() {
        for kind in [
            DashSDKMasternodeVotingLoadKind_OwnerIdentity,
            DashSDKMasternodeVotingLoadKind_VoterIdentity,
        ] {
            var load = DashSDKMasternodeVotingLoad()
            load.kind = kind
            XCTAssertThrowsError(try MasternodeVotingLoadFFI.consume(&load))
        }
    }

    func testShouldParseTheLoadedIdentityAsTheIdentityFetchDoes() throws {
        let loaded = MasternodeVotingLoadedIdentity(
            role: .voter,
            identityId: Data(repeating: 0x5a, count: 32),
            identityJSON: #"{"id":"abc","balance":11,"publicKeys":[]}"#,
            votingKeyCheck: .valid
        )
        let object = try loaded.identityObject()
        XCTAssertEqual(object["balance"] as? Int, 11)

        let broken = MasternodeVotingLoadedIdentity(
            role: .owner,
            identityId: Data(repeating: 0x5a, count: 32),
            identityJSON: "not JSON",
            votingKeyCheck: nil
        )
        XCTAssertThrowsError(try broken.identityObject())
    }
}
