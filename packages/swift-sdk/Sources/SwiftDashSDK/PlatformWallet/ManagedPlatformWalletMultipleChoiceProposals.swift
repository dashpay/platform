import Foundation
import DashSDKFFI

// MARK: - Multiple choice proposals (protocol version 15)
//
// Creating and editing a multiple choice proposal and entering a choice are
// platform wallet operations (`platform_wallet_multiple_choice_proposal_create`,
// `_edit` and `_enter_choice`, over
// `IdentityWallet::create_multiple_choice_proposal_with_signer`,
// `edit_multiple_choice_proposal_with_signer` and `enter_choice_with_signer`).
// rs-sdk and the wallet decide everything: the documents' properties, the
// contest fund a choice states, the signing key, refusing an edit that
// changes nothing, and the revision an edit is put at. These wrappers marshal
// the arguments in and the confirmed document out; the signing key never
// crosses into Swift logic, the `KeychainSigner` trampoline services the
// signature on demand.

extension ManagedPlatformWallet {
    /// Create + broadcast a multiple choice proposal of `ownerIdentityId`
    /// asking what `params` says. Returns the proposal's 32-byte id and the
    /// confirmed document's canonical JSON (the shape
    /// ``createDocument(ownerIdentityId:contractId:documentType:propertiesJSON:signer:maxContestFund:)``
    /// returns) once Platform confirms the transition.
    ///
    /// rs-sdk builds the proposal. Platform charges the owner the multiple
    /// choice proposal processing fee
    /// (``SDK/multipleChoiceProposalProcessingFee()``, 0.2 Dash) on top of
    /// the fees; the create does not state it. The wallet signs with the
    /// owner's first AUTHENTICATION + ECDSA key at a security level the
    /// `multipleChoiceProposal` type accepts that `signer` holds. Platform
    /// judges the title, the description document and the url; nothing is
    /// checked here. A network below protocol version 15 has no decision
    /// proposals, which is an error.
    ///
    /// Lifetime contract: as `createDocument`, the `signer` is pinned with
    /// `withExtendedLifetime` across the synchronous FFI call (Rust holds a
    /// `passUnretained` ctx pointer to it).
    ///
    /// - Parameters:
    ///   - ownerIdentityId: The proposer's 32-byte identity id.
    ///   - params: The question.
    ///   - signer: Signs the create.
    /// - Throws: `PlatformWalletError.invalidParameter` for an id that is not
    ///   32 bytes or a string holding a NUL character; a refusal by Platform
    ///   as `PlatformWalletError.consensusRejection`, with its consensus code.
    public func createMultipleChoiceProposal(
        ownerIdentityId: Identifier,
        params: MultipleChoiceProposalParams,
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
            // params synchronously inside the call, so the `withFFI` scope is
            // long enough.
            let result = try withExtendedLifetime(signer) {
                try ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    try params.withFFI { paramsPtr in
                        proposalIdBytes.withUnsafeMutableBufferPointer { outBp in
                            platform_wallet_multiple_choice_proposal_create(
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

            try result.check()
            // On success the Rust side always writes the JSON; a null pointer
            // is an FFI contract violation, not an empty proposal.
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "multiple_choice_proposal_create returned no canonical document JSON"
                )
            }
            return (Data(proposalIdBytes), String(cString: jsonPtr))
        }.value
    }

    /// Give the multiple choice proposal `proposalId` of `ownerIdentityId`
    /// what `edit` changes, and broadcast the replace. Returns the confirmed
    /// document's canonical JSON, at its new revision, once Platform confirms
    /// the transition.
    ///
    /// The wallet reads the proposal once, applies the edit with rs-sdk
    /// (which refuses an edit that changes nothing) and puts it at the
    /// revision after the one read, signed with a key chosen as
    /// ``createMultipleChoiceProposal(ownerIdentityId:params:signer:)``
    /// chooses it. It never reads the proposal again before the put: if
    /// another edit lands in between, Platform refuses this one instead of
    /// overwriting it (consensus code 40106, `InvalidDocumentRevisionError`),
    /// and the caller reads the proposal again before offering the edit anew.
    /// Platform also refuses an edit more than two hours after the proposal
    /// was created, and one that leaves it with neither a description
    /// document nor a url.
    ///
    /// Lifetime contract: as `createDocument`, the `signer` is pinned with
    /// `withExtendedLifetime` across the synchronous FFI call.
    ///
    /// - Parameters:
    ///   - ownerIdentityId: The proposer's 32-byte identity id.
    ///   - proposalId: The proposal's 32-byte id.
    ///   - edit: What changes.
    ///   - signer: Signs the replace.
    /// - Throws: `PlatformWalletError.notFound` when there is no such
    ///   proposal; `PlatformWalletError.invalidParameter` for an id that is
    ///   not 32 bytes, a string holding a NUL character, a proposal of another
    ///   identity or an edit that changes nothing; a refusal by Platform as
    ///   `PlatformWalletError.consensusRejection`, with its consensus code.
    public func editMultipleChoiceProposal(
        ownerIdentityId: Identifier,
        proposalId: Identifier,
        edit: MultipleChoiceProposalEdit,
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
        return try await Task.detached(priority: .userInitiated) {
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }

            // Pin `signer` for the whole FFI call (see `createDocument`).
            let result = try withExtendedLifetime(signer) {
                try ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    try proposalBytes.withUnsafeBufferPointer { proposalBp -> PlatformWalletFFIResult in
                        try edit.withFFI { editPtr in
                            platform_wallet_multiple_choice_proposal_edit(
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

            try result.check()
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "multiple_choice_proposal_edit returned no canonical document JSON"
                )
            }
            return String(cString: jsonPtr)
        }.value
    }

    /// Enter the choice `params` describes, owned by `ownerIdentityId`, for
    /// the multiple choice proposal `proposalId`, and broadcast it. Returns
    /// the choice's 32-byte id (the id a masternode vote for it names) and
    /// the confirmed document's canonical JSON once Platform confirms the
    /// transition.
    ///
    /// rs-sdk builds the choice; its put reads the fund of the contest among
    /// the proposal's choices just before it is signed and states it
    /// (``SDK/multipleChoiceProposalChoiceFundToJoin(proposalId:)`` reads the
    /// same amount), which the owner pays on top of the fees. The first
    /// choice opens the vote. Platform refuses a choice for a proposal that
    /// is not open from anyone but its owner, and one entered once the vote's
    /// join window closed. A choice is never edited.
    ///
    /// Lifetime contract: as `createDocument`, the `signer` is pinned with
    /// `withExtendedLifetime` across the synchronous FFI call.
    ///
    /// - Parameters:
    ///   - ownerIdentityId: The 32-byte id of the identity entering the
    ///     choice.
    ///   - proposalId: The proposal's 32-byte id.
    ///   - params: The answer.
    ///   - signer: Signs the create.
    /// - Throws: `PlatformWalletError.invalidParameter` for an id that is not
    ///   32 bytes or a string holding a NUL character; a refusal by Platform
    ///   as `PlatformWalletError.consensusRejection`, with its consensus code.
    public func enterMultipleChoiceProposalChoice(
        ownerIdentityId: Identifier,
        proposalId: Identifier,
        params: ProposalChoiceParams,
        signer: KeychainSigner
    ) async throws -> (Identifier, String) {
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
        return try await Task.detached(priority: .userInitiated) {
            var choiceIdBytes = [UInt8](repeating: 0, count: 32)
            var documentJsonPtr: UnsafeMutablePointer<CChar>? = nil
            defer { if let p = documentJsonPtr { platform_wallet_string_free(p) } }

            // Pin `signer` for the whole FFI call (see `createDocument`).
            let result = try withExtendedLifetime(signer) {
                try ownerBytes.withUnsafeBufferPointer { ownerBp -> PlatformWalletFFIResult in
                    try proposalBytes.withUnsafeBufferPointer { proposalBp -> PlatformWalletFFIResult in
                        try params.withFFI { paramsPtr in
                            choiceIdBytes.withUnsafeMutableBufferPointer { outBp in
                                platform_wallet_multiple_choice_proposal_enter_choice(
                                    handle,
                                    ownerBp.baseAddress!,
                                    proposalBp.baseAddress!,
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

            try result.check()
            guard let jsonPtr = documentJsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "multiple_choice_proposal_enter_choice returned no canonical document JSON"
                )
            }
            return (Data(choiceIdBytes), String(cString: jsonPtr))
        }.value
    }
}
