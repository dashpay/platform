import SwiftData
import XCTest

@testable import SwiftDashSDK

/// End-to-end coverage of the load-bearing path the funded sim UAT was meant to
/// exercise: `KeychainSigner.signIdentityKeyOnDemand` resolves a breadcrumb row,
/// the mnemonic resolver fetches the seed from `WalletStorage`, the key is
/// derived on demand at the DIP-9 path, the MF-2 binding confirms it reproduces
/// the row's public key, and a signature is produced — entirely in-process, no
/// network, no stored scalar. This is the derive-sign-destroy path that replaces
/// the carried scalar.
@MainActor
final class IdentityResolverSignIntegrationTests: XCTestCase {

    // Canonical BIP-39 test vector (all-zero entropy).
    private let mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

    /// In-memory `WalletStorage` so the resolver path under test never
    /// touches the real macOS Keychain — CI runners often have a locked
    /// or permission-denied keychain session (`keychainError(-60008)` /
    /// `(-61)`), which is an environment failure, not what these tests
    /// cover. The lock mirrors the real storage's thread-safety: the
    /// resolver trampoline reads from a Rust worker thread.
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

        // `hasMnemonic` (the `canSign` preflight) delegates here.
        override func mnemonicAvailability(for walletId: Data) -> MnemonicAvailability {
            lock.lock()
            defer { lock.unlock() }
            return mnemonics[walletId] != nil ? .present : .absent
        }
    }

    /// A consistent breadcrumb (the path derives to the row's pubkey) signs via
    /// the resolver: the seed is read from the Keychain, the key is derived on
    /// demand, the binding accepts it, and a 65-byte ECDSA signature comes back —
    /// proving the resolver path carries a real signature with no stored scalar.
    func testResolverSignsForConsistentBreadcrumb() throws {
        let container = try DashModelContainer.createInMemory()
        let walletId = Data(repeating: 0x7A, count: 32)

        // Derive the identity-auth pubkey at (identity 3, key 2) from the seed so
        // the seeded row matches what the resolver derives at sign time.
        let wallet = try Wallet(mnemonic: mnemonic, network: .testnet)
        let path = try KeyDerivation.getIdentityAuthenticationPath(
            network: .testnet, identityIndex: 3, keyIndex: 2)
        let pubkey = try XCTUnwrap(Data(hexString: wallet.derivePublicKey(path: path)))
        XCTAssertEqual(pubkey.count, 33, "compressed secp256k1 pubkey")

        // Seed the resolver's source: the mnemonic in WalletStorage, keyed by walletId.
        let storage = InMemoryWalletStorage()
        try storage.storeMnemonic(mnemonic, for: walletId)
        defer { try? storage.deleteMnemonic(for: walletId) }

        // Seed the breadcrumb row (no stored scalar — only the path + walletId).
        let ctx = ModelContext(container)
        let row = PersistentPublicKey(
            keyId: 2, purpose: .authentication, securityLevel: .high,
            keyType: .ecdsaSecp256k1, publicKeyData: pubkey, identityId: "id1")
        row.walletId = walletId
        row.identityDerivationPath = path
        ctx.insert(row)
        try ctx.save()

        let signer = KeychainSigner(modelContainer: container, network: .testnet, storage: storage)

        // The preflight consults the same injected storage as the sign
        // path, so a resolver-signable key must report signable.
        XCTAssertTrue(signer.canSign(publicKey: pubkey, keyType: 0))

        let message = Data("identity state transition".utf8)
        let result = signer.signIdentityKeyOnDemand(publicKey: pubkey, keyType: 0, data: message)

        guard case .success(let signature)? = result else {
            XCTFail("expected a resolver signature, got \(String(describing: result))")
            return
        }
        // dashcore compact-recoverable ECDSA signature is 65 bytes.
        XCTAssertEqual(signature.count, 65)
    }

    /// A breadcrumb whose path does NOT derive to the row's pubkey (a corrupt /
    /// stale path) is rejected by the MF-2 binding BEFORE signing — a `.failure`
    /// the trampoline falls back from, never a valid-but-wrong-key signature.
    func testResolverRejectsWrongPathBeforeSigning() throws {
        let container = try DashModelContainer.createInMemory()
        let walletId = Data(repeating: 0x7B, count: 32)

        let wallet = try Wallet(mnemonic: mnemonic, network: .testnet)
        let truePath = try KeyDerivation.getIdentityAuthenticationPath(
            network: .testnet, identityIndex: 3, keyIndex: 2)
        let pubkey = try XCTUnwrap(Data(hexString: wallet.derivePublicKey(path: truePath)))
        // The row stores a DIFFERENT path that derives a different key.
        let wrongPath = try KeyDerivation.getIdentityAuthenticationPath(
            network: .testnet, identityIndex: 9, keyIndex: 9)

        let storage = InMemoryWalletStorage()
        try storage.storeMnemonic(mnemonic, for: walletId)
        defer { try? storage.deleteMnemonic(for: walletId) }

        let ctx = ModelContext(container)
        let row = PersistentPublicKey(
            keyId: 2, purpose: .authentication, securityLevel: .high,
            keyType: .ecdsaSecp256k1, publicKeyData: pubkey, identityId: "id1")
        row.walletId = walletId
        row.identityDerivationPath = wrongPath
        ctx.insert(row)
        try ctx.save()

        let signer = KeychainSigner(modelContainer: container, network: .testnet, storage: storage)
        let result = signer.signIdentityKeyOnDemand(
            publicKey: pubkey, keyType: 0, data: Data("x".utf8))

        guard case .failure? = result else {
            XCTFail("expected binding rejection, got \(String(describing: result))")
            return
        }
    }
}
