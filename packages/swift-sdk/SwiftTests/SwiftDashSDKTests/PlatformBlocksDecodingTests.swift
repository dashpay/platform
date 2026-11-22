import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the JSON the block and transition status reads write
/// (`packages/rs-sdk-ffi/src/system/queries/blocks.rs` and `transition_status.rs`, whose tests
/// pin the same shapes), decoded with the decoder `SDK.getBlocks` and its siblings use. No SDK
/// handle, no network.
final class PlatformBlocksDecodingTests: XCTestCase {
    private let transactionJSON = """
        {"hash":"0707","code":40100,"executed":false,"gas_used":1200,"result_data":"abcd",\
        "type":"IdentityCreditTransfer","owner_id":"11111111111111111111111111111111"}
        """

    func testShouldDecodeBlocksWithTheirTransactions() throws {
        let json = """
            {"proved":true,"blocks":[{"height":120,"hash":"0909","time_ms":1700000000000,\
            "core_chain_locked_height":2100,"app_hash":"0101","proposer_pro_tx_hash":"0202",\
            "transactions":[\(transactionJSON)]}]}
            """

        let blocks = try JSONDecoder.platformBlocks.decode(PlatformBlocks.self, from: Data(json.utf8))

        XCTAssertTrue(blocks.proved)
        XCTAssertEqual(blocks.blocks.count, 1)
        let block = blocks.blocks[0]
        XCTAssertEqual(block.height, 120)
        XCTAssertEqual(block.timeMs, 1_700_000_000_000)
        XCTAssertEqual(block.coreChainLockedHeight, 2100)
        XCTAssertEqual(block.proposerProTxHash, "0202")
        let transaction = block.transactions[0]
        XCTAssertEqual(transaction.code, 40100)
        XCTAssertFalse(transaction.executed)
        XCTAssertEqual(transaction.gasUsed, 1200)
        XCTAssertEqual(transaction.resultData, "abcd")
        XCTAssertEqual(transaction.type, "IdentityCreditTransfer")
        XCTAssertEqual(transaction.ownerId, "11111111111111111111111111111111")
    }

    func testShouldDecodeATransitionWithoutAnOwner() throws {
        let json = """
            {"proved":false,"height":5,"transactions":[{"hash":"03","code":0,"executed":true,"gas_used":0,\
            "result_data":"","type":"ShieldedTransfer","owner_id":null}]}
            """

        let answer = try JSONDecoder.platformBlocks.decode(OwnerTransactionsInBlock.self, from: Data(json.utf8))

        XCTAssertFalse(answer.proved)
        XCTAssertEqual(answer.height, 5)
        XCTAssertNil(answer.transactions[0].ownerId)
        XCTAssertTrue(answer.transactions[0].executed)
    }

    func testShouldDecodeANodeViewAsUnproved() throws {
        let json = """
            {"proved":false,"status":"committed","height":120,\
            "error":{"code":40100,"message":"insufficient balance","consensus_error":null},\
            "node_height":122}
            """

        let view = try JSONDecoder.platformBlocks.decode(NodeTransitionStatus.self, from: Data(json.utf8))

        XCTAssertFalse(view.proved)
        XCTAssertEqual(view.status, .committed)
        XCTAssertEqual(view.height, 120)
        XCTAssertEqual(view.error?.code, 40100)
        XCTAssertNil(view.error?.consensusError)
        XCTAssertEqual(view.nodeHeight, 122)
    }

    func testShouldDecodeEveryStatusByHash() throws {
        let inBlock = """
            {"proved":true,"status":"in_block","block_height":120,"block_hash":"0909",\
            "block_time_ms":1700000000000,"transaction":\(transactionJSON)}
            """
        let status = try JSONDecoder.platformBlocks.decode(TransitionStatusByHash.self, from: Data(inBlock.utf8))
        XCTAssertTrue(status.proved)
        XCTAssertEqual(status.status, .inBlock)
        XCTAssertEqual(status.blockHeight, 120)
        XCTAssertEqual(status.blockTimeMs, 1_700_000_000_000)
        XCTAssertEqual(status.transaction?.code, 40100)
        XCTAssertEqual(status.transaction?.executed, false)
        XCTAssertNil(status.nodeHeight)

        let unproved: [(String, TransitionStatusByHash.Status)] = [
            (#"{"proved":false,"status":"committed_not_proved","height":120,"error":null,"node_height":122}"#, .committedNotProved),
            (#"{"proved":false,"status":"committed_refuted","height":120,"node_height":122}"#, .committedRefuted),
            (#"{"proved":false,"status":"pending","node_height":122}"#, .pending),
            (#"{"proved":false,"status":"unknown","node_height":null}"#, .unknown),
        ]
        for (json, expected) in unproved {
            let status = try JSONDecoder.platformBlocks.decode(TransitionStatusByHash.self, from: Data(json.utf8))
            XCTAssertFalse(status.proved, json)
            XCTAssertEqual(status.status, expected, json)
            XCTAssertNil(status.transaction, json)
        }
    }

    func testShouldReadExecutedFromTheJSONNotFromTheCode() throws {
        // `executed` is Rust's answer; Swift does not derive it from `code`
        let json = """
            {"hash":"03","code":0,"executed":false,"gas_used":0,"result_data":"",\
            "type":"ShieldedTransfer","owner_id":null}
            """

        let transaction = try JSONDecoder.platformBlocks.decode(PlatformBlockTransaction.self, from: Data(json.utf8))

        XCTAssertFalse(transaction.executed)
    }

    func testShouldRefuseATransactionWithoutExecuted() {
        let json = """
            {"hash":"03","code":0,"gas_used":0,"result_data":"","type":"ShieldedTransfer","owner_id":null}
            """

        XCTAssertThrowsError(
            try JSONDecoder.platformBlocks.decode(PlatformBlockTransaction.self, from: Data(json.utf8))
        )
    }

    func testShouldRefuseAQueryStringHoldingANULBeforeTheFFICall() {
        // The wrappers pass `ownerId` and `transitionHash` through this conversion; a NUL would
        // otherwise make Rust read only the prefix ("valid\0junk" queries "valid")
        var bodyRan = false
        XCTAssertThrowsError(
            try withDecisionProposalCString(
                "valid\0junk", field: "transitionHash", invalid: SDKError.invalidParameter
            ) { _ in bodyRan = true }
        ) { error in
            guard case SDKError.invalidParameter(let message) = error else {
                return XCTFail("unexpected error \(error)")
            }
            XCTAssertTrue(message.contains("transitionHash"), message)
        }
        XCTAssertFalse(bodyRan)
    }
}
