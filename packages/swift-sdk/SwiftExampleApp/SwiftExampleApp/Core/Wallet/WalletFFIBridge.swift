import Foundation
import DashSDKFFI

// MARK: - Wallet FFI Bridge

/// Bridge to access key-wallet functionality from rust-dashcore
public class WalletFFIBridge {
    public static let shared = WalletFFIBridge()
    
    private init() {
        // Initialize the key wallet FFI library
        // Note: FFI functions will be linked at runtime from DashSDK.xcframework
    }
    
    // Helper to get last error from FFI
    private func getLastError() -> String? {
        guard let errorPtr = dash_spv_ffi_get_last_error() else {
            return nil
        }
        // Note: dash_spv_ffi_get_last_error returns a const char* that doesn't need to be freed
        return String(cString: errorPtr)
    }
    
    // MARK: - Mnemonic Operations
    
    public func generateMnemonic(wordCount: UInt8 = 12) -> String? {
        print("WalletFFIBridge.generateMnemonic called with wordCount: \(wordCount)")
        
        guard let mnemonicPtr = dash_key_mnemonic_generate(wordCount) else {
            let error = getLastError() ?? "Unknown error"
            print("dash_key_mnemonic_generate returned nil. Error: \(error)")
            return nil
        }
        defer { dash_key_mnemonic_destroy(mnemonicPtr) }
        
        guard let phrasePtr = dash_key_mnemonic_phrase(mnemonicPtr) else {
            let error = getLastError() ?? "Unknown error"
            print("dash_key_mnemonic_phrase returned nil. Error: \(error)")
            return nil
        }
        
        let phrase = String(cString: phrasePtr)
        dash_sdk_string_free(UnsafeMutablePointer(mutating: phrasePtr))
        
        print("Generated mnemonic: \(phrase)")
        return phrase
    }
    
    public func validateMnemonic(_ phrase: String) -> Bool {
        print("WalletFFIBridge.validateMnemonic called with phrase: \(phrase)")
        
        guard let mnemonicPtr = dash_key_mnemonic_from_phrase(phrase) else {
            print("dash_key_mnemonic_from_phrase returned nil")
            return false
        }
        defer { dash_key_mnemonic_destroy(mnemonicPtr) }
        
        print("Mnemonic validation successful")
        return true
    }
    
    public func mnemonicToSeed(_ mnemonic: String, passphrase: String = "") -> Data? {
        print("WalletFFIBridge.mnemonicToSeed called with mnemonic: \(mnemonic)")
        
        guard let mnemonicPtr = dash_key_mnemonic_from_phrase(mnemonic) else {
            print("dash_key_mnemonic_from_phrase returned nil in mnemonicToSeed")
            return nil
        }
        defer { dash_key_mnemonic_destroy(mnemonicPtr) }
        
        var seed = Data(count: 64)
        let result = seed.withUnsafeMutableBytes { seedBytes in
            dash_key_mnemonic_to_seed(
                mnemonicPtr,
                passphrase.isEmpty ? nil : passphrase,
                seedBytes.bindMemory(to: UInt8.self).baseAddress
            )
        }
        
        if result == 0 {
            print("Seed generated successfully, length: \(seed.count)")
        } else {
            print("dash_key_mnemonic_to_seed failed with result: \(result)")
        }
        
        return result == 0 ? seed : nil
    }
    
    // MARK: - Key Derivation
    
    public func deriveKey(seed: Data, path: String, network: DashNetwork) -> DerivedKey? {
        print("WalletFFIBridge.deriveKey called with path: \(path)")
        
        // Create master key from seed
        guard let xprv = seed.withUnsafeBytes({ seedBytes in
            dash_key_xprv_from_seed(seedBytes.bindMemory(to: UInt8.self).baseAddress, networkToFFI(network))
        }) else {
            let error = getLastError() ?? "Unknown error"
            print("Failed to create master key: \(error)")
            return nil
        }
        defer { dash_key_xprv_destroy(xprv) }
        
        // Derive key at path
        guard let derivedXprv = dash_key_xprv_derive_path(xprv, path) else {
            let error = getLastError() ?? "Unknown error"
            print("Failed to derive key at path \(path): \(error)")
            return nil
        }
        defer { dash_key_xprv_destroy(derivedXprv) }
        
        // Get private key
        var privateKey = Data(count: 32)
        let privResult = privateKey.withUnsafeMutableBytes { privBytes in
            dash_key_xprv_private_key(derivedXprv, privBytes.bindMemory(to: UInt8.self).baseAddress)
        }
        
        guard privResult == 0 else {
            print("Failed to extract private key")
            return nil
        }
        
        // Get public key
        guard let xpub = dash_key_xprv_to_xpub(derivedXprv) else {
            let error = getLastError() ?? "Unknown error"
            print("Failed to get extended public key: \(error)")
            return nil
        }
        defer { dash_key_xpub_destroy(xpub) }
        
        var publicKey = Data(count: 33)
        let pubResult = publicKey.withUnsafeMutableBytes { pubBytes in
            dash_key_xpub_public_key(xpub, pubBytes.bindMemory(to: UInt8.self).baseAddress)
        }
        
        guard pubResult == 0 else {
            print("Failed to extract public key")
            return nil
        }
        
        print("Successfully derived key at path \(path)")
        return DerivedKey(
            privateKey: privateKey,
            publicKey: publicKey,
            path: path
        )
    }
    
    // MARK: - Address Generation
    
    public func addressFromPublicKey(_ publicKey: Data, network: DashNetwork) -> String? {
        print("WalletFFIBridge.addressFromPublicKey called, pubkey length: \(publicKey.count)")
        
        guard publicKey.count == 33 else {
            print("Invalid public key length: \(publicKey.count), expected 33")
            return nil
        }
        
        guard let addressPtr = publicKey.withUnsafeBytes({ pubkeyBytes in
            dash_key_address_from_pubkey(pubkeyBytes.bindMemory(to: UInt8.self).baseAddress, networkToFFI(network))
        }) else {
            let error = getLastError() ?? "Unknown error"
            print("Failed to generate address: \(error)")
            return nil
        }
        
        let address = String(cString: addressPtr)
        dash_sdk_string_free(addressPtr)
        
        print("Generated address: \(address)")
        return address
    }
    
    public func validateAddress(_ address: String, network: DashNetwork) -> Bool {
        let result = dash_key_address_validate(address, networkToFFI(network))
        return result == 1
    }
    
    // MARK: - Transaction Operations
    
    public func createTransaction() -> UnsafeMutablePointer<FFITransaction>? {
        return dash_tx_create()
    }
    
    public func destroyTransaction(_ tx: UnsafeMutablePointer<FFITransaction>) {
        dash_tx_destroy(tx)
    }
    
    public func addInput(to tx: UnsafeMutablePointer<FFITransaction>, txid: Data, vout: UInt32, scriptSig: Data = Data(), sequence: UInt32 = 0xFFFFFFFF) -> Bool {
        guard txid.count == 32 else { return false }
        
        var input = FFITxIn()
        txid.withUnsafeBytes { bytes in
            withUnsafeMutableBytes(of: &input.txid) { txidBytes in
                txidBytes.copyMemory(from: bytes)
            }
        }
        input.vout = vout
        input.sequence = sequence
        
        if scriptSig.isEmpty {
            input.script_sig_len = 0
            input.script_sig = nil
        } else {
            input.script_sig_len = UInt32(scriptSig.count)
            input.script_sig = scriptSig.withUnsafeBytes { $0.bindMemory(to: UInt8.self).baseAddress }
        }
        
        return dash_tx_add_input(tx, &input) == 0
    }
    
    public func addOutput(to tx: UnsafeMutablePointer<FFITransaction>, address: String, amount: UInt64, network: DashNetwork) -> Bool {
        let ffiNetwork = networkToFFI(network)
        
        // Convert address to pubkey hash
        var pubkeyHash = Data(count: 20)
        let hashResult = pubkeyHash.withUnsafeMutableBytes { hashBytes in
            dash_address_to_pubkey_hash(
                address,
                ffiNetwork,
                hashBytes.bindMemory(to: UInt8.self).baseAddress
            )
        }
        
        guard hashResult == 0 else { return false }
        
        // Create P2PKH script
        var scriptPubkey = Data(count: 25)  // Typical P2PKH script size
        var scriptLen: UInt32 = 25
        
        let scriptResult = scriptPubkey.withUnsafeMutableBytes { scriptBytes in
            pubkeyHash.withUnsafeBytes { hashBytes in
                dash_script_p2pkh(
                    hashBytes.bindMemory(to: UInt8.self).baseAddress,
                    scriptBytes.bindMemory(to: UInt8.self).baseAddress,
                    &scriptLen
                )
            }
        }
        
        guard scriptResult == 0 else { return false }
        scriptPubkey = scriptPubkey.prefix(Int(scriptLen))
        
        var output = FFITxOut()
        output.amount = amount
        output.script_pubkey_len = scriptLen
        output.script_pubkey = scriptPubkey.withUnsafeBytes { $0.bindMemory(to: UInt8.self).baseAddress }
        
        return dash_tx_add_output(tx, &output) == 0
    }
    
    public func getTransactionId(_ tx: UnsafeMutablePointer<FFITransaction>) -> Data? {
        // Placeholder
        return Data(repeating: 0xFF, count: 32)
    }
    
    public func serializeTransaction(_ tx: UnsafeMutablePointer<FFITransaction>) -> Data? {
        // Placeholder
        return Data()
    }
    
    public func signInput(tx: UnsafeMutablePointer<FFITransaction>, inputIndex: UInt32, privateKey: Data, scriptPubkey: Data, sighashType: UInt32 = 1) -> Bool {
        // Placeholder
        return false
    }
    
    // MARK: - Helper Functions
    
    private func networkToFFI(_ network: DashNetwork) -> FFIKeyNetwork {
        switch network {
        case .mainnet:
            return FFIKeyNetwork(0) // KeyMainnet
        case .testnet:
            return FFIKeyNetwork(1) // KeyTestnet
        case .devnet:
            return FFIKeyNetwork(3) // KeyDevnet
        }
    }
}

// MARK: - Helper Types

public struct DerivedKey {
    public let privateKey: Data
    public let publicKey: Data
    public let path: String
}

public enum DashNetwork: String {
    case mainnet = "mainnet"
    case testnet = "testnet"
    case devnet = "devnet"
}

// FFI types will be added when DashSDK import is fixed