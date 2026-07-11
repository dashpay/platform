import XCTest
@testable import SwiftDashSDK

class IdentityManagerTests: XCTestCase {

    var identityManager: IdentityManager!

    override func setUpWithError() throws {
        try super.setUpWithError()
        identityManager = try IdentityManager.create()
    }

    // MARK: - Identity Getters Tests

    func testInitialIdentityGetters() throws {
        let count = try identityManager.getIdentityCount()
        XCTAssertEqual(count, 0, "New manager should have 0 identities")

        let ids = try identityManager.getAllIdentityIds()
        XCTAssertEqual(ids.count, 0, "New manager should have no identities")
    }
}
