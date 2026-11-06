import XCTest
import DashSDKFFI

@testable import SwiftDashSDK

/// `ManagedPlatformWallet.checkIdentityKey` / `checkIdentityKeys`, the
/// bridges to the wallet-scoped Rust checks. Rust decides every outcome,
/// including refusing another wallet's recovery phrase; those outcomes
/// are pinned by the `identity_key_wallet_check` FFI tests against a
/// real wallet handle.
///
/// What only Swift can show is the decoding of a successful result: the
/// imported enums, the fixed-size byte arrays, the key order, and that
/// every decoded value is a copy that outlives the Rust free. Those tests
/// decode C results built by hand through the same projections the
/// bridges call (`identityKeyWalletCheck(from:)`,
/// `identityWalletCheck(from:keyCount:)`). The test owns those buffers,
/// so it wipes and releases them itself, the way the Rust free does,
/// and then checks the decoded values are unchanged.
///
/// A registered wallet handle needs a configured `PlatformWalletManager`,
/// so the bridge tests themselves pin what happens without one: they
/// throw and never report a verdict.
@MainActor
final class IdentityKeyWalletCheckTests: XCTestCase {

    private let walletId = Data(repeating: 0x07, count: 32)

    private let path = "m/9'/1'/5'/0'/0'/1'/2'"
    private let privateKey = (1...32).map { UInt8($0) }
    private let publicKeyHash = (0xA0..<0xB4).map { UInt8($0) }
    private let derivedPublicKey = [0x02] + (0x10..<0x30).map { UInt8($0) }

    /// A key check in its C form, as Rust hands it over, whose path and
    /// key data buffers this test owns.
    private final class OwnedKeyCheck {
        var ffi = IdentityKeyWalletCheckFFI()
        private var pathBuffer: UnsafeMutablePointer<CChar>?
        private var dataBuffer: UnsafeMutablePointer<UInt8>?
        private var dataCount = 0

        init(
            outcome: IdentityKeyWalletCheckOutcomeFFI,
            path: String? = nil,
            derivedPublicKeyData: [UInt8] = [],
            privateKey: [UInt8] = [],
            publicKeyHash: [UInt8] = []
        ) {
            ffi.outcome = outcome
            if let path {
                pathBuffer = strdup(path)
                ffi.derivation_path = pathBuffer
            }
            if !derivedPublicKeyData.isEmpty {
                dataCount = derivedPublicKeyData.count
                let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: dataCount)
                buffer.initialize(from: derivedPublicKeyData, count: dataCount)
                dataBuffer = buffer
                ffi.derived_public_key_data = buffer
                ffi.derived_public_key_data_len = UInt(dataCount)
            }
            withUnsafeMutableBytes(of: &ffi.private_key_bytes) { $0.copyBytes(from: privateKey) }
            withUnsafeMutableBytes(of: &ffi.public_key_hash) { $0.copyBytes(from: publicKeyHash) }
        }

        /// Zero every byte the C value holds or points to, then release
        /// the buffers, as the Rust free does.
        func wipeAndRelease() {
            if let pathBuffer {
                memset(pathBuffer, 0, strlen(pathBuffer))
                free(pathBuffer)
            }
            if let dataBuffer {
                memset(dataBuffer, 0, dataCount)
                dataBuffer.deallocate()
            }
            pathBuffer = nil
            dataBuffer = nil
            ffi = IdentityKeyWalletCheckFFI()
        }
    }

    private func matchingKey() -> OwnedKeyCheck {
        OwnedKeyCheck(
            outcome: IDENTITY_KEY_WALLET_CHECK_OUTCOME_FFI_MATCHES,
            path: path, privateKey: privateKey, publicKeyHash: publicKeyHash)
    }

    private func mismatchingKey() -> OwnedKeyCheck {
        OwnedKeyCheck(
            outcome: IDENTITY_KEY_WALLET_CHECK_OUTCOME_FFI_MISMATCH,
            path: path, derivedPublicKeyData: derivedPublicKey)
    }

    private var expectedMatch: ManagedPlatformWallet.IdentityKeyWalletCheck {
        .matches(
            derivationPath: path,
            privateKeyData: Data(privateKey),
            publicKeyHashHex: Data(publicKeyHash).toHexString())
    }

    private var expectedMismatch: ManagedPlatformWallet.IdentityKeyWalletCheck {
        .mismatch(derivationPath: path, derivedPublicKeyData: Data(derivedPublicKey))
    }

    /// An identity check in its C form, whose key array this test owns.
    private func identityCheck(
        ownership: IdentityWalletOwnershipFFI,
        keys: [OwnedKeyCheck]
    ) -> (IdentityWalletCheckFFI, release: () -> Void) {
        guard !keys.isEmpty else {
            return (IdentityWalletCheckFFI(ownership: ownership, keys: nil, keys_len: 0), {})
        }
        let array = UnsafeMutablePointer<IdentityKeyWalletCheckFFI>.allocate(capacity: keys.count)
        array.initialize(from: keys.map(\.ffi), count: keys.count)
        let check = IdentityWalletCheckFFI(ownership: ownership, keys: array, keys_len: UInt(keys.count))
        return (check, {
            keys.forEach { $0.wipeAndRelease() }
            array.update(repeating: IdentityKeyWalletCheckFFI(), count: keys.count)
            array.deinitialize(count: keys.count)
            array.deallocate()
        })
    }

    // MARK: - One key

    func testShouldDecodeAMatchingKeyAndKeepItsValuesAfterTheSourceIsWiped() throws {
        let key = matchingKey()

        let check = try ManagedPlatformWallet.identityKeyWalletCheck(from: key.ffi)
        key.wipeAndRelease()

        // The 32 private key bytes and 20 hash bytes come out whole and in
        // order, and the copies survive the source being wiped.
        XCTAssertEqual(check, expectedMatch)
        guard case let .matches(_, privateKeyData, publicKeyHashHex) = check else {
            return XCTFail("expected matches, got \(check)")
        }
        XCTAssertEqual(privateKeyData.count, 32)
        XCTAssertEqual(publicKeyHashHex, "a0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3")
    }

    func testShouldDecodeAMismatchingKeyAndKeepItsValuesAfterTheSourceIsWiped() throws {
        let key = mismatchingKey()

        let check = try ManagedPlatformWallet.identityKeyWalletCheck(from: key.ffi)
        key.wipeAndRelease()

        XCTAssertEqual(check, expectedMismatch)
    }

    func testShouldDecodeANotDerivedKey() throws {
        let key = OwnedKeyCheck(outcome: IDENTITY_KEY_WALLET_CHECK_OUTCOME_FFI_NOT_DERIVED)

        XCTAssertEqual(try ManagedPlatformWallet.identityKeyWalletCheck(from: key.ffi), .notDerived)
        key.wipeAndRelease()
    }

    func testShouldDecodeANotComparedKey() throws {
        let key = OwnedKeyCheck(outcome: IDENTITY_KEY_WALLET_CHECK_OUTCOME_FFI_NOT_COMPARED)

        XCTAssertEqual(try ManagedPlatformWallet.identityKeyWalletCheck(from: key.ffi), .notCompared)
        key.wipeAndRelease()
    }

    func testShouldRefuseAnUnknownKeyCheckOutcome() throws {
        let key = OwnedKeyCheck(outcome: IdentityKeyWalletCheckOutcomeFFI(rawValue: 99))
        defer { key.wipeAndRelease() }

        XCTAssertThrowsError(try ManagedPlatformWallet.identityKeyWalletCheck(from: key.ffi)) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("expected deserialization, got \(error)")
            }
        }
    }

    // MARK: - A whole identity

    func testShouldDecodeAnIdentityCheckInKeyOrderAndKeepItAfterTheSourceIsWiped() throws {
        let keys = [
            matchingKey(),
            mismatchingKey(),
            OwnedKeyCheck(outcome: IDENTITY_KEY_WALLET_CHECK_OUTCOME_FFI_NOT_DERIVED),
            OwnedKeyCheck(outcome: IDENTITY_KEY_WALLET_CHECK_OUTCOME_FFI_NOT_COMPARED),
        ]
        let (ffi, release) = identityCheck(ownership: IDENTITY_WALLET_OWNERSHIP_FFI_OWNED, keys: keys)

        let check = try ManagedPlatformWallet.identityWalletCheck(from: ffi, keyCount: keys.count)
        release()

        XCTAssertEqual(check.keys, [expectedMatch, expectedMismatch, .notDerived, .notCompared])
        XCTAssertEqual(check.ownership, .owned)
    }

    func testShouldDecodeEveryIdentityOwnership() throws {
        for (ffiOwnership, ownership) in [
            (IDENTITY_WALLET_OWNERSHIP_FFI_OWNED, ManagedPlatformWallet.IdentityWalletOwnership.owned),
            (IDENTITY_WALLET_OWNERSHIP_FFI_NOT_OWNED, .notOwned),
            (IDENTITY_WALLET_OWNERSHIP_FFI_UNDETERMINED, .undetermined),
        ] {
            let keys = [mismatchingKey()]
            let (ffi, release) = identityCheck(ownership: ffiOwnership, keys: keys)

            let check = try ManagedPlatformWallet.identityWalletCheck(from: ffi, keyCount: 1)
            release()

            XCTAssertEqual(check.ownership, ownership)
            XCTAssertEqual(check.keys, [expectedMismatch])
        }
    }

    func testShouldDecodeAnIdentityCheckWithNoKeys() throws {
        let (ffi, release) = identityCheck(ownership: IDENTITY_WALLET_OWNERSHIP_FFI_UNDETERMINED, keys: [])
        defer { release() }

        let check = try ManagedPlatformWallet.identityWalletCheck(from: ffi, keyCount: 0)

        XCTAssertEqual(check, ManagedPlatformWallet.IdentityWalletCheck(keys: [], ownership: .undetermined))
    }

    func testShouldRefuseAnIdentityCheckWithAResultCountOtherThanTheKeysSent() throws {
        let (ffi, release) = identityCheck(
            ownership: IDENTITY_WALLET_OWNERSHIP_FFI_OWNED, keys: [matchingKey()])
        defer { release() }

        for keyCount in [0, 2] {
            XCTAssertThrowsError(
                try ManagedPlatformWallet.identityWalletCheck(from: ffi, keyCount: keyCount)
            ) { error in
                guard case PlatformWalletError.deserialization = error else {
                    return XCTFail("expected deserialization, got \(error)")
                }
            }
        }
    }

    func testShouldRefuseAnUnknownIdentityOwnership() throws {
        let (ffi, release) = identityCheck(
            ownership: IdentityWalletOwnershipFFI(rawValue: 7),
            keys: [OwnedKeyCheck(outcome: IDENTITY_KEY_WALLET_CHECK_OUTCOME_FFI_NOT_DERIVED)])
        defer { release() }

        XCTAssertThrowsError(try ManagedPlatformWallet.identityWalletCheck(from: ffi, keyCount: 1)) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("expected deserialization, got \(error)")
            }
        }
    }

    // MARK: - The bridges without a registered wallet

    private func foreignMasterInput() -> ManagedPlatformWallet.IdentityKeyCheckInput {
        ManagedPlatformWallet.IdentityKeyCheckInput(
            keyId: 0, keyType: .ecdsaSecp256k1,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.master.rawValue,
            publicKeyData: Data(repeating: 0x02, count: 33), disabledAt: nil)
    }

    /// Without a wallet Rust holds there is no seed binding to check, so
    /// the identity check throws instead of calling the identity foreign.
    func testShouldThrowForAWalletRustDoesNotHold() throws {
        let wallet = ManagedPlatformWallet(handle: NULL_HANDLE, walletId: walletId)

        XCTAssertThrowsError(
            try wallet.checkIdentityKeys(
                identityIndex: 0, keys: [foreignMasterInput()], unreadableKeyCount: 0))
        XCTAssertThrowsError(
            try wallet.checkIdentityKey(
                identityIndex: 0, keyId: 0, keyType: .ecdsaSecp256k1,
                purpose: KeyPurpose.authentication.rawValue,
                securityLevel: SecurityLevel.master.rawValue,
                publicKeyData: Data(repeating: 0x02, count: 33)))
    }

    func testShouldRefuseANegativeUnreadableKeyCount() throws {
        let wallet = ManagedPlatformWallet(handle: NULL_HANDLE, walletId: walletId)

        XCTAssertThrowsError(
            try wallet.checkIdentityKeys(
                identityIndex: 0, keys: [foreignMasterInput()], unreadableKeyCount: -1)
        ) { error in
            guard case PlatformWalletError.invalidParameter = error else {
                return XCTFail("expected invalidParameter, got \(error)")
            }
        }
    }
}
