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
        XCTAssertEqual(publicKeyHashHex, KeychainManager.computePublicKeyHashHex(slot.publicKeyData))
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

    private func ecdsaInput(
        keyId: UInt32,
        securityLevel: SecurityLevel,
        publicKeyData: Data,
        disabledAt: UInt64? = nil
    ) -> ManagedPlatformWallet.IdentityKeyCheckInput {
        ManagedPlatformWallet.IdentityKeyCheckInput(
            keyId: keyId, keyType: .ecdsaSecp256k1,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: securityLevel.rawValue,
            publicKeyData: publicKeyData, disabledAt: disabledAt)
    }

    /// A master rotation leaves the former, foreign master disabled; the
    /// enabled master is the wallet's, so the identity is.
    func testShouldOwnAnIdentityWhoseEnabledMasterReplacedAForeignOne() throws {
        let (wallet, storage) = try makeWallet()
        let foreign = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 9, network: .testnet, storage: storage)
        let current = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 1, network: .testnet, storage: storage)

        let check = try wallet.checkIdentityKeys(
            identityIndex: 1,
            keys: [
                ecdsaInput(
                    keyId: 0, securityLevel: .master, publicKeyData: foreign.publicKeyData,
                    disabledAt: 1_700_000_000_000),
                ecdsaInput(keyId: 1, securityLevel: .master, publicKeyData: current.publicKeyData),
            ],
            unreadableKeyCount: 0,
            network: .testnet, storage: storage)

        XCTAssertEqual(check.ownership, .owned)
        XCTAssertEqual(check.keys.count, 2)
        guard case .mismatch = check.keys[0], case .matches = check.keys[1] else {
            return XCTFail("expected mismatch then matches, got \(check.keys)")
        }
    }

    /// A key added from elsewhere doesn't make an identity whose master
    /// is the wallet's look foreign.
    func testShouldOwnAnIdentityWhoseMasterMatchesBesideAForeignKey() throws {
        let (wallet, storage) = try makeWallet()
        let master = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 0, network: .testnet, storage: storage)
        let foreign = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 7, network: .testnet, storage: storage)

        let check = try wallet.checkIdentityKeys(
            identityIndex: 1,
            keys: [
                ecdsaInput(keyId: 0, securityLevel: .master, publicKeyData: master.publicKeyData),
                ecdsaInput(keyId: 1, securityLevel: .high, publicKeyData: foreign.publicKeyData),
            ],
            unreadableKeyCount: 0,
            network: .testnet, storage: storage)

        XCTAssertEqual(check.ownership, .owned)
        guard case .matches = check.keys[0], case .mismatch = check.keys[1] else {
            return XCTFail("expected matches then mismatch, got \(check.keys)")
        }
    }

    func testShouldNotOwnAnIdentityWhoseOnlyEnabledMasterIsForeign() throws {
        let (wallet, storage) = try makeWallet()
        let foreign = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 5, network: .testnet, storage: storage)

        let check = try wallet.checkIdentityKeys(
            identityIndex: 1,
            keys: [ecdsaInput(keyId: 0, securityLevel: .master, publicKeyData: foreign.publicKeyData)],
            unreadableKeyCount: 0,
            network: .testnet, storage: storage)

        XCTAssertEqual(check.ownership, .notOwned)
    }

    /// A key the caller couldn't describe could be the wallet's, so the
    /// identity isn't called foreign. SwiftExampleApp has no test target,
    /// so the sheet's case is pinned here at the bridge.
    func testShouldNotCallAnIdentityForeignBesideAKeyItCouldNotRead() throws {
        let (wallet, storage) = try makeWallet()
        let foreign = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 5, network: .testnet, storage: storage)

        let check = try wallet.checkIdentityKeys(
            identityIndex: 1,
            keys: [ecdsaInput(keyId: 0, securityLevel: .master, publicKeyData: foreign.publicKeyData)],
            unreadableKeyCount: 1,
            network: .testnet, storage: storage)

        XCTAssertNotEqual(check.ownership, .notOwned)
        XCTAssertEqual(check.ownership, .undetermined)
    }

    /// The wallet's own master (identity 1, key 0) encoded uncompressed.
    /// The check compares ECDSA keys only in their compressed form, so it
    /// doesn't compare this one, and the identity isn't called foreign.
    func testShouldNotCompareAnUncompressedMaster() throws {
        let (wallet, storage) = try makeWallet()
        let master = try wallet.deriveIdentityAuthKeyAtSlot(
            identityIndex: 1, keyId: 0, network: .testnet, storage: storage)
        let uncompressedHex =
            "0412aa9ac813f9751da6747c4ac7798a9c5f778545788204785eafbfe287544144"
            + "27ac78c58ecb3fa36681c136831f6ae9adb4fc619cc80dbd5e7977c2690122a1"
        // The pinned encoding is the slot key: same x, and an odd y for
        // the 0x03 prefix.
        XCTAssertEqual(master.publicKeyHex, "03" + uncompressedHex.dropFirst(2).prefix(64))
        let uncompressed = try XCTUnwrap(Data(hexString: uncompressedHex))

        let check = try wallet.checkIdentityKeys(
            identityIndex: 1,
            keys: [ecdsaInput(keyId: 0, securityLevel: .master, publicKeyData: uncompressed)],
            unreadableKeyCount: 0,
            network: .testnet, storage: storage)

        XCTAssertEqual(check.keys, [.notCompared])
        XCTAssertEqual(check.ownership, .undetermined)
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
