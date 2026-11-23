import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the contract moderation JSON the `rs-platform-wallet-ffi`
/// `platform_wallet_contract_moderation_offer`,
/// `platform_wallet_contract_user_moderate`,
/// `platform_wallet_contract_document_*` and
/// `platform_wallet_contract_team_action_approve` functions write, decoded
/// exactly as the `ManagedPlatformWallet` wrappers decode it, and for the
/// action byte a moderation of an identity takes.
///
/// The payloads are the shapes the FFI's
/// `packages/rs-platform-wallet-ffi/src/contract_user_moderation.rs` tests
/// pin. No wallet handle, no network.
final class ContractUserModerationDecodingTests: XCTestCase {
    private let id1 = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
    private let id2 = "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR"
    private let id3 = "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8"
    private let id5 = "LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY"

    /// The bytes must agree with `CONTRACT_USER_MODERATION_BAN` to
    /// `CONTRACT_USER_MODERATION_CLEAR_WARNINGS` in the FFI.
    func testShouldEncodeTheActionsAsTheFFIBytes() {
        XCTAssertEqual(ContractUserModerationAction.allCases.map(\.ffiTag), [0, 1, 2, 3, 4, 5])
        XCTAssertEqual(
            ContractUserModerationAction.allCases.map(\.rawValue),
            ["ban", "unban", "suspend", "unsuspend", "warn", "clearWarnings"]
        )
    }

    func testShouldDecodeWhoMayModerateAndWhatEachMayDo() throws {
        let json = """
        {"contractId":"\(id1)","moderatorsTeam":"electedSeated","identities":[\
        {"identityId":"\(id5)","role":"teamLeader","userActions":["ban","unban"],\
        "documentTypes":[{"documentTypeName":"post","delete":true,"deleteWithinSeconds":86400,\
        "restore":false,"changeableFields":["flag"],\
        "settledDeletion":{"approvals":2,"leader":true,"approversPredateDocument":true}},\
        {"documentTypeName":"note","delete":true,"restore":true,"changeableFields":[]}],\
        "approveTeamActions":true}]}
        """
        let offer = try ContractModerationOffer.decode(fromJSON: json)
        XCTAssertEqual(offer.contractId, id1)
        XCTAssertEqual(offer.moderatorsTeam, .electedSeated)
        let leader = try XCTUnwrap(offer.identities.first)
        XCTAssertEqual(leader.identityId, id5)
        XCTAssertEqual(leader.role, .teamLeader)
        XCTAssertEqual(leader.userActions, [.ban, .unban])
        XCTAssertTrue(leader.approveTeamActions)
        let post = try XCTUnwrap(leader.documentType(named: "post"))
        XCTAssertTrue(post.delete)
        XCTAssertEqual(post.deleteWithinSeconds, 86400)
        XCTAssertFalse(post.restore)
        XCTAssertEqual(post.changeableFields, ["flag"])
        XCTAssertEqual(
            post.settledDeletion,
            ContractSettledDeletionRule(approvals: 2, leader: true, approversPredateDocument: true)
        )
        let note = try XCTUnwrap(leader.documentType(named: "note"))
        XCTAssertNil(note.deleteWithinSeconds)
        XCTAssertNil(note.settledDeletion)
        XCTAssertNil(leader.documentType(named: "comment"))

        for role in ["owner", "appointedModerator", "interimModerator", "teamMember"] {
            let one = try ContractModerationOffer.decode(fromJSON: """
            {"contractId":"\(id1)","moderatorsTeam":"declared","identities":[\
            {"identityId":"\(id2)","role":"\(role)","userActions":[],"documentTypes":[],\
            "approveTeamActions":false}]}
            """)
            XCTAssertEqual(one.identities.first?.role.rawValue, role)
        }
    }

    /// A target's status is in the status read's shape.
    func testShouldDecodeAModeratedStatusWithTheStatusReadType() throws {
        let json = """
        {"contractId":"\(id1)","identityId":"\(id2)","status":{"banned":true,\
        "banReason":{"code":null,"text":"spam"},"lists":["banlist"]}}
        """
        let moderated = try ModeratedContractUser.decode(fromJSON: json)
        XCTAssertEqual(moderated.contractId, id1)
        XCTAssertEqual(moderated.identityId, id2)
        XCTAssertEqual(moderated.status.lists, [.banlist])
        XCTAssertEqual(moderated.status.banned, true)
        XCTAssertEqual(moderated.status.banReason?.text, "spam")
        XCTAssertNil(moderated.status.suspendedUntilMs)
    }

    private var removalJSON: String {
        """
        {"documentId":"\(id3)","documentOwnerId":"\(id2)","moderatorId":"\(id5)",\
        "reason":{"code":null,"text":"off topic"},"removedAt":1700000000000,\
        "documentHash":"\(String(repeating: "ab", count: 32))",\
        "keptFields":{"hashtag":{"type":"string","value":"dash"}}}
        """
    }

    func testShouldDecodeADeletionWithItsRecordAndTheDocumentBytes() throws {
        let json = """
        {"contractId":"\(id1)","documentTypeName":"post","documentId":"\(id3)",\
        "removal":\(removalJSON),"documentBytes":"AQID"}
        """
        let deletion = try ContractDocumentDeletion.decode(fromJSON: json)
        XCTAssertEqual(deletion.documentTypeName, "post")
        XCTAssertEqual(deletion.documentId, id3)
        XCTAssertEqual(deletion.removal?.moderatorId, id5)
        XCTAssertEqual(deletion.removal?.keptFields["hashtag"], .string("dash"))
        XCTAssertEqual(deletion.documentData, Data([1, 2, 3]))

        let unrecorded = try ContractDocumentDeletion.decode(fromJSON: """
        {"contractId":"\(id1)","documentTypeName":"post","documentId":"\(id3)"}
        """)
        XCTAssertNil(unrecorded.removal)
        XCTAssertNil(unrecorded.documentData)
    }

    func testShouldDecodeARestoreWithTheRecordMarkedRestored() throws {
        let restored = removalJSON.replacingOccurrences(
            of: "\"keptFields\"",
            with: "\"restoredBy\":\"\(id5)\",\"restoredAt\":1700000100000,\"keptFields\""
        )
        let json = """
        {"contractId":"\(id1)","documentTypeName":"post","documentId":"\(id3)","removal":\(restored)}
        """
        let restore = try ContractDocumentRestore.decode(fromJSON: json)
        XCTAssertEqual(restore.removal.restoredBy, id5)
        XCTAssertEqual(restore.removal.restoredAtMs, 1_700_000_100_000)
    }

    func testShouldDecodeTheChangedDocumentAndKeepItsJSON() throws {
        let json = """
        {"contractId":"\(id1)","documentTypeName":"post",\
        "document":{"$id":"\(id3)","$ownerId":"\(id2)","$revision":2,"flag":"hidden"}}
        """
        let change = try ContractDocumentFieldsChange.decode(fromJSON: json)
        XCTAssertEqual(change.contractId, id1)
        XCTAssertEqual(change.documentId, id3)
        XCTAssertEqual(change.revision, 2)
        let document = try XCTUnwrap(
            JSONSerialization.jsonObject(with: Data(change.documentJSON.utf8)) as? [String: Any]
        )
        XCTAssertEqual(document["flag"] as? String, "hidden")
        XCTAssertThrowsError(try ContractDocumentFieldsChange.decode(fromJSON: "{\"contractId\":\"x\"}"))
    }

    func testShouldDecodeATeamActionSignature() throws {
        for status in ContractTeamActionStatus.allCases {
            let signature = try ContractTeamActionSignature.decode(fromJSON: """
            {"contractId":"\(id1)","actionId":"\(id3)","status":"\(status.rawValue)"}
            """)
            XCTAssertEqual(signature.actionId, id3)
            XCTAssertEqual(signature.status, status)
        }
        XCTAssertThrowsError(try ContractTeamActionSignature.decode(fromJSON: """
        {"contractId":"\(id1)","actionId":"\(id3)","status":"pending"}
        """))
    }
}
