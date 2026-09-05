import CommonCrypto
import DashSDKFFI
import XCTest
@testable import SwiftDashSDK

/// Regression tests for non-English BIP-39 mnemonics (dashwallet-ios field
/// report, 2026-08-22): a French 12-word phrase validated fine
/// (`mnemonic_validate` tries every wordlist) but `Mnemonic.toSeed` and
/// `Wallet(mnemonic:)` failed with "Invalid Mnemonic: mnemonic contains an
/// unknown word (word 0)" because the key-wallet FFI they bind to parses with
/// a hardcoded English wordlist. That broke the DashSync→SwiftDashSDK upgrade
/// migration and "Import from Phrase" for every legacy localized wallet.
///
/// Expected seeds were generated with an independent oracle
/// (Python: `hashlib.pbkdf2_hmac('sha512', NFKD(phrase), NFKD('mnemonic'+pass),
/// 2048, 64)`), which reproduces the official BIP-39 English test vectors and
/// the derivation Electrum/DashSync use — the derivation that recovers real
/// user funds.
final class NonEnglishMnemonicTests: XCTestCase {

    // French mnemonic for entropy 000102030405060708090a0b0c0d0e0f, words
    // from the official BIP-39 French wordlist (NFKD-encoded, as published).
    private static let frenchPhrase =
        "abaisser agréable inductif agréable éligible achat bolide boucle amateur exister dérober bloquer"

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

    private func hex(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    // MARK: - Baseline: validation already accepts non-English phrases

    func testValidateAcceptsFrenchPhrase() {
        XCTAssertTrue(Mnemonic.validate(Self.frenchPhrase))
        XCTAssertTrue(Mnemonic.validate(Self.frenchPhraseNFC))
    }

    // MARK: - Seed derivation must accept every language validation accepts

    func testToSeedFrenchMatchesReferenceVector() throws {
        let seed = try Mnemonic.toSeed(mnemonic: Self.frenchPhrase)
        XCTAssertEqual(hex(seed), Self.frenchSeedHex)
    }

    func testToSeedFrenchNFCInputMatchesSameSeed() throws {
        let seed = try Mnemonic.toSeed(mnemonic: Self.frenchPhraseNFC)
        XCTAssertEqual(hex(seed), Self.frenchSeedHex)
    }

    func testToSeedFrenchWithPassphrase() throws {
        let seed = try Mnemonic.toSeed(mnemonic: Self.frenchPhrase, passphrase: "TREZOR")
        XCTAssertEqual(hex(seed), Self.frenchSeedTrezorHex)
    }

    // MARK: - Wallet construction must accept every language validation accepts

    func testWalletFromFrenchMnemonicDerivesSeedConsistentIds() throws {
        // `Wallet(mnemonic:)` previously threw KeyWalletError.invalidMnemonic
        // ("unknown word (word 0)") for any non-English phrase. It must
        // produce the same wallet as building from the phrase's BIP-39 seed.
        let fromMnemonic = try Wallet(mnemonic: Self.frenchPhrase, network: .mainnet)
        let fromSeed = try Wallet(
            seed: Data(Self.frenchSeedHex.hexToBytes()), network: .mainnet)
        XCTAssertEqual(try fromMnemonic.id, try fromSeed.id)

        // Wallet ids are network-scoped: same phrase, distinct id per network.
        let testnet = try Wallet(mnemonic: Self.frenchPhrase, network: .testnet)
        XCTAssertNotEqual(try fromMnemonic.id, try testnet.id)
    }

    func testWalletFromFrenchMnemonicNFCInputSameWallet() throws {
        let nfc = try Wallet(mnemonic: Self.frenchPhraseNFC, network: .mainnet)
        let nfkd = try Wallet(mnemonic: Self.frenchPhrase, network: .mainnet)
        XCTAssertEqual(try nfc.id, try nfkd.id)
    }

    // MARK: - English fast path is unchanged

    func testToSeedEnglishOfficialVectorUnchanged() throws {
        let seed = try Mnemonic.toSeed(mnemonic: Self.englishPhrase)
        XCTAssertEqual(hex(seed), Self.englishSeedHex)

        let trezor = try Mnemonic.toSeed(mnemonic: Self.englishPhrase, passphrase: "TREZOR")
        XCTAssertEqual(hex(trezor), Self.englishSeedTrezorHex)
    }

    func testWalletFromEnglishMnemonicMatchesSeedWallet() throws {
        let fromMnemonic = try Wallet(mnemonic: Self.englishPhrase, network: .mainnet)
        let fromSeed = try Wallet(
            seed: Data(Self.englishSeedHex.hexToBytes()), network: .mainnet)
        XCTAssertEqual(try fromMnemonic.id, try fromSeed.id)
    }

    // MARK: - Cross-implementation agreement

    func testRustMultiLanguageDerivationAgreesWithReferenceSeed() throws {
        // The platform wallet manager creates wallets through Rust's
        // language-auto-detecting parse (`parse_mnemonic_any_language`) and
        // rust-bip39's `to_seed`, while `Wallet(mnemonic:)`/`Mnemonic.toSeed`
        // reach the same seed through the Swift fallback. If the two ever
        // disagreed, wallet ids computed app-side would not match the wallets
        // the manager creates. Derive the BIP-32 master key from the French
        // phrase via the Rust path and compare with the master key computed
        // directly from the reference seed (HMAC-SHA512 keyed "Bitcoin seed").
        var secretKey = [UInt8](repeating: 0, count: 32)
        var chainCode = [UInt8](repeating: 0, count: 32)
        let result = Self.frenchPhrase.withCString { mnemonicPtr in
            "m".withCString { pathPtr in
                platform_wallet_derive_ext_priv_key_from_mnemonic(
                    mnemonicPtr, nil, Network.mainnet.ffiValue, pathPtr,
                    &secretKey, &chainCode, nil)
            }
        }
        try result.check()

        let seed = Data(Self.frenchSeedHex.hexToBytes())
        var hmac = [UInt8](repeating: 0, count: Int(CC_SHA512_DIGEST_LENGTH))
        let key = Array("Bitcoin seed".utf8)
        seed.withUnsafeBytes { seedBytes in
            CCHmac(CCHmacAlgorithm(kCCHmacAlgSHA512),
                   key, key.count,
                   seedBytes.baseAddress, seedBytes.count,
                   &hmac)
        }
        XCTAssertEqual(Array(secretKey), Array(hmac[0..<32]))
        XCTAssertEqual(Array(chainCode), Array(hmac[32..<64]))
    }

    // MARK: - Invalid input still refused

    func testToSeedGibberishStillThrows() {
        XCTAssertThrowsError(
            try Mnemonic.toSeed(mnemonic: "definitely not a bip39 phrase at all zz"))
    }

    func testWalletFromGibberishStillThrows() {
        XCTAssertThrowsError(
            try Wallet(mnemonic: "definitely not a bip39 phrase at all zz", network: .mainnet))
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
