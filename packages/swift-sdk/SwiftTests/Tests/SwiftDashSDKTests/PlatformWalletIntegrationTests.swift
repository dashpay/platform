import XCTest
@testable import SwiftDashSDK

/// Integration tests for Platform Wallet with real identity data and contact flows
/// These tests require the full FFI stack to be built and linked
class PlatformWalletIntegrationTests: XCTestCase {

    var wallet: PlatformWallet!
    var identityManager: IdentityManager!

    override func setUp() {
        super.setUp()

        do {
            // Create a test wallet from mnemonic
            let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
            wallet = try PlatformWallet.fromMnemonic(mnemonic)
            identityManager = try wallet.getIdentityManager(for: .testnet)
        } catch {
            XCTFail("Failed to set up integration test: \(error)")
        }
    }

    // MARK: - Wallet and Identity Manager Integration

    func testWalletToIdentityManagerFlow() throws {
        // Verify we can create wallet and get identity manager
        XCTAssertNotNil(wallet)
        XCTAssertNotNil(identityManager)

        // Verify initial state
        let count = try identityManager.getIdentityCount()
        XCTAssertGreaterThanOrEqual(count, 0, "Should have zero or more identities")

        let ids = try identityManager.getAllIdentityIds()
        XCTAssertEqual(ids.count, count, "ID count should match identity count")
    }

    func testMultipleNetworkIdentityManagers() throws {
        let mainnetManager = try wallet.getIdentityManager(for: .mainnet)
        let testnetManager = try wallet.getIdentityManager(for: .testnet)
        let devnetManager = try wallet.getIdentityManager(for: .devnet)

        XCTAssertNotEqual(mainnetManager.handle, testnetManager.handle)
        XCTAssertNotEqual(testnetManager.handle, devnetManager.handle)
        XCTAssertNotEqual(mainnetManager.handle, devnetManager.handle)
    }

    // MARK: - Contact Request Flow Integration

    func testContactRequestCreationAndRetrieval() throws {
        let senderId = try Identifier.random()
        let recipientId = try Identifier.random()
        let encryptedKey = Data(count: 65) // Mock encrypted key
        let timestamp = UInt64(Date().timeIntervalSince1970 * 1000)

        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedKey,
            createdAt: timestamp
        )

        // Verify all fields roundtrip correctly
        let retrievedSenderId = try contactRequest.getSenderId()
        let retrievedRecipientId = try contactRequest.getRecipientId()
        let retrievedKey = try contactRequest.getEncryptedPublicKey()
        let retrievedTimestamp = try contactRequest.getCreatedAt()

        XCTAssertEqual(retrievedSenderId.bytes, senderId.bytes)
        XCTAssertEqual(retrievedRecipientId.bytes, recipientId.bytes)
        XCTAssertEqual(retrievedKey, encryptedKey)
        XCTAssertEqual(retrievedTimestamp, timestamp)
    }

    // MARK: - BlockTime Integration

    func testBlockTimeRoundTrip() throws {
        let originalBlockTime = BlockTime(
            height: 1000,
            coreHeight: 2000,
            timestamp: 1234567890
        )

        let ffiBlockTime = originalBlockTime.ffiValue
        let convertedBlockTime = BlockTime(ffiBlockTime: ffiBlockTime)

        XCTAssertEqual(convertedBlockTime.height, originalBlockTime.height)
        XCTAssertEqual(convertedBlockTime.coreHeight, originalBlockTime.coreHeight)
        XCTAssertEqual(convertedBlockTime.timestamp, originalBlockTime.timestamp)
    }

    // MARK: - Identifier Integration

    func testIdentifierRandomnessAndUniqueness() throws {
        var identifiers = Set<String>()

        // Generate 100 random identifiers
        for _ in 0..<100 {
            let identifier = try Identifier.random()
            let hexString = identifier.hexString

            XCTAssertEqual(hexString.count, 64, "Hex string should be 64 characters (32 bytes)")
            XCTAssertFalse(identifiers.contains(hexString), "Should generate unique identifiers")

            identifiers.insert(hexString)
        }

        XCTAssertEqual(identifiers.count, 100, "All identifiers should be unique")
    }

    func testIdentifierHexConversions() throws {
        // Test various hex patterns
        let testCases: [String] = [
            "0000000000000000000000000000000000000000000000000000000000000000",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        ]

        for hexString in testCases {
            let identifier = try Identifier(hexString: hexString)
            let roundTrip = identifier.hexString

            XCTAssertEqual(roundTrip, hexString, "Hex string should round-trip correctly")
        }
    }

    // MARK: - Memory Management Stress Tests

    func testWalletCreationStressTest() throws {
        // Create and destroy many wallets to test memory management
        for i in 0..<100 {
            let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
            let wallet = try PlatformWallet.fromMnemonic(mnemonic, passphrase: "test\(i)")
            let manager = try wallet.getIdentityManager(for: .testnet)
            let count = try manager.getIdentityCount()

            XCTAssertGreaterThanOrEqual(count, 0)
        }

        // If this completes without crashing, memory management is working
        XCTAssertTrue(true, "Stress test completed")
    }

    func testIdentifierCreationStressTest() throws {
        // Create many identifiers to test memory management
        var identifiers: [Identifier] = []

        for _ in 0..<1000 {
            let identifier = try Identifier.random()
            identifiers.append(identifier)
        }

        XCTAssertEqual(identifiers.count, 1000)

        // Verify all are unique
        let uniqueHexStrings = Set(identifiers.map { $0.hexString })
        XCTAssertEqual(uniqueHexStrings.count, 1000, "All identifiers should be unique")
    }

    func testContactRequestStressTest() throws {
        // Create many contact requests to test memory management
        let senderId = try Identifier.random()
        let recipientId = try Identifier.random()
        let encryptedKey = Data(count: 65)

        for i in 0..<100 {
            let request = try ContactRequest.create(
                senderId: senderId,
                recipientId: recipientId,
                senderKeyIndex: UInt32(i),
                recipientKeyIndex: 0,
                accountReference: 0,
                encryptedPublicKey: encryptedKey,
                createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
            )

            let keyIndex = try request.getSenderKeyIndex()
            XCTAssertEqual(keyIndex, UInt32(i))
        }

        XCTAssertTrue(true, "Stress test completed")
    }

    // MARK: - Error Handling Integration

    func testWalletCreationErrorHandling() {
        // Test invalid mnemonic
        XCTAssertThrowsError(try PlatformWallet.fromMnemonic("invalid mnemonic phrase")) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }

        // Test invalid seed size
        let invalidSeed = Data(count: 10)
        XCTAssertThrowsError(try PlatformWallet.fromSeed(invalidSeed)) { error in
            if case PlatformWalletError.invalidParameter = error {
                // Expected
            } else {
                XCTFail("Expected invalidParameter error, got \(error)")
            }
        }
    }

    func testIdentityManagerErrorHandling() throws {
        let manager = try IdentityManager.create()
        let nonExistentId = try Identifier.random()

        // Test getting non-existent identity
        XCTAssertThrowsError(try manager.getIdentity(nonExistentId)) { error in
            if case PlatformWalletError.identityNotFound = error {
                // Expected
            } else {
                XCTFail("Expected identityNotFound error, got \(error)")
            }
        }

        // Test removing non-existent identity
        XCTAssertThrowsError(try manager.removeIdentity(nonExistentId)) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }

        // Test setting non-existent identity as primary
        XCTAssertThrowsError(try manager.setPrimaryIdentity(nonExistentId)) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }
    }

    // MARK: - Thread Safety Tests (if applicable)

    func testConcurrentWalletCreation() throws {
        let expectation = self.expectation(description: "Concurrent wallet creation")
        expectation.expectedFulfillmentCount = 10

        let queue = DispatchQueue(label: "test.concurrent", attributes: .concurrent)

        for i in 0..<10 {
            queue.async {
                do {
                    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
                    let wallet = try PlatformWallet.fromMnemonic(mnemonic, passphrase: "test\(i)")
                    let manager = try wallet.getIdentityManager(for: .testnet)
                    _ = try manager.getIdentityCount()
                    expectation.fulfill()
                } catch {
                    XCTFail("Concurrent creation failed: \(error)")
                }
            }
        }

        waitForExpectations(timeout: 10.0)
    }

    func testConcurrentIdentifierGeneration() throws {
        let expectation = self.expectation(description: "Concurrent identifier generation")
        expectation.expectedFulfillmentCount = 100

        let queue = DispatchQueue(label: "test.concurrent", attributes: .concurrent)
        let identifiersLock = NSLock()
        var identifiers: [String] = []

        for _ in 0..<100 {
            queue.async {
                do {
                    let identifier = try Identifier.random()
                    identifiersLock.lock()
                    identifiers.append(identifier.hexString)
                    identifiersLock.unlock()
                    expectation.fulfill()
                } catch {
                    XCTFail("Concurrent generation failed: \(error)")
                }
            }
        }

        waitForExpectations(timeout: 10.0)

        // Verify uniqueness
        let uniqueIdentifiers = Set(identifiers)
        XCTAssertEqual(uniqueIdentifiers.count, 100, "All concurrently generated identifiers should be unique")
    }
}

// MARK: - Integration Test Notes

/*
 These integration tests verify:

 1. Full roundtrip of wallet creation and identity management
 2. Proper memory management under stress
 3. Error handling across FFI boundary
 4. Thread safety of concurrent operations
 5. Data integrity through FFI conversions

 Additional tests that require Platform SDK connection:
 - Creating real identities on testnet
 - Sending and accepting actual contact requests
 - Testing established contact metadata persistence
 - Verifying balance tracking
 - Testing key synchronization

 For full end-to-end testing, see SwiftExampleApp which demonstrates:
 - Real wallet and identity lifecycle
 - Contact request bidirectional flow
 - Established contact management
 - Integration with Core wallet for funding
 */
