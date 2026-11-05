import CommonCrypto
import DashSDKFFI
import XCTest
@testable import SwiftDashSDK

/// Regression tests for non-English BIP-39 mnemonics (dashwallet-ios field
/// report, 2026-08-22): a French 12-word phrase validated fine but seed
/// derivation failed with "Invalid Mnemonic: mnemonic contains an unknown
/// word (word 0)" because the FFI parse path used a hardcoded English
/// wordlist. That broke the DashSync→SwiftDashSDK upgrade migration and
/// "Import from Phrase" for every legacy localized wallet. These tests pin
/// that the platform wallet's derivation auto-detects the wordlist.
///
/// Expected seeds were generated with an independent oracle
/// (Python: `hashlib.pbkdf2_hmac('sha512', NFKD(phrase), NFKD('mnemonic'+pass),
/// 2048, 64)`), which reproduces the official BIP-39 English test vectors and
/// the derivation Electrum/DashSync use — the derivation that recovers real
/// user funds.
final class NonEnglishMnemonicTests: XCTestCase {

    // French mnemonic for entropy 000102030405060708090a0b0c0d0e0f, words
    // from the official BIP-39 French wordlist, which publishes its non-ASCII
    // words compatibility-decomposed. Decompose explicitly rather than relying
    // on the literal: a source file carries whichever form its editor saved,
    // and a precomposed one silently makes this fixture and the NFC fixture
    // below the same bytes, which is the only thing that tells them apart.
    private static let frenchPhrase =
        "abaisser agréable inductif agréable éligible achat bolide boucle amateur exister dérober bloquer"
            .decomposedStringWithCompatibilityMapping

    // The same phrase with precomposed accents (NFC) — what an iOS keyboard
    // actually produces. Seed derivation must treat both forms identically.
    private static let frenchPhraseNFC =
        NonEnglishMnemonicTests.frenchPhrase.precomposedStringWithCanonicalMapping

    private static let frenchSeedHex =
        "b70232fad2698ee7236b5f789e1566157f41e9b0a22b4dfa0c3325172a6fd851" +
        "3e0d552a12c335737275847d5b25a24bfaad97bdb4d98541901d3bd2a9cbfcf1"

    private static let frenchSeedTrezorHex =
        "984ede340ea47fbf2794c9dcde0c4e2e92bf16a5e172083e0c734835c33c6f66" +
        "7a2c635ce38b0819fab9397c683692cc6f28523072d80b96e031022bbb532992"

    // Official BIP-39 English test vector (entropy 00…00, passphrase-less
    // seed cross-checked against the same oracle) — guards that the English
    // fast path is unchanged.
    private static let englishPhrase =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

    private static let englishSeedHex =
        "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc1" +
        "9a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4"

    private static let englishSeedTrezorHex =
        "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e5349553" +
        "1f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04"

    // MARK: - The two French fixtures must stay distinct encodings

    func testFrenchFixturesAreDistinctEncodingsOfTheSamePhrase() {
        let decomposed = Array(Self.frenchPhrase.utf8)
        let precomposed = Array(Self.frenchPhraseNFC.utf8)

        // Swift compares Strings by canonical equivalence, so the two fixtures
        // are `==` and only their encoded bytes distinguish them. Those bytes
        // are what the wrappers hand to the FFI, so every test below that
        // claims to cover both forms covers only one if they ever coincide.
        XCTAssertNotEqual(decomposed, precomposed)
        XCTAssertEqual(
            decomposed,
            Array(Self.frenchPhrase.decomposedStringWithCompatibilityMapping.utf8))
        XCTAssertEqual(
            precomposed,
            Array(Self.frenchPhraseNFC.precomposedStringWithCanonicalMapping.utf8))
    }

    // MARK: - Seed derivation must accept every language

    func testFrenchMasterKeyMatchesReferenceSeed() throws {
        try assertMasterKey(phrase: Self.frenchPhrase, matchesSeedHex: Self.frenchSeedHex)
    }

    func testFrenchNFCInputDerivesSameMasterKey() throws {
        try assertMasterKey(phrase: Self.frenchPhraseNFC, matchesSeedHex: Self.frenchSeedHex)
    }

    func testFrenchWithPassphraseMatchesReferenceSeed() throws {
        try assertMasterKey(
            phrase: Self.frenchPhrase, passphrase: "TREZOR",
            matchesSeedHex: Self.frenchSeedTrezorHex)
    }

    // MARK: - English fast path is unchanged

    func testEnglishOfficialVectorUnchanged() throws {
        try assertMasterKey(phrase: Self.englishPhrase, matchesSeedHex: Self.englishSeedHex)
        try assertMasterKey(
            phrase: Self.englishPhrase, passphrase: "TREZOR",
            matchesSeedHex: Self.englishSeedTrezorHex)
    }

    // MARK: - Validation, wallet ids and addresses accept every language

    func testValidateAcceptsFrenchPhrase() {
        XCTAssertTrue(Mnemonic.validate(Self.frenchPhrase))
        XCTAssertTrue(Mnemonic.validate(Self.frenchPhraseNFC))
    }

    func testWalletIdMatchesPinnedVectors() throws {
        // dashwallet-ios keys its wipe and recovery checks on these ids, so they must never drift.
        // Pinned from `Wallet(mnemonic:network:).id` before the `KeyWallet` classes were removed.
        let vectors: [(phrase: String, network: Network, id: String)] = [
            (Self.frenchPhrase, .mainnet,
             "446a4d98c6937d73ceb088f6800fddf4b8404734d7bb1946a130eb48aac02830"),
            (Self.frenchPhrase, .testnet,
             "b9dc02956dd9e3e65169d38b02cdd5a38d492f5171aab1420590297bfc4d382e"),
            (Self.englishPhrase, .mainnet,
             "0b91f36de2613a410303e8309b4f92a150738ae018695d2030b33e64ccea7b2e"),
            (Self.englishPhrase, .testnet,
             "0c9776ef403f2da80ef4cb3884a96d591805934effc0e014eadd2bb98a94ea6c"),
        ]
        for vector in vectors {
            XCTAssertEqual(
                try Mnemonic.walletId(for: vector.phrase, network: vector.network),
                Data(vector.id.hexToBytes()), "\(vector.network)")
        }

        let mainnet = try Mnemonic.walletId(for: Self.frenchPhrase, network: .mainnet)
        XCTAssertEqual(try Mnemonic.walletId(for: Self.frenchPhraseNFC, network: .mainnet), mainnet)
        XCTAssertNotEqual(try Mnemonic.walletId(for: Self.frenchPhrase, network: .devnet), mainnet)
    }

    func testFirstReceiveAddressMatchesPinnedVectors() throws {
        // The English address is the official vector. The French ones are pinned from
        // key-wallet-ffi's seed derivation before it was removed.
        let vectors: [(phrase: String, network: Network, address: String)] = [
            (Self.englishPhrase, .mainnet, "XoJA8qE3N2Y3jMLEtZ3vcN42qseZ8LvFf5"),
            (Self.frenchPhrase, .mainnet, "XvdJKc1Qf9QWHfHTp8rpiNZ3ZWLGcFnoqV"),
            (Self.frenchPhrase, .testnet, "yiz7bBUGQUJ4cTfUWqwwPq7g8XTc7ynEEw"),
        ]
        for vector in vectors {
            XCTAssertEqual(
                try Mnemonic.firstReceiveAddress(for: vector.phrase, network: vector.network),
                vector.address, "\(vector.network)")
        }
    }

    // MARK: - Invalid input still refused

    func testValidateRefusesGibberish() throws {
        XCTAssertFalse(Mnemonic.validate("definitely not a bip39 phrase at all zz"))
        XCTAssertFalse(Mnemonic.validate(Self.englishPhrase + "\u{0}"))
        XCTAssertThrowsError(
            try Mnemonic.walletId(for: "definitely not a bip39 phrase at all zz", network: .mainnet))

        // The FFI reads a C string, so an unguarded helper would answer for
        // the valid prefix before the NUL.
        let nulPhrase = Self.englishPhrase + "\u{0}junk"
        XCTAssertThrowsError(try Mnemonic.walletId(for: nulPhrase, network: .mainnet))
        XCTAssertThrowsError(try Mnemonic.firstReceiveAddress(for: nulPhrase, network: .mainnet))
        XCTAssertThrowsError(
            try Mnemonic.firstReceiveAddress(
                for: "definitely not a bip39 phrase at all zz", network: .mainnet))
        let validAddress = try Mnemonic.firstReceiveAddress(for: Self.englishPhrase, network: .mainnet)
        XCTAssertTrue(Address.validate(validAddress, network: .mainnet))
        XCTAssertFalse(Address.validate(validAddress + "\u{0}junk", network: .mainnet))
    }

    func testGibberishStillThrows() {
        XCTAssertThrowsError(
            try masterKey(phrase: "definitely not a bip39 phrase at all zz", passphrase: nil))
    }

    // MARK: - Helpers

    /// Derives the BIP-32 master key of `phrase` through the platform wallet's
    /// language-auto-detecting parse, the path every wallet the manager
    /// creates goes through.
    private func masterKey(
        phrase: String, passphrase: String?
    ) throws -> (secretKey: [UInt8], chainCode: [UInt8]) {
        var secretKey = [UInt8](repeating: 0, count: 32)
        var chainCode = [UInt8](repeating: 0, count: 32)
        let result = phrase.withCString { mnemonicPtr -> PlatformWalletFFIResult in
            "m".withCString { pathPtr -> PlatformWalletFFIResult in
                if let passphrase {
                    return passphrase.withCString { passphrasePtr in
                        platform_wallet_derive_ext_priv_key_from_mnemonic(
                            mnemonicPtr, passphrasePtr, Network.mainnet.ffiValue, pathPtr,
                            &secretKey, &chainCode, nil)
                    }
                }
                return platform_wallet_derive_ext_priv_key_from_mnemonic(
                    mnemonicPtr, nil, Network.mainnet.ffiValue, pathPtr,
                    &secretKey, &chainCode, nil)
            }
        }
        try result.check()
        return (secretKey, chainCode)
    }

    /// Asserts that `phrase` derives the master key computed directly from the
    /// reference seed (HMAC-SHA512 keyed "Bitcoin seed").
    private func assertMasterKey(
        phrase: String, passphrase: String? = nil, matchesSeedHex seedHex: String,
        file: StaticString = #filePath, line: UInt = #line
    ) throws {
        let derived = try masterKey(phrase: phrase, passphrase: passphrase)

        let seed = Data(seedHex.hexToBytes())
        var hmac = [UInt8](repeating: 0, count: Int(CC_SHA512_DIGEST_LENGTH))
        let key = Array("Bitcoin seed".utf8)
        seed.withUnsafeBytes { seedBytes in
            CCHmac(CCHmacAlgorithm(kCCHmacAlgSHA512),
                   key, key.count,
                   seedBytes.baseAddress, seedBytes.count,
                   &hmac)
        }
        XCTAssertEqual(derived.secretKey, Array(hmac[0..<32]), file: file, line: line)
        XCTAssertEqual(derived.chainCode, Array(hmac[32..<64]), file: file, line: line)
    }
}

private extension String {
    func hexToBytes() -> [UInt8] {
        var bytes: [UInt8] = []
        bytes.reserveCapacity(count / 2)
        var index = startIndex
        while index < endIndex {
            let next = self.index(index, offsetBy: 2)
            bytes.append(UInt8(self[index..<next], radix: 16)!)
            index = next
        }
        return bytes
    }
}
