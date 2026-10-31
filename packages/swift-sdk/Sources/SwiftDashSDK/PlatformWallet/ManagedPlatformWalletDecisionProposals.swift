import Foundation

// MARK: - Decision proposals (protocol version 15)
//
// A decision proposal is a `proposal` document of the decision proposals
// system contract, so it is created and edited through the platform wallet's
// document create and replace, the same path every other document takes:
// the Rust side signs through the `KeychainSigner` and the signing key never
// crosses into Swift logic. rs-sdk builds the properties
// (`SDK.buildDecisionProposal`, `DecisionProposalDocument.edited`); these
// wrappers pass them on with the contract id and type name the FFI names.

extension ManagedPlatformWallet {
    /// Create + broadcast the decision proposal `proposal`, built by
    /// ``SDK/buildDecisionProposal(ownerIdentityId:params:)`` for
    /// `ownerIdentityId`. Returns the proposal's 32-byte id and the confirmed
    /// document's canonical JSON once Platform confirms the transition.
    ///
    /// Routes through ``createDocument(ownerIdentityId:contractId:documentType:propertiesJSON:signer:maxContestFund:)``
    /// with the proposal's ``DecisionProposalDocument/propertiesJSON()``. The
    /// create states the decision proposal fund on top of the fees; a proposal
    /// joins no contest, so no contest fund is stated.
    ///
    /// - Parameters:
    ///   - proposal: The proposal built for `ownerIdentityId`.
    ///   - ownerIdentityId: The proposer, the identity the proposal was built
    ///     for.
    ///   - signer: Signs the create; see `createDocument` for its lifetime
    ///     contract.
    public func createDecisionProposal(
        _ proposal: DecisionProposalDocument,
        ownerIdentityId: Identifier,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        // `createDocument` reads exactly 32 bytes of the identifier.
        guard ownerIdentityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("ownerIdentityId must be exactly 32 bytes")
        }
        let propertiesJSON = try proposal.propertiesJSON()
        return try await createDocument(
            ownerIdentityId: ownerIdentityId,
            contractId: DecisionProposals.contractIdentifier,
            documentType: DecisionProposals.documentTypeName,
            propertiesJSON: propertiesJSON,
            signer: signer
        )
    }

    /// Replace + broadcast the decision proposal `proposalId` with the text of
    /// `edited`, the stored proposal edited by
    /// ``DecisionProposalDocument/edited(_:)``. Returns the proposal's 32-byte
    /// id and the confirmed document's canonical JSON at its new revision.
    ///
    /// Routes through ``replaceDocument(ownerIdentityId:contractId:documentType:documentId:propertiesJSON:signingKeyId:signer:)``
    /// with the edited document's ``DecisionProposalDocument/propertiesJSON()``:
    /// the wallet fetches the stored proposal, sets these properties and moves
    /// it to the same next revision. Platform refuses the replace outside the
    /// proposal's edit window.
    ///
    /// - Parameters:
    ///   - edited: The stored proposal with its new text.
    ///   - ownerIdentityId: The proposer.
    ///   - proposalId: The proposal's 32-byte id.
    ///   - signingKeyId: The owner's AUTHENTICATION + ECDSA key to sign with.
    ///   - signer: Signs the replace; see `replaceDocument` for its lifetime
    ///     contract.
    public func replaceDecisionProposal(
        _ edited: DecisionProposalDocument,
        ownerIdentityId: Identifier,
        proposalId: Identifier,
        signingKeyId: UInt32,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        // `replaceDocument` reads exactly 32 bytes of each identifier.
        guard ownerIdentityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("ownerIdentityId must be exactly 32 bytes")
        }
        guard proposalId.count == 32 else {
            throw PlatformWalletError.invalidParameter("proposalId must be exactly 32 bytes")
        }
        let propertiesJSON = try edited.propertiesJSON()
        return try await replaceDocument(
            ownerIdentityId: ownerIdentityId,
            contractId: DecisionProposals.contractIdentifier,
            documentType: DecisionProposals.documentTypeName,
            documentId: proposalId,
            propertiesJSON: propertiesJSON,
            signingKeyId: signingKeyId,
            signer: signer
        )
    }
}
