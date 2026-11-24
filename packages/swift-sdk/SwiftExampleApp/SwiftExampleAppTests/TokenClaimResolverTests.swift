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

    /// The stored `distributionRecipient` string for a perpetual distribution
    /// that pays `identity`, in the shape `DataContractParser` writes.
    private func identityRecipient(_ identity: PersistentIdentity) -> String {
        TokenDistributionRecipient.identity(identity.identityIdBase58)
    }

    /// Persist a contract whose single token declares the requested
    /// distributions, and hand back the token row.
    ///
    /// `perpetualRecipient`, when set, gives the token a perpetual
    /// distribution whose stored `distributionRecipient` is that string
    /// (`identityRecipient(_:)`, `"ContractOwner"`, ...), which is what the
    /// perpetual claim rules key off. `mintDestination` and
    /// `allowsChoosingDestination` are the minting settings, kept separate
    /// so tests can show they do not decide who claims. `contractOwner`
    /// becomes the contract's `ownerId`, and `withContract: false` leaves
    /// the token with no `dataContract` at all (which also means no
    /// once-per-identity kind, since that is derived from the contract).
    /// `preProgrammedRecipients` are written as scheduled payout recipients.
    /// `oncePerIdentityAmount` goes into the contract JSON, since that is
    /// the only place the derived kind is read from.
    private func makeToken(
        oncePerIdentityAmount: String? = nil,
        perpetualRecipient: String? = nil,
        mintDestination: PersistentIdentity? = nil,
        contractOwner: PersistentIdentity? = nil,
        withContract: Bool = true,
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
            ownerId: contractOwner?.identityId,
            network: .testnet
        )
        if withContract {
            context.insert(contract)
        }

        let token = PersistentToken(
            contractId: contractId,
            position: 0,
            name: "Fixture Token",
            baseSupply: "0"
        )
        if withContract {
            token.dataContract = contract
        }
        token.mintingAllowChoosingDestination = allowsChoosingDestination
        if let perpetualRecipient {
            token.perpetualDistribution = TokenPerpetualDistribution(
                distributionRecipient: perpetualRecipient
            )
        }
        if let mintDestination {
            token.newTokensDestinationIdentity = mintDestination.identityId
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
            perpetualRecipient: identityRecipient(owner),
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
            "the perpetual payout goes to the owner, so it is not on offer here"
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

    /// The designated perpetual recipient is eligible for both kinds and
    /// keeps Drive's ordering: perpetual first.
    func testDesignatedRecipientPrefersPerpetual() throws {
        let context = try makeContext()
        let owner = makeIdentity(byte: 0x05, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            perpetualRecipient: identityRecipient(owner),
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

    /// With only a pre-programmed distribution and no once-per-identity one,
    /// an identity the schedule does not name has nothing to claim: the row
    /// says so, the form offers nothing and starts on no kind.
    func testUnlistedIdentityOnPreProgrammedOnlyTokenIsDenied() throws {
        let context = try makeContext()
        let recipient = makeIdentity(byte: 0x2A, in: context)
        let stranger = makeIdentity(byte: 0x2B, in: context)
        let token = try makeToken(
            preProgrammedRecipients: [recipient],
            in: context
        )

        assertClaim(
            token: token,
            identity: stranger,
            claims: StubClaimStore(),
            resolves: .denied(reason: "Not a recipient of any pre-programmed release"),
            offers: []
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
            perpetualRecipient: identityRecipient(owner),
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
    /// send a claim that pays the designated recipient, which Drive rejects
    /// at this identity's expense.
    func testAlreadyClaimedStrangerIsDeniedEvenWithPerpetualPresent() throws {
        let context = try makeContext()
        let owner = makeIdentity(byte: 0x0D, in: context)
        let stranger = makeIdentity(byte: 0x0E, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            perpetualRecipient: identityRecipient(owner),
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
            "the perpetual kind is not a fallback: it pays the designated recipient"
        )
        XCTAssertNil(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: stranger,
                claims: claims
            )
        )
    }

    // MARK: - Perpetual recipient is the distribution's, not minting's

    /// Asserts that `resolveClaim`, `claimableDistributions` and
    /// `preferredClaimDistribution` agree for one identity.
    private func assertClaim(
        token: PersistentToken,
        identity: PersistentIdentity,
        claims: StubClaimStore,
        resolves expected: TokenActionPermission,
        offers expectedKinds: [TokenDistributionType],
        _ message: String = "",
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertEqual(
            TokenActionResolver.resolveClaim(token: token, identity: identity, claims: claims),
            expected,
            message,
            file: file,
            line: line
        )
        XCTAssertEqual(
            TokenActionResolver.claimableDistributions(
                token: token,
                identity: identity,
                claims: claims
            ),
            expectedKinds,
            message,
            file: file,
            line: line
        )
        XCTAssertEqual(
            TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: identity,
                claims: claims
            ),
            expectedKinds.first,
            message,
            file: file,
            line: line
        )
    }

    /// Drive compares the claimant against the perpetual distribution's
    /// recipient only, so the recipient may claim even when minting goes to
    /// someone else and the contract forbids choosing a destination.
    func testPerpetualRecipientIdentityMayClaimWhateverTheMintDestination() throws {
        let context = try makeContext()
        let recipient = makeIdentity(byte: 0x20, in: context)
        let minter = makeIdentity(byte: 0x21, in: context)
        let token = try makeToken(
            perpetualRecipient: identityRecipient(recipient),
            mintDestination: minter,
            allowsChoosingDestination: false,
            in: context
        )

        assertClaim(
            token: token,
            identity: recipient,
            claims: StubClaimStore(),
            resolves: .allowed,
            offers: [.perpetual]
        )
    }

    /// Being the minting destination is not being the perpetual recipient,
    /// whether or not the contract lets minters choose a destination.
    func testMintDestinationAloneMayNotClaimPerpetual() throws {
        let context = try makeContext()
        let recipient = makeIdentity(byte: 0x22, in: context)
        let minter = makeIdentity(byte: 0x23, in: context)

        for allowsChoosing in [true, false] {
            let token = try makeToken(
                perpetualRecipient: identityRecipient(recipient),
                mintDestination: minter,
                allowsChoosingDestination: allowsChoosing,
                in: context
            )
            assertClaim(
                token: token,
                identity: minter,
                claims: StubClaimStore(),
                resolves: .denied(reason: "Not the designated distribution recipient"),
                offers: [],
                "allowsChoosingDestination = \(allowsChoosing)"
            )
        }
    }

    /// A contract-owner recipient pays the contract's owner and nobody else.
    func testContractOwnerRecipientAllowsOnlyTheOwner() throws {
        let context = try makeContext()
        let owner = makeIdentity(byte: 0x24, in: context)
        let other = makeIdentity(byte: 0x25, in: context)
        let token = try makeToken(
            perpetualRecipient: TokenDistributionRecipient.contractOwner.rawValue,
            mintDestination: other,
            contractOwner: owner,
            in: context
        )
        let claims = StubClaimStore()

        assertClaim(
            token: token,
            identity: owner,
            claims: claims,
            resolves: .allowed,
            offers: [.perpetual]
        )
        assertClaim(
            token: token,
            identity: other,
            claims: claims,
            resolves: .denied(reason: "Not the designated distribution recipient"),
            offers: [],
            "the mint destination is not the contract owner"
        )
    }

    /// With no owner on record the app cannot tell who a contract-owner
    /// recipient is, so it offers nothing rather than guess.
    func testContractOwnerRecipientWithUnknownOwnerIsDenied() throws {
        let context = try makeContext()
        let identity = makeIdentity(byte: 0x26, in: context)
        let unknownReason = "Distribution recipient (contract owner) unknown locally"

        let ownerMissing = try makeToken(
            perpetualRecipient: TokenDistributionRecipient.contractOwner.rawValue,
            in: context
        )
        assertClaim(
            token: ownerMissing,
            identity: identity,
            claims: StubClaimStore(),
            resolves: .denied(reason: unknownReason),
            offers: [],
            "contract loaded without an ownerId"
        )

        let contractMissing = try makeToken(
            perpetualRecipient: TokenDistributionRecipient.contractOwner.rawValue,
            withContract: false,
            in: context
        )
        assertClaim(
            token: contractMissing,
            identity: identity,
            claims: StubClaimStore(),
            resolves: .denied(reason: unknownReason),
            offers: [],
            "token with no contract row"
        )
    }

    /// Evonode payouts depend on chain state this app does not prove, so
    /// the claim is never offered, whoever asks.
    func testEvonodesByParticipationRecipientIsDenied() throws {
        let context = try makeContext()
        let identity = makeIdentity(byte: 0x27, in: context)
        let token = try makeToken(
            perpetualRecipient: TokenDistributionRecipient.evonodesByParticipation.rawValue,
            mintDestination: identity,
            in: context
        )

        assertClaim(
            token: token,
            identity: identity,
            claims: StubClaimStore(),
            resolves: .denied(
                reason: "Distribution pays evonodes by participation; eligibility not evaluated by this app"
            ),
            offers: []
        )
    }

    /// A recipient this app does not know, the pre-#4824 placeholder, and an
    /// `Identity:` value with no id are all refused with their own reason.
    func testUnrecognisedLegacyAndEmptyIdentityRecipientsAreDenied() throws {
        let context = try makeContext()
        let identity = makeIdentity(byte: 0x28, in: context)
        let cases: [(recipient: String, reason: String)] = [
            ("futureVariant", "Distribution recipient 'futureVariant' not recognised by this app"),
            (
                "AllEqualShare",
                "Distribution recipient not recorded; delete the contract and load it again "
                    + "(this also clears its locally saved documents)"
            ),
            (TokenDistributionRecipient.identity(""), "Not the designated distribution recipient")
        ]

        for (recipient, reason) in cases {
            let token = try makeToken(
                perpetualRecipient: recipient,
                mintDestination: identity,
                contractOwner: identity,
                in: context
            )
            assertClaim(
                token: token,
                identity: identity,
                claims: StubClaimStore(),
                resolves: .denied(reason: reason),
                offers: [],
                "recipient '\(recipient)'"
            )
        }
    }

    /// A perpetual kind the identity cannot claim does not hide the
    /// once-per-identity kind it can.
    func testStrangerGetsOncePerIdentityBesideEvonodePerpetual() throws {
        let context = try makeContext()
        let stranger = makeIdentity(byte: 0x29, in: context)
        let token = try makeToken(
            oncePerIdentityAmount: "5000",
            perpetualRecipient: TokenDistributionRecipient.evonodesByParticipation.rawValue,
            in: context
        )

        assertClaim(
            token: token,
            identity: stranger,
            claims: StubClaimStore(),
            resolves: .allowed,
            offers: [.oncePerIdentity]
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
