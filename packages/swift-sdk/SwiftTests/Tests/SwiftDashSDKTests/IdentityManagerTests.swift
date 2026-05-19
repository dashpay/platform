import XCTest
@testable import SwiftDashSDK

class IdentityManagerTests: XCTestCase {

    var identityManager: IdentityManager!
    var testIdentityId: Identifier!

    override func setUp() {
        super.setUp()

        do {
            identityManager = try IdentityManager.create()
            testIdentityId = try Identifier.random()
        } catch {
            XCTFail("Failed to set up test: \(error)")
        }
    }

    // MARK: - Creation Tests

    func testCreateIdentityManager() throws {
        let manager = try IdentityManager.create()
        XCTAssertNotNil(manager, "Should create identity manager")
    }

    // MARK: - Identity Count Tests

    func testInitialIdentityCount() throws {
        let count = try identityManager.getIdentityCount()
        XCTAssertEqual(count, 0, "New manager should have 0 identities")
    }

    // MARK: - Get All Identity IDs Tests

    func testGetAllIdentityIdsEmpty() throws {
        let ids = try identityManager.getAllIdentityIds()
        XCTAssertEqual(ids.count, 0, "New manager should have no identities")
    }

    // MARK: - Primary Identity Tests

    func testGetPrimaryIdentityIdWhenNone() throws {
        let primaryId = try identityManager.getPrimaryIdentityId()
        XCTAssertNil(primaryId, "Should return nil when no primary identity")
    }

    func testSetPrimaryIdentity() throws {
        // This test would require adding an identity first
        // For now, we test that the method exists and can be called
        // In a real scenario, you'd add an identity then set it as primary

        // This should throw an error since identity doesn't exist
        XCTAssertThrowsError(try identityManager.setPrimaryIdentity(testIdentityId)) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }
    }

    // MARK: - Get Identity Tests

    func testGetNonExistentIdentity() throws {
        XCTAssertThrowsError(try identityManager.getIdentity(testIdentityId)) { error in
            XCTAssertTrue(error is PlatformWalletError)
            if case PlatformWalletError.identityNotFound = error {
                // Expected error
            } else {
                XCTFail("Expected identityNotFound error, got \(error)")
            }
        }
    }

    // MARK: - Remove Identity Tests

    func testRemoveNonExistentIdentity() throws {
        // Should handle gracefully or throw appropriate error
        XCTAssertThrowsError(try identityManager.removeIdentity(testIdentityId)) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }
    }

    // MARK: - Memory Management Tests

    func testIdentityManagerDeinit() throws {
        var manager: IdentityManager? = try IdentityManager.create()
        let handle = manager?.handle

        XCTAssertNotNil(handle)

        // Release manager - should call deinit
        manager = nil

        // If this doesn't crash, memory management is working
        XCTAssertNil(manager)
    }

    // MARK: - Integration Tests (would require real identity data)

    // Note: Full integration tests with actual identities would require:
    // 1. Creating a ManagedIdentity from real identity bytes
    // 2. Adding it to the manager
    // 3. Testing retrieval, primary identity setting, etc.
    // These tests are included in the integration test suite
}
