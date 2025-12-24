import XCTest

final class MinimalAsyncTest: XCTestCase {
    
    // Test 1: Simple async test
    func testSimpleAsync() async throws {
        print("Starting simple async test")
        try await Task.sleep(nanoseconds: 100_000_000)
        print("Simple async test completed")
        XCTAssertTrue(true)
    }
    
    // Test 2: Async test with "Transfer" in name
    func testTransferAsync() async throws {
        print("Starting transfer async test")
        try await Task.sleep(nanoseconds: 100_000_000)
        print("Transfer async test completed")
        XCTAssertTrue(true)
    }
    
    // Test 3: Async test with "CreditTransfer" in name
    func testCreditTransferAsync() async throws {
        print("Starting credit transfer async test")
        try await Task.sleep(nanoseconds: 100_000_000)
        print("Credit transfer async test completed")
        XCTAssertTrue(true)
    }
    
    // Test 4: Async test with exact failing name
    func testIdentityCreditTransfer() async throws {
        print("Starting identity credit transfer test")
        try await Task.sleep(nanoseconds: 100_000_000)
        print("Identity credit transfer test completed")
        XCTAssertTrue(true)
    }
}