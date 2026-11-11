import XCTest

@testable import SwiftDashSDK

/// DashPay Connect key derivation at the DIP-13 sub-feature paths
/// (`m/9'/coin'/5'/{6|7}'/0'/identityId'/leaf'[/purpose'/rotation']`,
/// dashpay/dips#191) through `ManagedPlatformWallet.deriveConnectKey` and
/// `deriveLegacyAppEncryptionKey`. The fixed vectors are the ones
/// `platform-wallet`'s `connect_keypairs_match_the_pinned_vectors` pins on
/// the Rust side, so a drift on either side of the FFI is caught.
@MainActor
final class ConnectKeyDerivationTests: XCTestCase {

    // Canonical BIP-39 test vector (all-zero entropy).
    private let mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    private let walletId = Data(repeating: 0x07, count: 32)
    private let identityId = Data(repeating: 0x35, count: 32)
    private let leaf = Data(repeating: 0x6B, count: 32)

    /// Pinned in `identity_handle.rs`: testnet, sub-feature 6', no purpose.
    private let testnetAuthPubkeyHex =
        "022c8b2e806244482374b1caf8306146dc03aad3b99a5954efd5e70a0eddd37a5d"
    /// Pinned in `identity_handle.rs`: mainnet, sub-feature 7', purpose 1',
    /// rotation 0'.
    private let mainnetEncryptionPubkeyHex =
        "031a882c119e2b1f8e0349b444d877afa2599efeef49a26930fd25518cb6e9d57a"
    /// Pinned in `identity_handle.rs`: mainnet, sub-feature 7', purpose 1',
    /// rotation 1'.
    private let mainnetEncryptionRotationOnePubkeyHex =
        "03d0f6f0f48d968b9554d8e7fe467d7135bacf29d5c4994ca593bd10ebeed696f6"
    /// Pinned in `identity_handle.rs`: mainnet, sub-feature 7', purpose 1',
    /// pre-rotation layout (no rotation level).
    private let mainnetLegacyEncryptionPubkeyHex =
        "03e989de1b62f137231cc06659c810c17100becd1f5e23d5710faa7602769fe434"

    /// In-memory `WalletStorage` so the resolver never touches the macOS
    /// Keychain (see `IdentityResolverSignIntegrationTests`).
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
        // The derive is resolver-driven and never touches the native
        // wallet handle, so a null handle is fine here.
        return (ManagedPlatformWallet(handle: NULL_HANDLE, walletId: walletId), storage)
    }

    func testDerivesThePinnedVectorsDeterministically() throws {
        let (wallet, storage) = try makeWallet()

        let auth = try wallet.deriveConnectKey(
            subFeature: .sessionAuthentication, identityId: identityId, leaf: leaf,
            network: .testnet, storage: storage)
        let authAgain = try wallet.deriveConnectKey(
            subFeature: .sessionAuthentication, identityId: identityId, leaf: leaf,
            network: .testnet, storage: storage)
        XCTAssertEqual(auth, authAgain, "same inputs derive the same key")
        XCTAssertEqual(auth.publicKeyData.count, 33)
        XCTAssertEqual(auth.privateKeyData.count, 32)
        XCTAssertEqual(auth.publicKeyData.toHexString(), testnetAuthPubkeyHex)

        let encryption = try wallet.deriveConnectKey(
            subFeature: .appEncryption, identityId: identityId, leaf: leaf,
            purpose: .encryption, rotation: 0, network: .mainnet, storage: storage)
        XCTAssertEqual(encryption.publicKeyData.toHexString(), mainnetEncryptionPubkeyHex)

        let nextRotation = try wallet.deriveConnectKey(
            subFeature: .appEncryption, identityId: identityId, leaf: leaf,
            purpose: .encryption, rotation: 1, network: .mainnet, storage: storage)
        XCTAssertEqual(
            nextRotation.publicKeyData.toHexString(), mainnetEncryptionRotationOnePubkeyHex)

        let legacy = try wallet.deriveLegacyAppEncryptionKey(
            identityId: identityId, contractId: leaf, purpose: .encryption,
            network: .mainnet, storage: storage)
        XCTAssertEqual(legacy.publicKeyData.toHexString(), mainnetLegacyEncryptionPubkeyHex)
    }

    func testDifferentLeavesPurposesAndSubFeaturesGiveDifferentKeys() throws {
        let (wallet, storage) = try makeWallet()

        let auth = try wallet.deriveConnectKey(
            subFeature: .sessionAuthentication, identityId: identityId, leaf: leaf,
            network: .testnet, storage: storage)
        let authOtherLeaf = try wallet.deriveConnectKey(
            subFeature: .sessionAuthentication, identityId: identityId,
            leaf: Data(repeating: 0x6C, count: 32), network: .testnet, storage: storage)
        let authOtherIdentity = try wallet.deriveConnectKey(
            subFeature: .sessionAuthentication, identityId: Data(repeating: 0x36, count: 32),
            leaf: leaf, network: .testnet, storage: storage)
        let enc = try wallet.deriveConnectKey(
            subFeature: .appEncryption, identityId: identityId, leaf: leaf,
            purpose: .encryption, rotation: 0, network: .testnet, storage: storage)
        let dec = try wallet.deriveConnectKey(
            subFeature: .appEncryption, identityId: identityId, leaf: leaf,
            purpose: .decryption, rotation: 0, network: .testnet, storage: storage)
        let encNextRotation = try wallet.deriveConnectKey(
            subFeature: .appEncryption, identityId: identityId, leaf: leaf,
            purpose: .encryption, rotation: 1, network: .testnet, storage: storage)
        let encLegacy = try wallet.deriveLegacyAppEncryptionKey(
            identityId: identityId, contractId: leaf, purpose: .encryption,
            network: .testnet, storage: storage)
        let mainnetAuth = try wallet.deriveConnectKey(
            subFeature: .sessionAuthentication, identityId: identityId, leaf: leaf,
            network: .mainnet, storage: storage)

        let all = [
            auth, authOtherLeaf, authOtherIdentity, enc, dec, encNextRotation, encLegacy,
            mainnetAuth,
        ]
            .map { $0.publicKeyData }
        XCTAssertEqual(Set(all).count, all.count, "every variation derives a distinct key")
    }

    func testRejectsMismatchedSubFeaturesAndAWideRotation() throws {
        let (wallet, storage) = try makeWallet()

        // App encryption needs a purpose and rotation; the session key has
        // neither.
        XCTAssertThrowsError(
            try wallet.deriveConnectKey(
                subFeature: .appEncryption, identityId: identityId, leaf: leaf,
                network: .testnet, storage: storage))
        XCTAssertThrowsError(
            try wallet.deriveConnectKey(
                subFeature: .sessionAuthentication, identityId: identityId, leaf: leaf,
                purpose: .encryption, rotation: 0, network: .testnet, storage: storage))
        // A rotation is a 31-bit hardened index.
        XCTAssertThrowsError(
            try wallet.deriveConnectKey(
                subFeature: .appEncryption, identityId: identityId, leaf: leaf,
                purpose: .encryption, rotation: 0x8000_0000, network: .testnet,
                storage: storage)
        ) { error in
            guard case PlatformWalletError.invalidParameter = error else {
                return XCTFail("expected invalidParameter, got \(error)")
            }
        }
    }

    func testRejectsMalformedIdsAndAMissingMnemonic() throws {
        let (wallet, storage) = try makeWallet()

        XCTAssertThrowsError(
            try wallet.deriveConnectKey(
                subFeature: .sessionAuthentication, identityId: Data(repeating: 0x35, count: 31),
                leaf: leaf, network: .testnet, storage: storage))
        XCTAssertThrowsError(
            try wallet.deriveConnectKey(
                subFeature: .sessionAuthentication, identityId: identityId,
                leaf: Data(), network: .testnet, storage: storage))

        let empty = InMemoryWalletStorage()
        XCTAssertThrowsError(
            try wallet.deriveConnectKey(
                subFeature: .sessionAuthentication, identityId: identityId, leaf: leaf,
                network: .testnet, storage: empty)
        ) { error in
            guard case PlatformWalletError.walletOperation(let message) = error else {
                return XCTFail("expected walletOperation, got \(error)")
            }
            XCTAssertTrue(message.contains("no mnemonic stored"), message)
        }
    }
}
