import XCTest

@testable import SwiftDashSDK

/// The BIP-39 passphrase ("25th word") leg of the resolver path: a wallet
/// whose `WalletStorage` holds a passphrase next to its mnemonic must sign
/// with the passphrase-derived keys, and a wallet without one must keep
/// signing exactly as before. Runs entirely in-process against an
/// in-memory storage — no Keychain, no network.
@MainActor
final class SeedPassphraseResolverTests: XCTestCase {

    // Canonical BIP-39 test vector (all-zero entropy).
    private let mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    private let passphrase = "TREZOR"
    private let path = "m/9'/1'/5'/0'/0'/3'/2'"

    /// In-memory `WalletStorage` holding both the mnemonic and the optional
    /// passphrase, mirroring the real storage's per-item contract.
    private final class InMemoryWalletStorage: WalletStorage {
        private var mnemonics: [Data: Data] = [:]
        private var passphrases: [Data: Data] = [:]
        private let lock = NSLock()

        override func storeMnemonic(_ mnemonic: String, for walletId: Data) throws {
            lock.lock(); defer { lock.unlock() }
            mnemonics[walletId] = Data(mnemonic.utf8)
        }

        override func retrieveMnemonicUTF8Bytes(for walletId: Data) throws -> Data {
            lock.lock(); defer { lock.unlock() }
            guard let data = mnemonics[walletId], !data.isEmpty else {
                throw WalletStorageError.mnemonicNotFound
            }
            return data
        }

        override func deleteMnemonic(for walletId: Data) throws {
            lock.lock(); defer { lock.unlock() }
            mnemonics[walletId] = nil
        }

        override func mnemonicAvailability(for walletId: Data) -> MnemonicAvailability {
            lock.lock(); defer { lock.unlock() }
            return mnemonics[walletId] != nil ? .present : .absent
        }

        override func storePassphrase(_ passphrase: String, for walletId: Data) throws {
            guard !passphrase.isEmpty else { throw WalletStorageError.emptyPassphrase }
            lock.lock(); defer { lock.unlock() }
            passphrases[walletId] = Data(passphrase.utf8)
        }

        override func retrievePassphraseUTF8Bytes(for walletId: Data) throws -> Data {
            lock.lock(); defer { lock.unlock() }
            guard let data = passphrases[walletId], !data.isEmpty else {
                throw WalletStorageError.passphraseNotFound
            }
            return data
        }

        override func passphraseAvailability(for walletId: Data) -> MnemonicAvailability {
            lock.lock(); defer { lock.unlock() }
            return passphrases[walletId] != nil ? .present : .absent
        }

        override func deletePassphrase(for walletId: Data) throws {
            lock.lock(); defer { lock.unlock() }
            passphrases[walletId] = nil
        }
    }

    /// Compressed pubkey at `path` derived locally from `(mnemonic, passphrase)`.
    private func expectedPubkey(passphrase: String?) throws -> Data {
        let seed = try Mnemonic.toSeed(mnemonic: mnemonic, passphrase: passphrase)
        let wallet = try Wallet(seed: seed, network: .testnet)
        return try XCTUnwrap(Data(hexString: wallet.derivePublicKey(path: path)))
    }

    /// Sign through the resolver FFI for `walletId` and return the error tag
    /// (`.ok` on success) — the same entry point `KeychainSigner` uses for
    /// resolver-backed identity keys.
    private func signViaResolver(
        storage: WalletStorage,
        walletId: Data,
        expectedKey: Data
    ) -> SignWithMnemonicResolverError {
        let resolver = MnemonicResolver(storage: storage)
        var signature = [UInt8](repeating: 0, count: 128)
        var signatureLen: UInt = 0
        var errorTag: UInt8 = 0
        let data = Data("identity state transition".utf8)
        let rc: Int32 = withExtendedLifetime(resolver) {
            walletId.withUnsafeBytes { idPtr in
                path.withCString { pathPtr in
                    data.withUnsafeBytes { dataPtr in
                        expectedKey.withUnsafeBytes { keyPtr in
                            dash_sdk_sign_with_mnemonic_resolver_and_path(
                                resolver.handle,
                                idPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                pathPtr,
                                dataPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                UInt(data.count),
                                0, // ECDSA_SECP256K1
                                Network.testnet.ffiValue,
                                keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                UInt(expectedKey.count),
                                &signature,
                                UInt(signature.count),
                                &signatureLen,
                                &errorTag
                            )
                        }
                    }
                }
            }
        }
        if rc == 0 {
            XCTAssertEqual(signatureLen, 65, "compact-recoverable ECDSA signature")
            return .ok
        }
        return SignWithMnemonicResolverError(rawValue: errorTag) ?? .resolverFailed
    }

    func testToSeedHonoursThePassphraseAndDetectsLanguage() throws {
        let plain = try Mnemonic.toSeed(mnemonic: mnemonic)
        let withPassphrase = try Mnemonic.toSeed(mnemonic: mnemonic, passphrase: passphrase)
        XCTAssertEqual(plain.count, 64)
        XCTAssertEqual(
            plain.map { String(format: "%02x", $0) }.joined(),
            "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc19a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4")
        XCTAssertEqual(
            withPassphrase.map { String(format: "%02x", $0) }.joined(),
            "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04")
        XCTAssertEqual(try Mnemonic.toSeed(mnemonic: mnemonic, passphrase: ""), plain,
                       "an empty passphrase is no passphrase")

        // The any-language FFI must accept a non-English phrase, which the
        // English-only key-wallet-ffi helper rejected.
        let japanese = "あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あおぞら"
        XCTAssertEqual(try Mnemonic.toSeed(mnemonic: japanese).count, 64)
    }

    func testPassphraseWalletSignsWithPassphraseDerivedKey() throws {
        let walletId = Data(repeating: 0x7C, count: 32)
        let storage = InMemoryWalletStorage()
        try storage.storeMnemonic(mnemonic, for: walletId)
        try storage.storePassphrase(passphrase, for: walletId)

        let passphraseKey = try expectedPubkey(passphrase: passphrase)
        let plainKey = try expectedPubkey(passphrase: nil)
        XCTAssertNotEqual(passphraseKey, plainKey)

        XCTAssertEqual(
            signViaResolver(storage: storage, walletId: walletId, expectedKey: passphraseKey), .ok,
            "the resolver must hand Rust the stored passphrase")
        XCTAssertEqual(
            signViaResolver(storage: storage, walletId: walletId, expectedKey: plainKey),
            .pubkeyMismatch,
            "the passphrase-less key must not bind for a passphrase wallet")
    }

    func testWalletWithoutPassphraseSignsAsBefore() throws {
        let walletId = Data(repeating: 0x7D, count: 32)
        let storage = InMemoryWalletStorage()
        try storage.storeMnemonic(mnemonic, for: walletId)

        let plainKey = try expectedPubkey(passphrase: nil)
        XCTAssertEqual(
            signViaResolver(storage: storage, walletId: walletId, expectedKey: plainKey), .ok)
    }

    func testDeletingThePassphraseChangesWhichKeyBinds() throws {
        let walletId = Data(repeating: 0x7E, count: 32)
        let storage = InMemoryWalletStorage()
        try storage.storeMnemonic(mnemonic, for: walletId)
        try storage.storePassphrase(passphrase, for: walletId)
        XCTAssertTrue(storage.hasPassphrase(for: walletId))

        try storage.deletePassphrase(for: walletId)
        XCTAssertFalse(storage.hasPassphrase(for: walletId))
        XCTAssertEqual(
            signViaResolver(
                storage: storage, walletId: walletId, expectedKey: try expectedPubkey(passphrase: nil)),
            .ok)
    }

    func testEmptyPassphraseIsRejectedByStorage() {
        let storage = InMemoryWalletStorage()
        XCTAssertThrowsError(try storage.storePassphrase("", for: Data(repeating: 1, count: 32))) { error in
            guard case WalletStorageError.emptyPassphrase = error else {
                return XCTFail("expected emptyPassphrase, got \(error)")
            }
        }
    }
}
