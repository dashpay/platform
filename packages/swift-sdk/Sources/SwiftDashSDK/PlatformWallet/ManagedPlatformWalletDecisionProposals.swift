import Foundation
import DashSDKFFI

// MARK: - Decision proposals (protocol version 15)
//
// Creating and editing a decision proposal are platform wallet operations
// (`platform_wallet_decision_proposal_create` / `_edit`, over
// `IdentityWallet::create_decision_proposal_with_signer` /
// `edit_decision_proposal_with_signer`). rs-sdk and the wallet decide
// everything: the proposal's properties, the decision proposal fund the
// create states, the signing key, refusing an edit that changes nothing, and
// the revision an edit is put at. These wrappers marshal the arguments in and
// the confirmed document out; the signing key never crosses into Swift logic,
// the `KeychainSigner` trampoline services the signature on demand.

extension ManagedPlatformWallet {
    /// Create + broadcast a decision proposal of `ownerIdentityId` asking what
    /// `params` says. Returns the proposal's 32-byte id and the confirmed
    /// document's canonical JSON (the shape
    /// ``createDocument(ownerIdentityId:contractId:documentType:propertiesJSON:signer:maxContestFund:)``
    /// returns) once Platform confirms the transition.
    ///
    /// rs-sdk builds the proposal; its create states the decision proposal
    /// fund (0.5 Dash), which the owner pays on top of the fees. The wallet
    /// signs with the owner's first AUTHENTICATION + ECDSA key at a security
    /// level the `proposal` type accepts that `signer` holds. Platform judges
    /// the text, the rule and the end date; nothing is checked here. A
    /// network below protocol version 15 has no decision proposals, which is
    /// an error.
    ///
    /// Lifetime contract: as `createDocument`, the `signer` is pinned with
    /// `withExtendedLifetime` across the synchronous FFI call (Rust holds a
    /// `passUnretained` ctx pointer to it).
    ///
    /// - Parameters:
    ///   - ownerIdentityId: The proposer's 32-byte identity id.
    ///   - params: The question and the rule.
    ///   - signer: Signs the create.
    /// - Throws: `PlatformWalletError.invalidParameter` for an id that is not
    ///   32 bytes or a string holding a NUL character; a refusal by Platform
    ///   as `PlatformWalletError.consensusRejection`, with its consensus code.
    public func createDecisionProposal(
        ownerIdentityId: Identifier,
        params: DecisionProposalParams,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
        // The FFI reads exactly 32 bytes of the identifier.
        guard ownerIdentityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("ownerIdentityId must be exactly 32 bytes")
        }
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes = [UInt8](ownerIdentityId)
        return try await Task.detached(priority: .userInitiated) {
            var proposalIdBytes = [UInt8](repeating: 0, count: 32)
            // Receives an owned canonical-document JSON C string on success;
            // freed with `platform_wallet_string_free` below.
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }

            // Pin `signer` for the whole FFI call (see `createDocument` for
            // why a bare `_ = signer` is unreliable under -O). Rust reads the
            // C strings synchronously inside the call, so the
            // `withDecisionProposalCString` scopes are long enough.
            let result = try withExtendedLifetime(signer) {
                try ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    try withDecisionProposalCString(
                        params.title, field: "title", invalid: PlatformWalletError.invalidParameter
                    ) { titlePtr in
                        try withDecisionProposalCString(
                            params.description, field: "description",
                            invalid: PlatformWalletError.invalidParameter
                        ) { descriptionPtr in
                            try withDecisionProposalCString(
                                params.url, field: "url", invalid: PlatformWalletError.invalidParameter
                            ) { urlPtr in
                                let ffiParams = DecisionProposalParamsFFI(
                                    title: titlePtr,
                                    description: descriptionPtr,
                                    url: urlPtr,
                                    pass_percent: params.passPercent,
                                    minimum_turnout_percent: params.minimumTurnoutPercent,
                                    end_date_ms: params.endDateMs
                                )
                                return withUnsafePointer(to: ffiParams) { paramsPtr in
                                    proposalIdBytes.withUnsafeMutableBufferPointer { outBp in
                                        platform_wallet_decision_proposal_create(
                                            handle,
                                            ownerBp.baseAddress!,
                                            paramsPtr,
                                            signerHandle,
                                            outBp.baseAddress!,
                                            &documentJsonPtr
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            // On success the Rust side always writes the JSON; a null pointer
            // is an FFI contract violation, not an empty proposal.
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "decision_proposal_create returned no canonical document JSON"
                )
            }
            return (Data(proposalIdBytes), String(cString: jsonPtr))
        }.value
    }

    /// Give the decision proposal `proposalId` of `ownerIdentityId` the new
    /// text `edit` sets, and broadcast the replace. Returns the confirmed
    /// document's canonical JSON, at its new revision, once Platform confirms
    /// the transition.
    ///
    /// The wallet reads the proposal once, applies the edit with rs-sdk
    /// (which refuses an edit that changes nothing) and puts it at the
    /// revision after the one read, signed with a key chosen as
    /// ``createDecisionProposal(ownerIdentityId:params:signer:)`` chooses it.
    /// It never reads the proposal again before the put: if another edit
    /// lands in between, Platform refuses this one instead of overwriting it
    /// (consensus code 40106, `InvalidDocumentRevisionError`), and the caller
    /// reads the proposal again before offering the edit anew. Platform also
    /// refuses an edit more than a day after the proposal was created (or a
    /// fifth of the time it runs, if that is shorter).
    ///
    /// Lifetime contract: as `createDocument`, the `signer` is pinned with
    /// `withExtendedLifetime` across the synchronous FFI call.
    ///
    /// - Parameters:
    ///   - ownerIdentityId: The proposer's 32-byte identity id.
    ///   - proposalId: The proposal's 32-byte id.
    ///   - edit: The new text.
    ///   - signer: Signs the replace.
    /// - Throws: `PlatformWalletError.notFound` when there is no such
    ///   proposal; `PlatformWalletError.invalidParameter` for an id that is
    ///   not 32 bytes, a string holding a NUL character, a proposal of another
    ///   identity or an edit that changes nothing; a refusal by Platform as
    ///   `PlatformWalletError.consensusRejection`, with its consensus code.
    public func editDecisionProposal(
        ownerIdentityId: Identifier,
        proposalId: Identifier,
        edit: DecisionProposalEdit,
        signer: KeychainSigner
    ) async throws -> String {
        // The FFI reads exactly 32 bytes of each identifier.
        guard ownerIdentityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("ownerIdentityId must be exactly 32 bytes")
        }
        guard proposalId.count == 32 else {
            throw PlatformWalletError.invalidParameter("proposalId must be exactly 32 bytes")
        }
        let handle = self.handle
        let signerHandle = signer.handle
        let ownerBytes = [UInt8](ownerIdentityId)
        let proposalBytes = [UInt8](proposalId)
        // `DecisionProposalEditFFI` says "remove the url" with a flag beside
        // a null `url`.
        let url: String?
        let removeURL: Bool
        switch edit.url {
        case .keep:
            url = nil
            removeURL = false
        case .set(let value):
            url = value
            removeURL = false
        case .remove:
            url = nil
            removeURL = true
        }
        return try await Task.detached(priority: .userInitiated) {
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }

            // Pin `signer` for the whole FFI call (see `createDocument`).
            let result = try withExtendedLifetime(signer) {
                try ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    try proposalBytes.withUnsafeBufferPointer { proposalBp -> PlatformWalletFFIResult in
                        try withDecisionProposalCString(
                            edit.title, field: "title", invalid: PlatformWalletError.invalidParameter
                        ) { titlePtr in
                            try withDecisionProposalCString(
                                edit.description, field: "description",
                                invalid: PlatformWalletError.invalidParameter
                            ) { descriptionPtr in
                                try withDecisionProposalCString(
                                    url, field: "url", invalid: PlatformWalletError.invalidParameter
                                ) { urlPtr in
                                    let ffiEdit = DecisionProposalEditFFI(
                                        title: titlePtr,
                                        description: descriptionPtr,
                                        url: urlPtr,
                                        remove_url: removeURL
                                    )
                                    return withUnsafePointer(to: ffiEdit) { editPtr in
                                        platform_wallet_decision_proposal_edit(
                                            handle,
                                            ownerBp.baseAddress!,
                                            proposalBp.baseAddress!,
                                            editPtr,
                                            signerHandle,
                                            &documentJsonPtr
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }

            try result.check()
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "decision_proposal_edit returned no canonical document JSON"
                )
            }
            return String(cString: jsonPtr)
        }.value
    }
}
