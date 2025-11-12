import XCTest
@testable import SwiftDashSDK

final class WalletSerializationTests: XCTestCase {
    
    func testWalletSerializationRoundTrip() throws {
        // Create first manager
        let manager1 = try WalletManager()
        
        // Test mnemonic
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        
        // Add wallet and get serialized bytes
        let (walletId1, serializedWallet) = try manager1.addWalletAndSerialize(
            mnemonic: mnemonic,
            passphrase: nil,
            network: .testnet,
            birthHeight: 0,
            accountOptions: .default,
            downgradeToPublicKeyWallet: false,
            allowExternalSigning: false
        )
        
        // Verify we got a wallet ID
        XCTAssertEqual(walletId1.count, 32, "Wallet ID should be 32 bytes")
        XCTAssertFalse(serializedWallet.isEmpty, "Serialized wallet should not be empty")
        
        // Create second manager
        let manager2 = try WalletManager()
        
        // Import the wallet from serialized bytes
        let walletId2 = try manager2.importWallet(from: serializedWallet)
        
        // Verify the wallet IDs match
        XCTAssertEqual(walletId1, walletId2, "Wallet IDs should match after import")
        
        // Verify both managers have the wallet
        let wallets1 = try manager1.getWalletIds()
        let wallets2 = try manager2.getWalletIds()
        
        XCTAssertTrue(wallets1.contains(walletId1), "Manager 1 should contain the wallet")
        XCTAssertTrue(wallets2.contains(walletId2), "Manager 2 should contain the imported wallet")
        
        // Verify addresses match
        let address1 = try manager1.getReceiveAddress(walletId: walletId1, network: .testnet)
        let address2 = try manager2.getReceiveAddress(walletId: walletId2, network: .testnet)
        
        XCTAssertEqual(address1, address2, "Addresses should match after import")
    }
    
    func testWatchOnlyWalletSerialization() throws {
        let manager = try WalletManager()
        
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        
        // Create a watch-only wallet (downgrade to public key wallet)
        let (walletId, serializedWallet) = try manager.addWalletAndSerialize(
            mnemonic: mnemonic,
            passphrase: nil,
            network: .testnet,
            birthHeight: 100000,
            accountOptions: .default,
            downgradeToPublicKeyWallet: true,
            allowExternalSigning: false
        )
        
        XCTAssertEqual(walletId.count, 32, "Wallet ID should be 32 bytes")
        XCTAssertFalse(serializedWallet.isEmpty, "Serialized wallet should not be empty")
        
        // Import in another manager
        let manager2 = try WalletManager()
        let importedWalletId = try manager2.importWallet(from: serializedWallet)
        
        XCTAssertEqual(walletId, importedWalletId, "Wallet IDs should match")
        
        // Verify we can get addresses (watch-only wallets can still derive addresses)
        let address = try manager2.getReceiveAddress(walletId: importedWalletId, network: .testnet)
        XCTAssertFalse(address.isEmpty, "Should be able to get address from watch-only wallet")
    }
    
    func testExternallySignableWalletSerialization() throws {
        let manager = try WalletManager()
        
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        
        // Create an externally signable wallet
        let (walletId, serializedWallet) = try manager.addWalletAndSerialize(
            mnemonic: mnemonic,
            passphrase: "test-passphrase",
            network: .mainnet,
            birthHeight: 50000,
            accountOptions: .default,
            downgradeToPublicKeyWallet: true,
            allowExternalSigning: true
        )
        
        XCTAssertEqual(walletId.count, 32, "Wallet ID should be 32 bytes")
        XCTAssertFalse(serializedWallet.isEmpty, "Serialized wallet should not be empty")
        
        // Import and verify
        let manager2 = try WalletManager()
        let importedWalletId = try manager2.importWallet(from: serializedWallet)
        
        XCTAssertEqual(walletId, importedWalletId, "Wallet IDs should match")
    }
    
    func testInvalidSerializedBytesImport() throws {
        let manager = try WalletManager()
        
        // Test with empty data
        XCTAssertThrowsError(try manager.importWallet(from: Data())) { error in
            guard let walletError = error as? KeyWalletError else {
                XCTFail("Expected KeyWalletError")
                return
            }
            
            switch walletError {
            case .invalidInput(let message):
                XCTAssertEqual(message, "Wallet bytes cannot be empty")
            default:
                XCTFail("Expected invalidInput error")
            }
        }
        
        // Test with invalid data
        let invalidData = Data([0x00, 0x01, 0x02, 0x03])
        XCTAssertThrowsError(try manager.importWallet(from: invalidData)) { error in
            // Should throw an error when trying to deserialize invalid data
            XCTAssertNotNil(error)
        }
    }
    
    func testMultipleWalletsSerialization() throws {
        let manager = try WalletManager()
        
        let mnemonics = [
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            "all all all all all all all all all all all all"
        ]
        
        var serializedWallets: [(walletId: Data, serialized: Data)] = []
        
        // Create multiple wallets and serialize them
        for mnemonic in mnemonics {
            let (walletId, serialized) = try manager.addWalletAndSerialize(
                mnemonic: mnemonic,
                network: .testnet
            )
            serializedWallets.append((walletId: walletId, serialized: serialized))
        }
        
        // Create new manager and import all wallets
        let manager2 = try WalletManager()
        
        for (originalId, serializedData) in serializedWallets {
            let importedId = try manager2.importWallet(from: serializedData)
            XCTAssertEqual(originalId, importedId, "Wallet IDs should match after import")
        }
        
        // Verify all wallets were imported
        let importedWalletIds = try manager2.getWalletIds()
        XCTAssertEqual(importedWalletIds.count, mnemonics.count, "Should have imported all wallets")
        
        for (originalId, _) in serializedWallets {
            XCTAssertTrue(importedWalletIds.contains(originalId), "Should contain wallet \(originalId.hexEncodedString())")
        }
    }
}

// Helper extension for hex encoding
private extension Data {
    func hexEncodedString() -> String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}