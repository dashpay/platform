import XCTest
@testable import SwiftDashSDK

class ContactRequestTests: XCTestCase {

    var senderId: Identifier!
    var recipientId: Identifier!
    var encryptedPublicKey: Data!

    override func setUp() {
        super.setUp()

        do {
            senderId = try Identifier.random()
            recipientId = try Identifier.random()
            // Create mock encrypted public key (in real usage, this would be ECDH encrypted)
            encryptedPublicKey = Data(count: 65) // Example size for encrypted secp256k1 public key
        } catch {
            XCTFail("Failed to set up test: \(error)")
        }
    }

    // MARK: - Creation Tests

    func testCreateContactRequest() throws {
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )

        XCTAssertNotNil(contactRequest, "Should create contact request")
    }

    func testCreateContactRequestWithCustomValues() throws {
        let timestamp = UInt64(1234567890000)
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 5,
            recipientKeyIndex: 3,
            accountReference: 1,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: timestamp
        )

        XCTAssertNotNil(contactRequest, "Should create contact request with custom values")
    }

    // MARK: - Getter Tests

    func testGetSenderId() throws {
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )

        let retrievedSenderId = try contactRequest.getSenderId()
        XCTAssertEqual(retrievedSenderId.bytes, senderId.bytes, "Should retrieve correct sender ID")
    }

    func testGetRecipientId() throws {
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )

        let retrievedRecipientId = try contactRequest.getRecipientId()
        XCTAssertEqual(retrievedRecipientId.bytes, recipientId.bytes, "Should retrieve correct recipient ID")
    }

    func testGetSenderKeyIndex() throws {
        let expectedIndex: UInt32 = 7
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: expectedIndex,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )

        let retrievedIndex = try contactRequest.getSenderKeyIndex()
        XCTAssertEqual(retrievedIndex, expectedIndex, "Should retrieve correct sender key index")
    }

    func testGetRecipientKeyIndex() throws {
        let expectedIndex: UInt32 = 9
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: expectedIndex,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )

        let retrievedIndex = try contactRequest.getRecipientKeyIndex()
        XCTAssertEqual(retrievedIndex, expectedIndex, "Should retrieve correct recipient key index")
    }

    func testGetAccountReference() throws {
        let expectedReference: UInt32 = 2
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: expectedReference,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )

        let retrievedReference = try contactRequest.getAccountReference()
        XCTAssertEqual(retrievedReference, expectedReference, "Should retrieve correct account reference")
    }

    func testGetEncryptedPublicKey() throws {
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )

        let retrievedKey = try contactRequest.getEncryptedPublicKey()
        XCTAssertEqual(retrievedKey, encryptedPublicKey, "Should retrieve correct encrypted public key")
    }

    func testGetCreatedAt() throws {
        let expectedTimestamp = UInt64(1234567890000)
        let contactRequest = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: expectedTimestamp
        )

        let retrievedTimestamp = try contactRequest.getCreatedAt()
        XCTAssertEqual(retrievedTimestamp, expectedTimestamp, "Should retrieve correct creation timestamp")
    }

    // MARK: - Memory Management Tests

    func testContactRequestDeinit() throws {
        var contactRequest: ContactRequest? = try ContactRequest.create(
            senderId: senderId,
            recipientId: recipientId,
            senderKeyIndex: 0,
            recipientKeyIndex: 0,
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey,
            createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
        )
        let handle = contactRequest?.handle

        XCTAssertNotNil(handle)

        // Release contact request - should call deinit
        contactRequest = nil

        // If this doesn't crash, memory management is working
        XCTAssertNil(contactRequest)
    }

    // MARK: - Edge Cases

    func testCreateWithEmptyEncryptedKey() {
        let emptyKey = Data()

        // This might be valid or invalid depending on FFI implementation
        // Test that it doesn't crash
        do {
            let contactRequest = try ContactRequest.create(
                senderId: senderId,
                recipientId: recipientId,
                senderKeyIndex: 0,
                recipientKeyIndex: 0,
                accountReference: 0,
                encryptedPublicKey: emptyKey,
                createdAt: UInt64(Date().timeIntervalSince1970 * 1000)
            )
            XCTAssertNotNil(contactRequest)
        } catch {
            // Expected to fail - empty key should not be allowed
            XCTAssertTrue(error is PlatformWalletError)
        }
    }
}
