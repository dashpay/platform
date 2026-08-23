import XCTest
@testable import SwiftDashSDK

/// `EvonodeStatus` decodes the exact JSON shape `dash_sdk_evonode_get_status`
/// emits (see the Rust `evonode_status_json` test fixture), and
/// `PlatformMasternode.platformDAPIAddress` builds the URI the query is sent to.
final class EvonodeStatusTests: XCTestCase {
    /// The Rust FFI's full-fixture JSON, byte-for-byte in shape.
    private static let fullJSON = """
    {"version":{"software":{"dapi":"1.2.3","drive":"4.5.6","tenderdash":"0.14.0-dev.1"},
                "protocol":{"tenderdash":{"p2p":9,"block":12},
                            "drive":{"latest":7,"current":6,"nextEpoch":7}}},
     "node":{"id":"\(String(repeating: "aa", count: 20))","proTxHash":"\(String(repeating: "bb", count: 32))"},
     "chain":{"catchingUp":true,
              "latestBlockHash":"\(String(repeating: "11", count: 32))",
              "latestAppHash":"\(String(repeating: "22", count: 32))",
              "earliestBlockHash":"\(String(repeating: "33", count: 32))",
              "earliestAppHash":"\(String(repeating: "44", count: 32))",
              "latestBlockHeight":5000,"earliestBlockHeight":10,"maxPeerBlockHeight":5001,
              "coreChainLockedHeight":750},
     "network":{"chainId":"dash-mainnet","peersCount":50,"listening":true},
     "stateSync":{"totalSyncedTime":7200,"remainingTime":60,"totalSnapshots":3,
                  "chunkProcessAvgTime":25,"snapshotHeight":4500,"snapshotChunksCount":200,
                  "backfilledBlocks":1000,"backfillBlocksTotal":2000},
     "time":{"local":1700000000000,"block":1699999900000,"genesis":1690000000000,"epoch":42}}
    """

    /// What the FFI emits for `EvoNodeStatus::default()` — every optional
    /// section `null`, required scalars at their zero values.
    private static let omittedJSON = """
    {"version":{"software":null,"protocol":null},
     "node":{"id":"","proTxHash":null},
     "chain":{"catchingUp":false,"latestBlockHash":"","latestAppHash":"",
              "earliestBlockHash":"","earliestAppHash":"",
              "latestBlockHeight":0,"earliestBlockHeight":0,"maxPeerBlockHeight":0,
              "coreChainLockedHeight":null},
     "network":{"chainId":"","peersCount":0,"listening":false},
     "stateSync":{"totalSyncedTime":0,"remainingTime":0,"totalSnapshots":0,
                  "chunkProcessAvgTime":0,"snapshotHeight":0,"snapshotChunksCount":0,
                  "backfilledBlocks":0,"backfillBlocksTotal":0},
     "time":{"local":0,"block":null,"genesis":null,"epoch":null}}
    """

    func testDecodesEveryField() throws {
        let status = try JSONDecoder().decode(EvonodeStatus.self, from: Data(Self.fullJSON.utf8))

        XCTAssertEqual(status.version.software?.dapi, "1.2.3")
        XCTAssertEqual(status.version.software?.drive, "4.5.6")
        XCTAssertEqual(status.version.software?.tenderdash, "0.14.0-dev.1")
        XCTAssertEqual(status.version.protocol?.tenderdash?.p2p, 9)
        XCTAssertEqual(status.version.protocol?.tenderdash?.block, 12)
        XCTAssertEqual(status.version.protocol?.drive?.latest, 7)
        XCTAssertEqual(status.version.protocol?.drive?.current, 6)
        XCTAssertEqual(status.version.protocol?.drive?.nextEpoch, 7)

        XCTAssertEqual(status.node.id, String(repeating: "aa", count: 20))
        XCTAssertEqual(status.node.proTxHash, String(repeating: "bb", count: 32))

        XCTAssertTrue(status.chain.catchingUp)
        XCTAssertEqual(status.chain.latestBlockHash, String(repeating: "11", count: 32))
        XCTAssertEqual(status.chain.latestAppHash, String(repeating: "22", count: 32))
        XCTAssertEqual(status.chain.earliestBlockHash, String(repeating: "33", count: 32))
        XCTAssertEqual(status.chain.earliestAppHash, String(repeating: "44", count: 32))
        XCTAssertEqual(status.chain.latestBlockHeight, 5000)
        XCTAssertEqual(status.chain.earliestBlockHeight, 10)
        XCTAssertEqual(status.chain.maxPeerBlockHeight, 5001)
        XCTAssertEqual(status.chain.coreChainLockedHeight, 750)

        XCTAssertEqual(status.network.chainId, "dash-mainnet")
        XCTAssertEqual(status.network.peersCount, 50)
        XCTAssertTrue(status.network.listening)

        XCTAssertEqual(status.stateSync.totalSyncedTime, 7200)
        XCTAssertEqual(status.stateSync.remainingTime, 60)
        XCTAssertEqual(status.stateSync.totalSnapshots, 3)
        XCTAssertEqual(status.stateSync.chunkProcessAvgTime, 25)
        XCTAssertEqual(status.stateSync.snapshotHeight, 4500)
        XCTAssertEqual(status.stateSync.snapshotChunksCount, 200)
        XCTAssertEqual(status.stateSync.backfilledBlocks, 1000)
        XCTAssertEqual(status.stateSync.backfillBlocksTotal, 2000)

        XCTAssertEqual(status.time.local, 1_700_000_000_000)
        XCTAssertEqual(status.time.block, 1_699_999_900_000)
        XCTAssertEqual(status.time.genesis, 1_690_000_000_000)
        XCTAssertEqual(status.time.epoch, 42)
    }

    func testOmittedOptionalFieldsDecodeAsNil() throws {
        let status = try JSONDecoder().decode(EvonodeStatus.self, from: Data(Self.omittedJSON.utf8))

        XCTAssertNil(status.version.software)
        XCTAssertNil(status.version.protocol)
        XCTAssertNil(status.node.proTxHash)
        XCTAssertNil(status.chain.coreChainLockedHeight)
        XCTAssertNil(status.time.block)
        XCTAssertNil(status.time.genesis)
        XCTAssertNil(status.time.epoch)
        XCTAssertEqual(status.node.id, "")
        XCTAssertEqual(status.time.local, 0)
    }

    // MARK: - Time units

    /// rs-dapi reports `local` in seconds, the legacy JS DAPI in milliseconds;
    /// `block` / `genesis` are always milliseconds and `0` means unknown.
    func testTimeAccessorsResolveUnitsByMagnitude() {
        let rustDapi = EvonodeStatus.Time(local: 1_787_478_228, block: 1_787_478_100_783, genesis: 0, epoch: 79)
        XCTAssertEqual(rustDapi.localDate?.timeIntervalSince1970, 1_787_478_228)
        XCTAssertEqual(rustDapi.blockDate?.timeIntervalSince1970 ?? 0, 1_787_478_100.783, accuracy: 0.001)
        XCTAssertNil(rustDapi.genesisDate, "Drive's 0 genesis is unknown, not 1970")

        let jsDapi = EvonodeStatus.Time(local: 1_700_000_000_000, block: nil, genesis: 1_690_000_000_000, epoch: nil)
        XCTAssertEqual(jsDapi.localDate?.timeIntervalSince1970, 1_700_000_000)
        XCTAssertNil(jsDapi.blockDate)
        XCTAssertEqual(jsDapi.genesisDate?.timeIntervalSince1970, 1_690_000_000)

        XCTAssertNil(EvonodeStatus.Time(local: 0, block: nil, genesis: nil, epoch: nil).localDate)
    }

    // MARK: - PlatformMasternode.platformDAPIAddress

    private func masternode(serviceAddress: String?, platformHTTPPort: UInt16?) -> PlatformMasternode {
        PlatformMasternode(
            proTxHash: Data(repeating: 1, count: 32),
            hasRegistration: true,
            registrationHeight: 1,
            orderIndex: 0,
            typeIndex: 1,
            isEvonode: platformHTTPPort != nil,
            revoked: false,
            revocationReason: 0,
            status: 0,
            txCount: 1,
            collateralTxid: nil,
            collateralVout: 0,
            ownerKeyHash: nil,
            votingKeyHash: nil,
            serviceAddress: serviceAddress,
            platformHTTPPort: platformHTTPPort,
            ownerAddress: nil,
            votingAddress: nil,
            operatorPublicKey: nil,
            platformNodeId: nil,
            payoutAddress: nil,
            operatorPseudoAddress: nil,
            platformNodeAddress: nil,
            operatorInWallet: false,
            operatorAccountType: 0,
            operatorKeyIndex: 0,
            platformInWallet: false,
            platformAccountType: 0,
            platformKeyIndex: 0,
            platformOwnershipChecked: false)
    }

    func testPlatformDAPIAddressDropsTheCorePort() {
        XCTAssertEqual(
            masternode(serviceAddress: "203.0.113.7:9999", platformHTTPPort: 443).platformDAPIAddress,
            "https://203.0.113.7:443")
        XCTAssertEqual(
            masternode(serviceAddress: "203.0.113.7:19999", platformHTTPPort: 1443).platformDAPIAddress,
            "https://203.0.113.7:1443")
    }

    func testPlatformDAPIAddressBracketsIPv6() {
        XCTAssertEqual(
            masternode(serviceAddress: "[2001:db8::1]:9999", platformHTTPPort: 443).platformDAPIAddress,
            "https://[2001:db8::1]:443")
        // ProUpServTx addresses are formatted `ip:port` without brackets.
        XCTAssertEqual(
            masternode(serviceAddress: "2001:db8::1:9999", platformHTTPPort: 443).platformDAPIAddress,
            "https://[2001:db8::1]:443")
    }

    func testPlatformDAPIAddressIsNilWithoutEitherHalf() {
        XCTAssertNil(masternode(serviceAddress: "203.0.113.7:9999", platformHTTPPort: nil).platformDAPIAddress,
                     "regular masternode: no platform port ⇒ no DAPI address")
        XCTAssertNil(masternode(serviceAddress: nil, platformHTTPPort: 443).platformDAPIAddress,
                     "no service address seen ⇒ no host to build from")
    }
}
