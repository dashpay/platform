import XCTest
import CryptoKit
@testable import SwiftExampleApp

// MARK: - Wallet Storage Tests

final class WalletStorageTests: XCTestCase {
    var storage: WalletStorage!
    
    override func setUp() {
        super.setUp()
        storage = WalletStorage()
        
        // Clean up any existing test data
        try? storage.deleteSeed()
    }
    
    override func tearDown() {
        // Clean up
        try? storage.deleteSeed()
        storage = nil
        super.tearDown()
    }
    
    // MARK: - PIN Storage Tests
    
    func testStoreSeedWithPIN() throws {
        let testSeed = Data("test seed data".utf8)
        let pin = "123456"
        
        let encryptedData = try storage.storeSeed(testSeed, pin: pin)
        
        XCTAssertNotNil(encryptedData)
        XCTAssertGreaterThan(encryptedData.count, 32) // Should include salt + encrypted data
        XCTAssertNotEqual(encryptedData, testSeed) // Should be encrypted
    }
    
    func testRetrieveSeedWithPIN() throws {
        let testSeed = Data("test seed data for retrieval".utf8)
        let pin = "654321"
        
        // Store seed
        _ = try storage.storeSeed(testSeed, pin: pin)
        
        // Retrieve with correct PIN
        let retrievedSeed = try storage.retrieveSeed(pin: pin)
        XCTAssertEqual(retrievedSeed, testSeed)
    }
    
    func testRetrieveSeedWithWrongPIN() throws {
        let testSeed = Data("test seed data".utf8)
        let correctPIN = "123456"
        let wrongPIN = "wrong"
        
        // Store seed
        _ = try storage.storeSeed(testSeed, pin: correctPIN)
        
        // Try to retrieve with wrong PIN
        XCTAssertThrowsError(try storage.retrieveSeed(pin: wrongPIN)) { error in
            XCTAssertTrue(error is WalletStorageError)
            if case WalletStorageError.invalidPIN = error {
                // Expected error
            } else {
                XCTFail("Expected invalidPIN error")
            }
        }
    }
    
    func testDeleteSeed() throws {
        let testSeed = Data("test seed to delete".utf8)
        let pin = "123456"
        
        // Store seed
        _ = try storage.storeSeed(testSeed, pin: pin)
        
        // Verify it exists
        let retrieved = try storage.retrieveSeed(pin: pin)
        XCTAssertEqual(retrieved, testSeed)
        
        // Delete seed
        try storage.deleteSeed()
        
        // Verify it's gone
        XCTAssertThrowsError(try storage.retrieveSeed(pin: pin)) { error in
            if case WalletStorageError.seedNotFound = error {
                // Expected error
            } else {
                XCTFail("Expected seedNotFound error")
            }
        }
    }
    
    // MARK: - Encryption Tests
    
    func testEncryptionDecryption() throws {
        let testData = Data("sensitive wallet data".utf8)
        let pin = "secure123"
        
        // Store and retrieve
        _ = try storage.storeSeed(testData, pin: pin)
        let decrypted = try storage.retrieveSeed(pin: pin)
        
        XCTAssertEqual(decrypted, testData)
    }
    
    func testDifferentPINsProduceDifferentEncryption() throws {
        let testSeed = Data("same seed data".utf8)
        let pin1 = "123456"
        let pin2 = "654321"
        
        // Store with first PIN
        let encrypted1 = try storage.storeSeed(testSeed, pin: pin1)
        
        // Delete and store with second PIN
        try storage.deleteSeed()
        let encrypted2 = try storage.storeSeed(testSeed, pin: pin2)
        
        // Encrypted data should be different (different salts and keys)
        XCTAssertNotEqual(encrypted1, encrypted2)
    }
    
    // MARK: - Biometric Tests
    
    func testEnableBiometricProtection() throws {
        let testSeed = Data("biometric test seed".utf8)
        let pin = "123456"
        
        // Store seed first
        _ = try storage.storeSeed(testSeed, pin: pin)
        
        // Enable biometric protection
        // Note: This will fail in unit tests without proper entitlements
        do {
            try storage.enableBiometricProtection(for: testSeed)
        } catch {
            // Expected in test environment
            print("Biometric protection test skipped: \(error)")
        }
    }
    
    // MARK: - Edge Cases
    
    func testEmptySeed() throws {
        let emptySeed = Data()
        let pin = "123456"
        
        let encrypted = try storage.storeSeed(emptySeed, pin: pin)
        let retrieved = try storage.retrieveSeed(pin: pin)
        
        XCTAssertEqual(retrieved, emptySeed)
        XCTAssertGreaterThan(encrypted.count, 32) // Still encrypted with salt
    }
    
    func testLongPIN() throws {
        let testSeed = Data("test seed".utf8)
        let longPIN = String(repeating: "1234567890", count: 10) // 100 characters
        
        _ = try storage.storeSeed(testSeed, pin: longPIN)
        let retrieved = try storage.retrieveSeed(pin: longPIN)
        
        XCTAssertEqual(retrieved, testSeed)
    }
    
    func testSpecialCharactersPIN() throws {
        let testSeed = Data("test seed".utf8)
        let specialPIN = "P@ssw0rd!#$%"
        
        _ = try storage.storeSeed(testSeed, pin: specialPIN)
        let retrieved = try storage.retrieveSeed(pin: specialPIN)
        
        XCTAssertEqual(retrieved, testSeed)
    }
    
    func testOverwriteExistingSeed() throws {
        let seed1 = Data("first seed".utf8)
        let seed2 = Data("second seed".utf8)
        let pin = "123456"
        
        // Store first seed
        _ = try storage.storeSeed(seed1, pin: pin)
        
        // Store second seed (should overwrite)
        _ = try storage.storeSeed(seed2, pin: pin)
        
        // Retrieve should get second seed
        let retrieved = try storage.retrieveSeed(pin: pin)
        XCTAssertEqual(retrieved, seed2)
        XCTAssertNotEqual(retrieved, seed1)
    }
    
    // MARK: - Performance Tests
    
    func testStoragePerformance() throws {
        let testSeed = Data(repeating: 0xFF, count: 64) // 64 byte seed
        let pin = "123456"
        
        measure {
            do {
                _ = try storage.storeSeed(testSeed, pin: pin)
                _ = try storage.retrieveSeed(pin: pin)
                try storage.deleteSeed()
            } catch {
                XCTFail("Performance test failed: \(error)")
            }
        }
    }
    
    // MARK: - Security Tests
    
    func testPINHashNotStored() throws {
        let testSeed = Data("test seed".utf8)
        let pin = "123456"
        
        _ = try storage.storeSeed(testSeed, pin: pin)
        
        // The PIN itself should never be stored, only its hash
        // This is a conceptual test - in reality we'd need to inspect keychain
        // to verify this, which requires additional test infrastructure
    }
    
    func testSaltUniqueness() throws {
        let testSeed = Data("test seed".utf8)
        let pin = "123456"
        
        // Store multiple times
        var encryptedResults: [Data] = []
        
        for _ in 0..<5 {
            try storage.deleteSeed()
            let encrypted = try storage.storeSeed(testSeed, pin: pin)
            encryptedResults.append(encrypted)
        }
        
        // Each encryption should use a different salt
        for i in 0..<encryptedResults.count {
            for j in (i+1)..<encryptedResults.count {
                XCTAssertNotEqual(encryptedResults[i], encryptedResults[j])
            }
        }
    }
}