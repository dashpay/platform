import XCTest

@testable import SwiftDashSDK

/// `ManagedPlatformWallet.checkIdentityKey`, the bridge to
/// `platform_wallet_check_identity_key_with_resolver`. Rust decides the
/// outcome; these check that each outcome reaches Swift with its data,
/// against the slot derivation `deriveIdentityAuthKeyAtSlot` gives.
@MainActor
final class IdentityKeyWalletCheckTests: XCTestCase {

    // Canonical BIP-39 test vector (all-zero entropy).
    private let mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    private let walletId = Data(repeating: 0x07, count: 32)

    /// In-memory `WalletStorage` so the resolver never touches the
    /// Keychain (see `ConnectKeyDerivationTests`).
    private final class InMemoryWalletStorage: WalletStorage {
        private var mnemonics: [Data: Data] = [:]
        private let lock = NSLock()

        override func storeMnemonic(_ mnemonic: String, for walletId: Data) throws {
            lock.lock()
            defer { lock.unlock() }
            mnemonics[walletId] = Data(mnemonic.utf8)
        }

        override func retrieveMnemonicUTF8Bytes(for walletId: Data) throws -> Data {
            lock.lock()
            defer { lock.unlock() }
            guard let data = mnemonics[walletId], !data.isEmpty else {
                throw WalletStorageError.mnemonicNotFound
            }
            return data
        }

        override func deleteMnemonic(for walletId: Data) throws {
            lock.lock()
            defer { lock.unlock() }
            mnemonics[walletId] = nil
        }

        override func mnemonicAvailability(for walletId: Data) -> MnemonicAvailability {
            lock.lock()
            defer { lock.unlock() }
            return mnemonics[walletId] != nil ? .present : .absent
        }
    }

    private func makeWallet() throws -> (ManagedPlatformWallet, WalletStorage) {
        let storage = InMemoryWalletStorage()
        try storage.storeMnemonic(mnemonic, for: walletId)
        // The check is resolver-driven and never touches the native
        // wallet handle, so a null handle is fine here.
        return (ManagedPlatformWallet(handle: NULL_HANDLE, walletId: walletId), storage)
    }

    func testShouldReportAHecateSignerKeyAsNotDerived() throws {
        let (wallet, storage) = try makeWallet()

        let check = try wallet.checkIdentityKey(
            identityIndex: 0, keyId: 3, keyType: .redpallasHecate,
            purpose: KeyPurpose.hecateSigner.rawValue,
            securityLevel: SecurityLevel.medium.rawValue,
            publicKeyData: Data(repeating: 0x01, count: 64),
            network: .testnet, storage: storage)

        XCTAssertEqual(check, .notDerived)
    }

    func testShouldMatchAnEcdsaKeyAtItsSlot() throws {
        let (wallet, storage) = try makeWallet()
        let slot = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 2, network: .testnet, storage: storage)

        let check = try wallet.checkIdentityKey(
            identityIndex: 1, keyId: 2, keyType: .ecdsaSecp256k1,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.high.rawValue,
            publicKeyData: slot.publicKeyData,
            network: .testnet, storage: storage)

        guard case let .matches(path, privateKeyData, publicKeyHashHex) = check else {
            return XCTFail("expected matches, got \(check)")
        }
        XCTAssertEqual(path, slot.derivationPath)
        XCTAssertEqual(privateKeyData, slot.privateKeyData)
        XCTAssertEqual(publicKeyHashHex.count, 40)
    }

    func testShouldReportAMismatchForAKeyFromAnotherSlot() throws {
        let (wallet, storage) = try makeWallet()
        let otherSlot = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 2, network: .testnet, storage: storage)
        let ownSlot = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 3, network: .testnet, storage: storage)

        let check = try wallet.checkIdentityKey(
            identityIndex: 1, keyId: 3, keyType: .ecdsaSecp256k1,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.high.rawValue,
            publicKeyData: otherSlot.publicKeyData,
            network: .testnet, storage: storage)

        XCTAssertEqual(
            check,
            .mismatch(
                derivationPath: ownSlot.derivationPath,
                derivedPublicKeyData: ownSlot.publicKeyData))
    }

    func testShouldThrowWithoutAMnemonic() throws {
        let wallet = ManagedPlatformWallet(handle: NULL_HANDLE, walletId: walletId)

        XCTAssertThrowsError(
            try wallet.checkIdentityKey(
                identityIndex: 0, keyId: 0, keyType: .ecdsaSecp256k1,
                purpose: KeyPurpose.authentication.rawValue,
                securityLevel: SecurityLevel.high.rawValue,
                publicKeyData: Data(repeating: 0x02, count: 33),
                network: .testnet, storage: InMemoryWalletStorage()))
    }
}
