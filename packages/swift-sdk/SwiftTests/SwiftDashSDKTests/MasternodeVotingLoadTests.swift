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
    /// was loaded with and its role, not anything derived from the identity
    /// id, and as a voting-only identity it is never a recipient.
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
        identity.masternodeRole = .voter
        context.insert(identity)
        try context.save()

        let fresh = ModelContext(container)
        let fetched = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentIdentity>()).first)
        XCTAssertFalse(fetched === identity)
        XCTAssertEqual(fetched.identityId, voterIdentityId)
        XCTAssertEqual(fetched.masternodeProTxHash, wireOrderProTxHash)
        XCTAssertEqual(fetched.masternodeRole, .voter)
        XCTAssertTrue(fetched.isVotingOnlyIdentity)
    }

    /// A send is refused to an identity this device stores on the network as
    /// a masternode's voter identity, and only to that: not the same id on
    /// another network, an owner identity's row, or an unknown id.
    func testShouldKnowOnlyAStoredVoterIdentityAsAVotingOnlyRecipient() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let voterId = Data(repeating: 0x5a, count: 32)
        let ownerId = Data(repeating: 0x6b, count: 32)
        let voter = PersistentIdentity(identityId: voterId, identityType: .masternode, network: .testnet)
        voter.masternodeRole = .voter
        let owner = PersistentIdentity(identityId: ownerId, identityType: .masternode, network: .testnet)
        owner.masternodeRole = .owner
        context.insert(voter)
        context.insert(owner)
        try context.save()

        XCTAssertTrue(try PersistentIdentity.isKnownVotingOnlyIdentity(voterId, on: .testnet, in: context))
        XCTAssertFalse(try PersistentIdentity.isKnownVotingOnlyIdentity(voterId, on: .mainnet, in: context))
        XCTAssertFalse(try PersistentIdentity.isKnownVotingOnlyIdentity(ownerId, on: .testnet, in: context))
        XCTAssertFalse(try PersistentIdentity.isKnownVotingOnlyIdentity(
            Data(repeating: 0x01, count: 32),
            on: .testnet,
            in: context
        ))
    }

    /// A row stored before the pro_tx_hash was recorded names no masternode:
    /// nothing is inferred from its id until Rust's load rule resolves it,
    /// and it is not a voting-only identity.
    func testShouldNameNoMasternodeForARowStoredBeforeItWasRecorded() throws {
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
        XCTAssertNil(fetched.masternodeRole)
        XCTAssertFalse(fetched.isVotingOnlyIdentity)
    }

    /// A successful voter load decodes from the C struct: role, id, the
    /// Rust-owned JSON (freed, pointer cleared) and the verdict.
    func testShouldDecodeASuccessfulVoterLoad() throws {
        // Rust-owned strings, so `consume` can free them as it frees the
        // load's: `dash_sdk_utils_hex_to_base58` allocates its answer in Rust.
        func rustString(_ hex: String) throws -> (UnsafeMutablePointer<CChar>, String) {
            let result = hex.withCString { dash_sdk_utils_hex_to_base58($0) }
            XCTAssertNil(result.error)
            let pointer = try XCTUnwrap(result.data).assumingMemoryBound(to: CChar.self)
            return (pointer, String(cString: pointer))
        }
        let (json, jsonText) = try rustString(String(repeating: "5a", count: 32))
        var load = DashSDKMasternodeVotingLoad()
        load.kind = DashSDKMasternodeVotingLoadKind_VoterIdentity
        let identityId = Data((101...132).map { UInt8($0) })
        withUnsafeMutableBytes(of: &load.identity_id) { $0.copyBytes(from: identityId) }
        load.identity_json = json
        load.has_voting_key_verdict = true
        load.voting_key_verdict = DashSDKVotingKeyVerdict_Valid
        guard case .loaded(let loaded) = try MasternodeVotingLoadFFI.consume(&load) else {
            return XCTFail("expected a load")
        }
        XCTAssertEqual(loaded.role, .voter)
        XCTAssertEqual(loaded.identityId, identityId)
        XCTAssertEqual(loaded.identityJSON, jsonText)
        XCTAssertEqual(loaded.votingKeyCheck, .valid)
        XCTAssertNil(load.identity_json, "the string was freed and its pointer cleared")
        XCTAssertNil(load.message)
    }

    /// Rust's sibling rule crosses the wrapper: once one key is proved
    /// valid, a valid record of another key is checked again, with the
    /// wrapper's message; a refused key is left as it is.
    func testShouldCheckAgainAValidSiblingKeyOnceAnotherKeyIsProvedValid() throws {
        let loaded = try XCTUnwrap(MasternodeVotingKeyAddress(Data(repeating: 0x07, count: 20)))
        let other = try XCTUnwrap(MasternodeVotingKeyAddress(Data(repeating: 0x09, count: 20)))
        XCTAssertEqual(
            MasternodeVotingKeyCheck.valid.siblingRecord(
                votingAddress: loaded,
                sibling: MasternodeRecordedVotingKey(check: .valid, votingAddress: other)
            ),
            MasternodeRecordedVotingKey(
                check: .undetermined(message: MasternodeVotingKeyCheck.checkAgainAfterAnotherKeyMessage),
                votingAddress: other
            )
        )
        XCTAssertNil(MasternodeVotingKeyCheck.valid.siblingRecord(
            votingAddress: loaded,
            sibling: MasternodeRecordedVotingKey(check: .invalid(message: "rotated"), votingAddress: other)
        ))
        XCTAssertNil(MasternodeVotingKeyCheck.undetermined(message: "timeout").siblingRecord(
            votingAddress: loaded,
            sibling: MasternodeRecordedVotingKey(check: .valid, votingAddress: other)
        ))
    }

    func testShouldReadARefusalWithoutAMessageAsAnEmptyReason() throws {
        var load = DashSDKMasternodeVotingLoad()
        load.kind = DashSDKMasternodeVotingLoadKind_Refused
        guard case .refused(let message, let refusedFor) = try MasternodeVotingLoadFFI.consume(&load) else {
            return XCTFail("expected a refusal")
        }
        XCTAssertEqual(message, "")
        XCTAssertNil(refusedFor, "no verdict: the refusal names no voter identity")
    }

    /// A refused voting key names the voter identity it was refused for.
    func testShouldCarryTheVoterIdentityARefusedVotingKeyNames() throws {
        var load = DashSDKMasternodeVotingLoad()
        load.kind = DashSDKMasternodeVotingLoadKind_Refused
        load.has_voting_key_verdict = true
        load.voting_key_verdict = DashSDKVotingKeyVerdict_Invalid
        withUnsafeMutableBytes(of: &load.identity_id) { $0.copyBytes(from: Data(repeating: 0x5a, count: 32)) }
        guard case .refused(_, let refusedFor) = try MasternodeVotingLoadFFI.consume(&load) else {
            return XCTFail("expected a refusal")
        }
        XCTAssertEqual(refusedFor, Data(repeating: 0x5a, count: 32))
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
