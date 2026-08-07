import XCTest

@testable import SwiftDashSDK

/// `Account.derivePrivateKeyWIF(wallet:index:)` must apply the account's
/// derivation path exactly once.
///
/// The FFI resolves `Account::derivation_path()` itself — the full path from
/// the wallet master — so the wrapper has to hand it the master. It used to
/// take a `masterPath`, pre-derive that, and pass the result, which applied the
/// account path twice: provider voting keys came back from
/// `m/9'/5'/3'/1'/9'/5'/3'/1'/index` instead of `m/9'/5'/3'/1'/index`.
///
/// Nothing failed locally when that happened. The key was well-formed and
/// derived deterministically, so it round-tripped through WIF parsing and
/// signing without complaint — it simply wasn't the account's key at that
/// index. Only a counterparty holding the real key could tell: a masternode
/// vote signed with it is rejected as having no voter identity, because the
/// voter identity is derived from the signing key's own hash160.
///
/// So these pin account-based derivation against the explicit DIP-3 path,
/// which is the thing the doubling silently broke.
final class AccountDerivationPathTests: XCTestCase {
    /// Standard BIP39 test vector, matching the other wallet tests here.
    private let mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

    private func assertAccountDerivationMatchesPath(
        accountType: AccountType,
        pathPrefix: String,
        network: Network,
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws {
        let wallet = try Wallet(mnemonic: mnemonic, network: network)
        let account = try wallet.getAccount(type: accountType)

        // Indexes beyond 0 matter: the doubled path agreed with nothing, but a
        // single wrong level could still coincide at index 0 in principle.
        for index in [UInt32(0), 1, 19] {
            let viaAccount = try account.derivePrivateKeyWIF(wallet: wallet, index: index)
            let viaPath = try wallet.derivePrivateKey(path: "\(pathPrefix)/\(index)")
            XCTAssertEqual(
                viaAccount, viaPath,
                """
                account-based derivation at index \(index) disagrees with \
                \(pathPrefix)/\(index) — the account path is being applied the \
                wrong number of times
                """,
                file: file, line: line)
        }
    }

    func testProviderVotingKeysDeriveAtTheDIP3Path() throws {
        try assertAccountDerivationMatchesPath(
            accountType: .providerVotingKeys,
            pathPrefix: "m/9'/5'/3'/1'",
            network: .mainnet)
    }

    func testProviderOwnerKeysDeriveAtTheDIP3Path() throws {
        try assertAccountDerivationMatchesPath(
            accountType: .providerOwnerKeys,
            pathPrefix: "m/9'/5'/3'/2'",
            network: .mainnet)
    }

    /// The account resolves its own path, including coin type, so testnet must
    /// land on `1'` without the caller saying so.
    func testProviderVotingKeysUseTheTestnetCoinType() throws {
        try assertAccountDerivationMatchesPath(
            accountType: .providerVotingKeys,
            pathPrefix: "m/9'/1'/3'/1'",
            network: .testnet)
    }

    /// The specific regression: deriving against the account root rather than
    /// the master must NOT reproduce account-based derivation. If these ever
    /// match, the wrapper has gone back to pre-deriving the account path.
    func testDoubledAccountPathIsNotWhatWeDerive() throws {
        let wallet = try Wallet(mnemonic: mnemonic, network: .mainnet)
        let account = try wallet.getAccount(type: .providerVotingKeys)

        let correct = try account.derivePrivateKeyWIF(wallet: wallet, index: 19)
        let doubled = try wallet.derivePrivateKey(path: "m/9'/5'/3'/1'/9'/5'/3'/1'/19")
        XCTAssertNotEqual(
            correct, doubled,
            "account-based derivation is applying the account path twice again")
    }
}
