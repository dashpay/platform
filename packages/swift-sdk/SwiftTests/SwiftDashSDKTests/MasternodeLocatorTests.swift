import XCTest
@testable import SwiftDashSDK

/// Pure marshalling helpers of the masternode locator wrapper: role masks,
/// proTxHash display order, and the DAPI address a match renders.
final class MasternodeLocatorTests: XCTestCase {
    private func match(
        serviceAddress: String?,
        platformHTTPPort: UInt16?,
        matchedKeys: [MasternodeKeyRole] = []
    ) -> MasternodeLocateMatch {
        MasternodeLocateMatch(
            proTxHash: Data((0..<32).map { UInt8($0) }),
            serviceAddress: serviceAddress,
            platformHTTPPort: platformHTTPPort,
            operatorPublicKey: Data(repeating: 0x02, count: 48),
            votingKeyId: Data(repeating: 0x03, count: 20),
            platformNodeId: nil,
            isValid: true,
            isEvonode: platformHTTPPort != nil,
            matchedBy: matchedKeys.isEmpty ? .serviceAddress : .key,
            matchedKeys: matchedKeys,
            inWalletId: nil,
            alreadyTracked: false
        )
    }

    func testRoleMaskDecodesEveryBitInRoleOrder() {
        XCTAssertEqual(MasternodeKeyRole.roles(fromMask: 0), [])
        XCTAssertEqual(MasternodeKeyRole.roles(fromMask: 0b0000_0001), [.owner])
        XCTAssertEqual(MasternodeKeyRole.roles(fromMask: 0b0000_0011), [.owner, .voting])
        XCTAssertEqual(MasternodeKeyRole.roles(fromMask: 0b0011_0100), [.operator, .ownerPayout, .operatorPayout])
        XCTAssertEqual(MasternodeKeyRole.roles(fromMask: 0b0000_1000), [.platformNode])
    }

    func testRoleRawValuesLineUpWithAndroid() {
        XCTAssertEqual(MasternodeKeyRole.owner.rawValue, 0)
        XCTAssertEqual(MasternodeKeyRole.voting.rawValue, 1)
        XCTAssertEqual(MasternodeKeyRole.operator.rawValue, 2)
        XCTAssertEqual(MasternodeKeyRole.platformNode.rawValue, 3)
    }

    func testProTxHashHexIsTheReversedWireBytes() {
        let m = match(serviceAddress: nil, platformHTTPPort: nil)
        XCTAssertTrue(m.proTxHashHex.hasPrefix("1f1e1d1c"))
        XCTAssertTrue(m.proTxHashHex.hasSuffix("03020100"))
        XCTAssertEqual(m.proTxHashHex.count, 64)
    }

    func testServiceHostAndDAPIAddress() {
        let v4 = match(serviceAddress: "1.2.3.4:9999", platformHTTPPort: 443)
        XCTAssertEqual(v4.serviceHost, "1.2.3.4")
        XCTAssertEqual(v4.platformDAPIAddress, "https://1.2.3.4:443")

        let v6 = match(serviceAddress: "[2001:db8::1]:9999", platformHTTPPort: 1443)
        XCTAssertEqual(v6.serviceHost, "[2001:db8::1]")
        XCTAssertEqual(v6.platformDAPIAddress, "https://[2001:db8::1]:1443")

        let regular = match(serviceAddress: "1.2.3.4:9999", platformHTTPPort: nil)
        XCTAssertNil(regular.platformDAPIAddress, "a regular masternode has no DAPI")

        let torOnly = match(serviceAddress: nil, platformHTTPPort: 443)
        XCTAssertNil(torOnly.serviceHost)
        XCTAssertNil(torOnly.platformDAPIAddress)
    }

    func testKeyMatchCarriesItsRoles() {
        let m = match(serviceAddress: "1.2.3.4:9999", platformHTTPPort: nil, matchedKeys: [.voting])
        XCTAssertEqual(m.matchedBy, .key)
        XCTAssertEqual(m.matchedKeys, [.voting])
    }
}
