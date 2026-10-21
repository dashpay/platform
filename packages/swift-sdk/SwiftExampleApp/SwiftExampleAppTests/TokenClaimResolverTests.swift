import XCTest
import SwiftData
import SwiftDashSDK
@testable import SwiftExampleApp

/// `TokenActionResolver`'s claim rules, and the default the claim form
/// starts on, for the three distribution kinds a token can carry.
///
/// The once-per-identity kind (protocol version 14) pays every identity a
/// fixed amount exactly once, so it makes strangers eligible where the other
/// two kinds would not. That is also what makes the default matter: a
/// stranger to a token with an owner-paid perpetual distribution plus a
/// once-per-identity one must not open the form on Perpetual, because Drive
/// charges for the rejected claim.
///
/// The kind has no column on `PersistentToken`, so every fixture here seeds
/// the owning contract's `serializedContract`: that JSON is where
/// `oncePerIdentityDistribution` is derived from.
@MainActor
final class TokenClaimResolverTests: XCTestCase {

    /// Distinct per fixture so two contracts in one test (or one per test)
    /// never share a row id.
    private var nextContractByte: UInt8 = 0x10

    private func makeContext() throws -> ModelContext {
        let container = try DashModelContainer.createInMemory()
        return ModelContext(container)
    }

    private func makeIdentity(
        byte: UInt8,
        in context: ModelContext
    ) -> PersistentIdentity {
        let identity = PersistentIdentity(
            identityId: Data(repeating: byte, count: 32),
            network: .testnet
        )
        context.insert(identity)
        return identity
    }

    /// Persist a contract whose single token declares the requested
    /// distributions, and hand back the token row.
    ///
    /// `perpetualRecipient` becomes the pinned `newTokensDestinationIdentity`
    /// the perpetual rules key off; `preProgrammedRecipients` are written as
    /// scheduled payout recipients. `oncePerIdentityAmount` goes into the
    /// contract JSON, since that is the only place the derived kind is read
    /// from.
    private func makeToken(
        oncePerIdentityAmount: String? = nil,
        perpetual: Bool = false,
        perpetualRecipient: PersistentIdentity? = nil,
        preProgrammedRecipients: [PersistentIdentity] = [],
        allowsChoosingDestination: Bool = true,
        in context: ModelContext
    ) throws -> PersistentToken {
        nextContractByte &+= 1
        let contractId = Data(repeating: nextContractByte, count: 32)

        var distributionRules: [String: Any] = [:]
        if let oncePerIdentityAmount {
            distributionRules["oncePerIdentityDistribution"] = [
                "$formatVersion": "0",
                "amount": oncePerIdentityAmount
            ]
        }
        let contractData: [String: Any] = [
            "tokens": [
                "0": [
                    "baseSupply": 0,
                    "distributionRules": distributionRules
                ]
            ]
        ]

        let contract = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: try JSONSerialization.data(
                withJSONObject: contractData,
                options: []
            ),
            network: .testnet
        )
        context.insert(contract)

        let token = PersistentToken(
            contractId: contractId,
            position: 0,
            name: "Fixture Token",
            baseSupply: "0"
        )
        token.dataContract = contract
        token.mintingAllowChoosingDestination = allowsChoosingDestination
        if perpetual {
            token.perpetualDistribution = TokenPerpetualDistribution()
        }
        if let perpetualRecipient {
            token.newTokensDestinationIdentity = perpetualRecipient.identityId
        }
        if !preProgrammedRecipients.isEmpty {
            var distribution = TokenPreProgrammedDistribution()
            distribution.distributionSchedule = preProgrammedRecipients.map { recipient in
                DistributionEvent(
                    triggerTime: Date(timeIntervalSince1970: 1_750_000_000),
                    amount: "7",
                    recipient: recipient.identityIdBase58
                )
            }
            token.preProgrammedDistribution = distribution
        }
        context.insert(token)
        try context.save()
        return token
    }

    // MARK: - Nothing to claim

    func testTokenWithoutAnyDistributionDeniesAndHasNoDefault() throws {
        let context = try makeContext()
        let identity = makeIdentity(byte: 0x01, in: context)
        let token = try makeToken(in: context)
        let claims = StubClaimStore()

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: identity, claims: claims),
            .denied(reason: "Token has no distribution schedule")
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: identity,
                claims: claims
            ),
            []
        )
        XCTAssertNil(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: identity,
                claims: claims
            )
        )
    }

    // MARK: - Once-per-identity makes strangers eligible

    func testStrangerMayClaimOncePerIdentityAlone() throws {
        let context = try makeContext()
        let identity = makeIdentity(byte: 0x02, in: context)
        let token = try makeToken(oncePerIdentityAmount: "5000", in: context)
        let claims = StubClaimStore()

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: identity, claims: claims),
            .allowed
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: identity,
                claims: claims
            ),
            [.oncePerIdentity]
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: identity,
                claims: claims
            ),
            .oncePerIdentity
        )
    }

    /// The regression this suite exists for: with a perpetual distribution
    /// alongside, a stranger is eligible only for the once-per-identity kind,
    /// so that is the only kind the form may offer and the one it must start
    /// on. Offering Perpetual by declaration order sends a claim Drive
    /// rejects as the wrong claimant, at the user's expense.
    func testStrangerPrefersOncePerIdentityOverPerpetual() throws {
        let context = try makeContext()
        let owner = makeIdentity(byte: 0x03, in: context)
        let stranger = makeIdentity(byte: 0x04, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            perpetual: true,
            perpetualRecipient: owner,
            in: context
        )
        let claims = StubClaimStore()

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: stranger, claims: claims),
            .allowed
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: stranger,
                claims: claims
            ),
            [.oncePerIdentity],
            "the perpetual payout is pinned to the owner, so it is not on offer here"
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: stranger,
                claims: claims
            ),
            .oncePerIdentity
        )
    }

    /// The pinned recipient is eligible for both kinds and keeps Drive's
    /// ordering: perpetual first.
    func testDesignatedRecipientPrefersPerpetual() throws {
        let context = try makeContext()
        let owner = makeIdentity(byte: 0x05, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            perpetual: true,
            perpetualRecipient: owner,
            in: context
        )
        let claims = StubClaimStore()

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: owner, claims: claims),
            .allowed
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: owner,
                claims: claims
            ),
            [.perpetual, .oncePerIdentity]
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: owner,
                claims: claims
            ),
            .perpetual
        )
    }

    /// A listed pre-programmed recipient is eligible for that kind, which
    /// outranks once-per-identity the same way Drive orders them.
    func testListedPreProgrammedRecipientPrefersPreProgrammed() throws {
        let context = try makeContext()
        let recipient = makeIdentity(byte: 0x06, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            preProgrammedRecipients: [recipient],
            in: context
        )
        let claims = StubClaimStore()

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: recipient, claims: claims),
            .allowed
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: recipient,
                claims: claims
            ),
            [.preProgrammed, .oncePerIdentity]
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: recipient,
                claims: claims
            ),
            .preProgrammed
        )
    }

    /// An identity absent from the schedule is offered the universal kind
    /// only, never the pre-programmed one it is not listed in.
    func testUnlistedIdentityPrefersOncePerIdentityOverPreProgrammed() throws {
        let context = try makeContext()
        let recipient = makeIdentity(byte: 0x07, in: context)
        let stranger = makeIdentity(byte: 0x08, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            preProgrammedRecipients: [recipient],
            in: context
        )
        let claims = StubClaimStore()

        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: stranger,
                claims: claims
            ),
            [.oncePerIdentity]
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: stranger,
                claims: claims
            ),
            .oncePerIdentity
        )
    }

    // MARK: - Already claimed

    func testAlreadyClaimedOncePerIdentityAloneIsDenied() throws {
        let context = try makeContext()
        let identity = makeIdentity(byte: 0x09, in: context)
        let token = try makeToken(oncePerIdentityAmount: "5000", in: context)
        let claims = StubClaimStore()
        claims.recordClaim(token: token, identity: identity)

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: identity, claims: claims),
            .denied(reason: "Already claimed the once-per-identity distribution")
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: identity,
                claims: claims
            ),
            []
        )
        XCTAssertNil(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: identity,
                claims: claims
            )
        )
    }

    /// The claim that was recorded belongs to one identity: another one on
    /// the same token is untouched.
    func testAlreadyClaimedIsPerIdentity() throws {
        let context = try makeContext()
        let claimed = makeIdentity(byte: 0x0A, in: context)
        let other = makeIdentity(byte: 0x0B, in: context)
        let token = try makeToken(oncePerIdentityAmount: "5000", in: context)
        let claims = StubClaimStore()
        claims.recordClaim(token: token, identity: claimed)

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: other, claims: claims),
            .allowed
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: other,
                claims: claims
            ),
            [.oncePerIdentity]
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: other,
                claims: claims
            ),
            .oncePerIdentity
        )
    }

    /// Spending the single claim does not take away an eligibility the
    /// identity holds through another kind, and only the spent kind leaves
    /// the list.
    func testAlreadyClaimedDesignatedRecipientStaysAllowedOnPerpetual() throws {
        let context = try makeContext()
        let owner = makeIdentity(byte: 0x0C, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            perpetual: true,
            perpetualRecipient: owner,
            in: context
        )
        let claims = StubClaimStore()
        claims.recordClaim(token: token, identity: owner)

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: owner, claims: claims),
            .allowed
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: owner,
                claims: claims
            ),
            [.perpetual]
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: owner,
                claims: claims
            ),
            .perpetual
        )
    }

    /// A stranger whose single claim is spent loses the only eligibility it
    /// had: nothing is left to offer, and the denial names the kind rather
    /// than falling through to the perpetual kind's "not the designated
    /// recipient" reason. Offering the perpetual kind as a fallback would
    /// send a claim that pays the pinned recipient, which Drive rejects at
    /// this identity's expense.
    func testAlreadyClaimedStrangerIsDeniedEvenWithPerpetualPresent() throws {
        let context = try makeContext()
        let owner = makeIdentity(byte: 0x0D, in: context)
        let stranger = makeIdentity(byte: 0x0E, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            perpetual: true,
            perpetualRecipient: owner,
            allowsChoosingDestination: false,
            in: context
        )
        let claims = StubClaimStore()
        claims.recordClaim(token: token, identity: stranger)

        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: stranger, claims: claims),
            .denied(reason: "Already claimed the once-per-identity distribution")
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: stranger,
                claims: claims
            ),
            [],
            "the perpetual kind is not a fallback: it pays the pinned recipient"
        )
        XCTAssertNil(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: stranger,
                claims: claims
            )
        )
    }

    // MARK: - The local record

    /// The store keys by network, token and identity, so none of the three
    /// bleeds into another.
    func testClaimStoreKeysByNetworkTokenAndIdentity() throws {
        let context = try makeContext()
        let identity = makeIdentity(byte: 0x0F, in: context)
        let otherIdentity = makeIdentity(byte: 0x11, in: context)
        let token = try makeToken(oncePerIdentityAmount: "5000", in: context)
        let otherToken = try makeToken(oncePerIdentityAmount: "5000", in: context)

        let suiteName = "TokenClaimResolverTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = OncePerIdentityClaimStore(defaults: defaults)

        XCTAssertFalse(store.hasClaimed(token: token, identity: identity))
        store.recordClaim(token: token, identity: identity)

        XCTAssertTrue(store.hasClaimed(token: token, identity: identity))
        XCTAssertFalse(store.hasClaimed(token: token, identity: otherIdentity))
        XCTAssertFalse(store.hasClaimed(token: otherToken, identity: identity))

        // A second store over the same defaults sees it: the record has to
        // survive the form being dismissed and reopened.
        let reopened = OncePerIdentityClaimStore(defaults: defaults)
        XCTAssertTrue(reopened.hasClaimed(token: token, identity: identity))
    }

    /// Drive's rejection is recognised from the consensus code the FFI hands
    /// over beside the message.
    func testAlreadyClaimedRejectionIsRecognisedFromTheConsensusCode() {
        let rejection = PlatformWalletError.consensusRejection(
            PlatformConsensusError(code: 40722, kind: .state),
            """
            Token operation failed: Token claim failed: state transition broadcast error: \
            identity '5r5MYEznyc9UtKQZpmM1DUisVDwtPhUpaBxNpeTBQEHi' already claimed the \
            once-per-identity distribution of token \
            '6vk7Xk3dLFdBfNkAvj6vSpk6NNAoBEMRkGbdBHSqzmPu' at 1750000000000
            """
        )
        XCTAssertTrue(OncePerIdentityClaimRejection.isAlreadyClaimed(rejection))
        XCTAssertEqual(OncePerIdentityClaimRejection.alreadyClaimedConsensusCode, 40722)
    }

    /// Any other rejection is a different code, including the neighbours in
    /// the token block: recording a claim that never happened would hide the
    /// kind from that identity for good, since the record is never cleared.
    func testOtherConsensusCodesAreNotTheAlreadyClaimedRejection() {
        let others: [UInt32] = [40721, 40723, 40108, 10000]
        for code in others {
            XCTAssertFalse(
                OncePerIdentityClaimRejection.isAlreadyClaimed(
                    PlatformWalletError.consensusRejection(
                        PlatformConsensusError(code: code, kind: .state),
                        "Token operation failed: Token claim failed"
                    )
                ),
                "consensus code \(code) is not the already-claimed rejection"
            )
        }

        // A failure that never reached Platform carries no verdict at all.
        XCTAssertFalse(
            OncePerIdentityClaimRejection.isAlreadyClaimed(
                PlatformWalletError.unknown("Token operation failed: Token claim failed: timeout")
            )
        )
        struct SomeOtherError: Error {}
        XCTAssertFalse(OncePerIdentityClaimRejection.isAlreadyClaimed(SomeOtherError()))
    }

    /// Text is not a signal. rs-dpp owns the wording, claim errors quote
    /// millisecond timestamps whose digits contain the code, and an error
    /// that only describes the rejection did not come from Platform refusing
    /// this claim.
    func testRenderedTextAloneIsNotTheAlreadyClaimedRejection() {
        let messagesThatOnlyDescribeIt = [
            """
            Token operation failed: Token claim failed: identity \
            '5r5MYEznyc9UtKQZpmM1DUisVDwtPhUpaBxNpeTBQEHi' already claimed the once-per-identity \
            distribution of token '6vk7Xk3dLFdBfNkAvj6vSpk6NNAoBEMRkGbdBHSqzmPu' at 1750000000000
            """,
            "TokenOncePerIdentityDistributionAlreadyClaimedError { token_id: .. }",
            "Token claim failed: consensus error 40722: claim rejected",
            "Token claim failed: 40722 at 1758140722000",
            "Token mint past max supply: 1758140722000"
        ]
        for message in messagesThatOnlyDescribeIt {
            XCTAssertFalse(
                OncePerIdentityClaimRejection.isAlreadyClaimed(
                    PlatformWalletError.unknown(message)
                ),
                "text must not stand in for the consensus code: \(message)"
            )
        }
    }
}

/// In-memory stand-in for `OncePerIdentityClaimStore`, so the resolver tests
/// never touch the shared `UserDefaults`.
private final class StubClaimStore: OncePerIdentityClaimRecording {
    private var claimed: Set<String> = []

    func hasClaimed(token: PersistentToken, identity: PersistentIdentity) -> Bool {
        claimed.contains(OncePerIdentityClaimStore.key(token: token, identity: identity))
    }

    func recordClaim(token: PersistentToken, identity: PersistentIdentity) {
        claimed.insert(OncePerIdentityClaimStore.key(token: token, identity: identity))
    }
}
