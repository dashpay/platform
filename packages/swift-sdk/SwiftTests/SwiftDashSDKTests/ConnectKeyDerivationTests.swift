import XCTest

@testable import SwiftDashSDK

/// DashPay Connect key derivation at the DIP-13 sub-feature paths
/// (`m/9'/coin'/5'/{6|7}'/0'/identityId'/leaf'[/purpose']`, dashpay/dips#191)
/// through `ManagedPlatformWallet.deriveConnectKey`. The fixed vectors are
/// the ones `platform-wallet`'s
/// `connect_keypair_is_deterministic_from_fixed_vectors` pins on the Rust
/// side, so a drift on either side of the FFI is caught.
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
    /// Pinned in `identity_handle.rs`: mainnet, sub-feature 7', purpose 1'.
    private let mainnetEncryptionPubkeyHex =
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
            purpose: .encryption, network: .mainnet, storage: storage)
        XCTAssertEqual(encryption.publicKeyData.toHexString(), mainnetEncryptionPubkeyHex)
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
            purpose: .encryption, network: .testnet, storage: storage)
        let dec = try wallet.deriveConnectKey(
            subFeature: .appEncryption, identityId: identityId, leaf: leaf,
            purpose: .decryption, network: .testnet, storage: storage)
        let mainnetAuth = try wallet.deriveConnectKey(
            subFeature: .sessionAuthentication, identityId: identityId, leaf: leaf,
            network: .mainnet, storage: storage)

        let all = [auth, authOtherLeaf, authOtherIdentity, enc, dec, mainnetAuth]
            .map { $0.publicKeyData }
        XCTAssertEqual(Set(all).count, all.count, "every variation derives a distinct key")
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
