import XCTest

@testable import SwiftDashSDK

/// The confirmed service values reach the update-service externs in the
/// shape Rust reads them: a null node id and zero ports for a regular
/// masternode, the 20-byte node id and both ports for an evonode.
final class MasternodeServiceConfirmationTests: XCTestCase {
    func testShouldPassNoPlatformFieldsForARegularMasternode() throws {
        let service = MasternodeServiceConfirmation(serviceAddress: "203.0.113.7:9999")
        try service.validateShape()

        let (address, hasNodeId, p2pPort, httpPort) = service.withFFIArguments {
            cAddress, nodeId, p2pPort, httpPort in
            (String(cString: cAddress), nodeId != nil, p2pPort, httpPort)
        }
        XCTAssertEqual(address, "203.0.113.7:9999")
        XCTAssertFalse(hasNodeId, "a null node id is what marks a regular masternode")
        XCTAssertEqual(p2pPort, 0)
        XCTAssertEqual(httpPort, 0)
    }

    func testShouldPassEveryConfirmedFieldForAnEvonode() throws {
        let nodeId = Data(repeating: 0x5A, count: 20)
        let service = MasternodeServiceConfirmation(
            serviceAddress: "203.0.113.8:9999",
            evonode: EvonodePlatformService(
                platformNodeId: nodeId, platformP2PPort: 26656, platformHTTPPort: 443))
        try service.validateShape()

        let (address, passedNodeId, p2pPort, httpPort) = service.withFFIArguments {
            cAddress, nodeIdPtr, p2pPort, httpPort in
            (
                String(cString: cAddress),
                nodeIdPtr.map { Data(bytes: $0, count: 20) },
                p2pPort,
                httpPort
            )
        }
        XCTAssertEqual(address, "203.0.113.8:9999")
        XCTAssertEqual(passedNodeId, nodeId)
        XCTAssertEqual(p2pPort, 26656)
        XCTAssertEqual(httpPort, 443)
    }

    /// An empty `Data` would reach Rust as a null pointer, which reads as a
    /// regular masternode, so a short node id is refused on the Swift side.
    func testShouldRefuseAnEvonodeNodeIdThatIsNotTwentyBytes() {
        for length in [0, 19, 21] {
            let service = MasternodeServiceConfirmation(
                serviceAddress: "203.0.113.8:9999",
                evonode: EvonodePlatformService(
                    platformNodeId: Data(repeating: 1, count: length),
                    platformP2PPort: 26656,
                    platformHTTPPort: 443))
            XCTAssertThrowsError(try service.validateShape(), "length \(length)")
        }
    }
}
