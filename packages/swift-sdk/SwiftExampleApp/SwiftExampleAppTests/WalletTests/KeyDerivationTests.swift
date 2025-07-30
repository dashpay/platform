import XCTest
@testable import SwiftExampleApp

// MARK: - Key Derivation Tests

final class KeyDerivationTests: XCTestCase {
    
    // MARK: - Mnemonic Tests
    
    func testMnemonicGeneration() {
        let mnemonic = CoreSDKWrapper.shared.generateMnemonic()
        
        XCTAssertNotNil(mnemonic)
        
        // Check word count (12 words by default)
        let words = mnemonic?.split(separator: " ")
        XCTAssertEqual(words?.count, 12)
    }
    
    func testMnemonicValidation() {
        // Valid mnemonic
        let validMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        XCTAssertTrue(CoreSDKWrapper.shared.validateMnemonic(validMnemonic))
        
        // Invalid mnemonic (wrong word)
        let invalidMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon invalid"
        XCTAssertFalse(CoreSDKWrapper.shared.validateMnemonic(invalidMnemonic))
        
        // Invalid mnemonic (wrong count)
        let shortMnemonic = "abandon abandon abandon"
        XCTAssertFalse(CoreSDKWrapper.shared.validateMnemonic(shortMnemonic))
    }
    
    func testMnemonicToSeed() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        
        let seed = CoreSDKWrapper.shared.mnemonicToSeed(mnemonic)
        XCTAssertNotNil(seed)
        XCTAssertEqual(seed?.count, 64) // 512 bits
        
        // Test with passphrase
        let seedWithPassphrase = CoreSDKWrapper.shared.mnemonicToSeed(mnemonic, passphrase: "TREZOR")
        XCTAssertNotNil(seedWithPassphrase)
        XCTAssertNotEqual(seed, seedWithPassphrase) // Different seeds
    }
    
    // MARK: - Derivation Path Tests
    
    func testDerivationPathBIP44() {
        let path = DerivationPath.dashBIP44(account: 0, change: 0, index: 0, testnet: false)
        XCTAssertEqual(path.stringRepresentation, "m/44'/5'/0'/0/0")
        
        let testnetPath = DerivationPath.dashBIP44(account: 0, change: 0, index: 0, testnet: true)
        XCTAssertEqual(testnetPath.stringRepresentation, "m/44'/1'/0'/0/0")
        
        let accountPath = DerivationPath.dashBIP44(account: 1, change: 1, index: 5, testnet: false)
        XCTAssertEqual(accountPath.stringRepresentation, "m/44'/5'/1'/1/5")
    }
    
    func testDerivationPathCoinJoin() {
        let path = DerivationPath.coinJoin(account: 0, change: 0, index: 0, testnet: false)
        XCTAssertEqual(path.stringRepresentation, "m/9'/5'/0'/0/0")
        
        let testnetPath = DerivationPath.coinJoin(account: 0, change: 0, index: 0, testnet: true)
        XCTAssertEqual(testnetPath.stringRepresentation, "m/9'/1'/0'/0/0")
    }
    
    func testDerivationPathDIP13Identity() {
        let path = DerivationPath.dip13Identity(
            account: 0,
            identityIndex: 0,
            keyType: .authentication,
            keyIndex: 0,
            testnet: false
        )
        XCTAssertEqual(path.stringRepresentation, "m/13'/5'/0'/0'/0/0")
        
        let masterPath = DerivationPath.dip13Identity(
            account: 0,
            identityIndex: 1,
            keyType: .master,
            keyIndex: 0,
            testnet: false
        )
        XCTAssertEqual(masterPath.stringRepresentation, "m/13'/5'/0'/1'/1/0")
        
        let topupPath = DerivationPath.dip13Identity(
            account: 0,
            identityIndex: 0,
            keyType: .topup,
            keyIndex: 5,
            testnet: false
        )
        XCTAssertEqual(topupPath.stringRepresentation, "m/13'/5'/0'/0'/2/5")
    }
    
    func testDerivationPathParsing() {
        // Test parsing valid path
        if let path = DerivationPath.parse("m/44'/5'/0'/0/0") {
            XCTAssertEqual(path.indexes, [2147483692, 2147483653, 2147483648, 0, 0])
            XCTAssertEqual(path.stringRepresentation, "m/44'/5'/0'/0/0")
        } else {
            XCTFail("Failed to parse valid path")
        }
        
        // Test invalid paths
        XCTAssertNil(DerivationPath.parse("invalid"))
        XCTAssertNil(DerivationPath.parse("44'/5'/0'/0/0")) // Missing 'm/'
        XCTAssertNil(DerivationPath.parse("m/")) // Empty path
    }
    
    // MARK: - Key Derivation Tests
    
    func testKeyDerivation() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        guard let seed = CoreSDKWrapper.shared.mnemonicToSeed(mnemonic) else {
            XCTFail("Failed to generate seed")
            return
        }
        
        // Test master key derivation
        let masterKey = HDKeyDerivation.masterKey(from: seed, network: .testnet)
        XCTAssertNotNil(masterKey)
        
        // Test derived key
        let path = DerivationPath.dashBIP44(account: 0, change: 0, index: 0, testnet: true)
        let derivedKey = HDKeyDerivation.deriveKey(seed: seed, path: path, network: .testnet)
        XCTAssertNotNil(derivedKey)
        
        // Verify we get consistent results
        let derivedKey2 = HDKeyDerivation.deriveKey(seed: seed, path: path, network: .testnet)
        XCTAssertEqual(derivedKey?.privateKey, derivedKey2?.privateKey)
        XCTAssertEqual(derivedKey?.publicKey, derivedKey2?.publicKey)
    }
    
    func testAddressGeneration() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        guard let seed = CoreSDKWrapper.shared.mnemonicToSeed(mnemonic) else {
            XCTFail("Failed to generate seed")
            return
        }
        
        let path = DerivationPath.dashBIP44(account: 0, change: 0, index: 0, testnet: true)
        guard let derivedKey = HDKeyDerivation.deriveKey(seed: seed, path: path, network: .testnet) else {
            XCTFail("Failed to derive key")
            return
        }
        
        // Test address generation
        let address = HDKeyDerivation.addressFromPublicKey(derivedKey.publicKey, network: .testnet)
        XCTAssertNotNil(address)
        XCTAssertTrue(address?.starts(with: "y") ?? false) // Testnet addresses start with 'y'
    }
    
    // MARK: - FFI Bridge Tests
    
    func testFFIBridgeKeyDerivation() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        guard let seed = CoreSDKWrapper.shared.mnemonicToSeed(mnemonic) else {
            XCTFail("Failed to generate seed")
            return
        }
        
        let bridge = WalletFFIBridge.shared
        
        // Test key derivation through FFI
        let path = "m/44'/1'/0'/0/0" // Testnet path
        let derivedKey = bridge.deriveKey(seed: seed, path: path, network: .testnet)
        
        XCTAssertNotNil(derivedKey)
        XCTAssertEqual(derivedKey?.privateKey.count, 32)
        XCTAssertEqual(derivedKey?.publicKey.count, 33)
        
        // Test address generation
        if let pubKey = derivedKey?.publicKey {
            let address = bridge.addressFromPublicKey(pubKey, network: .testnet)
            XCTAssertNotNil(address)
            XCTAssertTrue(address?.starts(with: "y") ?? false)
        }
    }
    
    // MARK: - Network Tests
    
    func testNetworkAddressPrefix() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        guard let seed = CoreSDKWrapper.shared.mnemonicToSeed(mnemonic) else {
            XCTFail("Failed to generate seed")
            return
        }
        
        let path = DerivationPath.dashBIP44(account: 0, change: 0, index: 0, testnet: false)
        
        // Mainnet address
        if let mainnetKey = HDKeyDerivation.deriveKey(seed: seed, path: path, network: .mainnet),
           let mainnetAddress = HDKeyDerivation.addressFromPublicKey(mainnetKey.publicKey, network: .mainnet) {
            XCTAssertTrue(mainnetAddress.starts(with: "X"))
        }
        
        // Testnet address
        let testnetPath = DerivationPath.dashBIP44(account: 0, change: 0, index: 0, testnet: true)
        if let testnetKey = HDKeyDerivation.deriveKey(seed: seed, path: testnetPath, network: .testnet),
           let testnetAddress = HDKeyDerivation.addressFromPublicKey(testnetKey.publicKey, network: .testnet) {
            XCTAssertTrue(testnetAddress.starts(with: "y"))
        }
    }
    
    // MARK: - Error Cases
    
    func testInvalidDerivationPath() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        guard let seed = CoreSDKWrapper.shared.mnemonicToSeed(mnemonic) else {
            XCTFail("Failed to generate seed")
            return
        }
        
        // Test with invalid path
        let invalidPath = DerivationPath(indexes: [])
        let derivedKey = HDKeyDerivation.deriveKey(seed: seed, path: invalidPath, network: .testnet)
        
        // Should handle gracefully
        XCTAssertNil(derivedKey)
    }
}