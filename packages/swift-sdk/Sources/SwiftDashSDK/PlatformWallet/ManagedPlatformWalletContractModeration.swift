import Foundation
import DashSDKFFI

// MARK: - Contract moderation writes (protocol version 14)
//
// Moderating a contract's users and documents, and asking who may, are
// platform wallet operations (`platform_wallet_contract_moderation_offer`,
// `platform_wallet_contract_user_moderate`,
// `platform_wallet_contract_document_delete` / `_restore` /
// `_change_fields` / `_propose_settled_deletion` and
// `platform_wallet_contract_team_action_approve`, over the matching
// `IdentityWallet` methods). rs-sdk, the wallet and Platform decide
// everything: the signing key, who may moderate what, and whether the target,
// document or team action admits it. These wrappers marshal the arguments in
// and the JSON out; the signing key never crosses into Swift logic, the
// `KeychainSigner` trampoline services the signature on demand.

extension ManagedPlatformWallet {
    /// Which of this wallet's identities may moderate `contractId`, and what
    /// each may do: the moderations of an identity, what it may do on each
    /// document type, and whether it approves team actions.
    ///
    /// Rust reads the contract and, when it declares elected moderation, the
    /// team of its seated charter. A host offers a moderation only to the
    /// identities this names; Platform still checks it when it is sent.
    ///
    /// It names identities by role, not by key: a named identity whose
    /// signer holds no CRITICAL authentication key without contract bounds
    /// is still named, and its moderation throws when sent; show that error
    /// where the moderation was asked for.
    ///
    /// - Parameter contractId: The contract's 32-byte id.
    /// - Throws: `PlatformWalletError.invalidParameter` for an id that is not
    ///   32 bytes; `PlatformWalletError.notFound` when Platform holds no such
    ///   contract; a failed read as the error its result code maps to.
    public func contractModerationOffer(contractId: Identifier) async throws -> ContractModerationOffer {
        let contract = try Self.moderationIdentifier(contractId, "contractId")
        let json = try await moderationCall("contract_moderation_offer", signer: nil) { handle, _, out in
            contract.withUnsafeBufferPointer { contractBp in
                platform_wallet_contract_moderation_offer(handle, contractBp.baseAddress!, out)
            }
        }
        return try ContractModerationOffer.decode(fromJSON: json)
    }

    /// Send one moderation of `targetIdentityId` on `contractId`, signed by
    /// `moderatorIdentityId`, an identity this wallet holds, and return the
    /// target's status on the list it edited as the proof shows it.
    ///
    /// - Parameters:
    ///   - suspendUntilMs: For a suspension, the block time it lasts until,
    ///     in milliseconds; ignored by every other action.
    ///   - reason: Stored with a ban, a suspension and a warning; the
    ///     reversals carry none.
    /// - Throws: `PlatformWalletError.invalidParameter` for an id that is not
    ///   32 bytes or a suspension without an end;
    ///   `PlatformWalletError.identityNotFound` when this wallet holds no
    ///   such signing identity; a refusal by Platform as
    ///   `PlatformWalletError.consensusRejection`, with its consensus code.
    public func moderateContractUser(
        moderatorIdentityId: Identifier,
        contractId: Identifier,
        targetIdentityId: Identifier,
        action: ContractUserModerationAction,
        suspendUntilMs: UInt64?,
        reason: ContractModerationReasonInput,
        signer: KeychainSigner
    ) async throws -> ModeratedContractUser {
        let moderator = try Self.moderationIdentifier(moderatorIdentityId, "moderatorIdentityId")
        let contract = try Self.moderationIdentifier(contractId, "contractId")
        let target = try Self.moderationIdentifier(targetIdentityId, "targetIdentityId")
        let reasonDocument = try reason.reasonDocumentId.map { try Self.moderationIdentifier($0, "reasonDocumentId") }
        let reasonText = reason.text
        let tag = action.ffiTag
        let json = try await moderationCall("contract_user_moderate", signer: signer) { handle, signerHandle, out in
            withOptionalPointer(to: suspendUntilMs) { untilPtr in
                withOptionalBytes(reasonDocument) { reasonDocumentPtr in
                    reasonText.withCString { reasonTextPtr in
                        moderator.withUnsafeBufferPointer { moderatorBp in
                            contract.withUnsafeBufferPointer { contractBp in
                                target.withUnsafeBufferPointer { targetBp in
                                    platform_wallet_contract_user_moderate(
                                        handle,
                                        moderatorBp.baseAddress!,
                                        contractBp.baseAddress!,
                                        targetBp.baseAddress!,
                                        tag,
                                        untilPtr,
                                        reasonTextPtr,
                                        reasonDocumentPtr,
                                        signerHandle,
                                        out
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
        return try ModeratedContractUser.decode(fromJSON: json)
    }

    /// Delete document `documentId` of `documentTypeName` on `contractId` as
    /// a moderator, signed by `moderatorIdentityId`, an identity this wallet
    /// holds. Returns the removal record the deletion left, and the document
    /// as it was just before, which ``restoreContractDocument`` takes back.
    ///
    /// - Throws: as ``moderateContractUser``; `PlatformWalletError.notFound`
    ///   when Platform holds no such contract.
    public func deleteContractDocument(
        moderatorIdentityId: Identifier,
        contractId: Identifier,
        documentTypeName: String,
        documentId: Identifier,
        reason: ContractModerationReasonInput,
        signer: KeychainSigner
    ) async throws -> ContractDocumentDeletion {
        let moderator = try Self.moderationIdentifier(moderatorIdentityId, "moderatorIdentityId")
        let contract = try Self.moderationIdentifier(contractId, "contractId")
        let document = try Self.moderationIdentifier(documentId, "documentId")
        let reasonDocument = try reason.reasonDocumentId.map { try Self.moderationIdentifier($0, "reasonDocumentId") }
        let reasonText = reason.text
        let json = try await moderationCall("contract_document_delete", signer: signer) { handle, signerHandle, out in
            withOptionalBytes(reasonDocument) { reasonDocumentPtr in
                reasonText.withCString { reasonTextPtr in
                    documentTypeName.withCString { typePtr in
                        moderator.withUnsafeBufferPointer { moderatorBp in
                            contract.withUnsafeBufferPointer { contractBp in
                                document.withUnsafeBufferPointer { documentBp in
                                    platform_wallet_contract_document_delete(
                                        handle,
                                        moderatorBp.baseAddress!,
                                        contractBp.baseAddress!,
                                        typePtr,
                                        documentBp.baseAddress!,
                                        reasonTextPtr,
                                        reasonDocumentPtr,
                                        signerHandle,
                                        out
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
        return try ContractDocumentDeletion.decode(fromJSON: json)
    }

    /// Restore a document of `documentTypeName` on `contractId` a moderator
    /// deleted, signed by `moderatorIdentityId`, an identity this wallet
    /// holds. `documentBytes` is the document as it was:
    /// ``ContractDocumentDeletion/documentData`` of its deletion.
    ///
    /// - Throws: as ``moderateContractUser``; `invalidParameter` for empty
    ///   bytes or bytes that are not a document of the type.
    public func restoreContractDocument(
        moderatorIdentityId: Identifier,
        contractId: Identifier,
        documentTypeName: String,
        documentBytes: Data,
        signer: KeychainSigner
    ) async throws -> ContractDocumentRestore {
        let moderator = try Self.moderationIdentifier(moderatorIdentityId, "moderatorIdentityId")
        let contract = try Self.moderationIdentifier(contractId, "contractId")
        guard !documentBytes.isEmpty else {
            throw PlatformWalletError.invalidParameter("documentBytes is empty")
        }
        let bytes = [UInt8](documentBytes)
        let json = try await moderationCall("contract_document_restore", signer: signer) { handle, signerHandle, out in
            documentTypeName.withCString { typePtr in
                moderator.withUnsafeBufferPointer { moderatorBp in
                    contract.withUnsafeBufferPointer { contractBp in
                        bytes.withUnsafeBufferPointer { bytesBp in
                            platform_wallet_contract_document_restore(
                                handle,
                                moderatorBp.baseAddress!,
                                contractBp.baseAddress!,
                                typePtr,
                                bytesBp.baseAddress!,
                                UInt(bytesBp.count),
                                signerHandle,
                                out
                            )
                        }
                    }
                }
            }
        }
        return try ContractDocumentRestore.decode(fromJSON: json)
    }

    /// Set the moderator fields `fieldsJSON` names on document `documentId`
    /// of `documentTypeName` on `contractId`, signed by
    /// `moderatorIdentityId`, an identity this wallet holds, and return the
    /// document as it now stands.
    ///
    /// - Parameter fieldsJSON: A JSON object keyed by top-level property
    ///   name, `null` removing the field; Rust reads its values under the
    ///   document type as a document create reads its properties.
    /// - Throws: as ``moderateContractUser``; `invalidParameter` for fields
    ///   that are not a JSON object.
    public func changeContractDocumentFields(
        moderatorIdentityId: Identifier,
        contractId: Identifier,
        documentTypeName: String,
        documentId: Identifier,
        fieldsJSON: String,
        reason: ContractModerationReasonInput,
        signer: KeychainSigner
    ) async throws -> ContractDocumentFieldsChange {
        let moderator = try Self.moderationIdentifier(moderatorIdentityId, "moderatorIdentityId")
        let contract = try Self.moderationIdentifier(contractId, "contractId")
        let document = try Self.moderationIdentifier(documentId, "documentId")
        let reasonDocument = try reason.reasonDocumentId.map { try Self.moderationIdentifier($0, "reasonDocumentId") }
        let reasonText = reason.text
        let json = try await moderationCall("contract_document_change_fields", signer: signer) { handle, signerHandle, out in
            withOptionalBytes(reasonDocument) { reasonDocumentPtr in
                reasonText.withCString { reasonTextPtr in
                    fieldsJSON.withCString { fieldsPtr in
                        documentTypeName.withCString { typePtr in
                            moderator.withUnsafeBufferPointer { moderatorBp in
                                contract.withUnsafeBufferPointer { contractBp in
                                    document.withUnsafeBufferPointer { documentBp in
                                        platform_wallet_contract_document_change_fields(
                                            handle,
                                            moderatorBp.baseAddress!,
                                            contractBp.baseAddress!,
                                            typePtr,
                                            documentBp.baseAddress!,
                                            fieldsPtr,
                                            reasonTextPtr,
                                            reasonDocumentPtr,
                                            signerHandle,
                                            out
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        return try ContractDocumentFieldsChange.decode(fromJSON: json)
    }

    /// Propose, as a member of the seated team of `contractId`, the deletion
    /// of settled document `documentId` of `documentTypeName`, signed by
    /// `moderatorIdentityId`, an identity this wallet holds. Returns the team
    /// action the other members approve, `closed` once the document is gone.
    ///
    /// - Parameter reason: Must name a reason document the team's proposal
    ///   lists.
    /// - Throws: as ``moderateContractUser``.
    public func proposeSettledContractDocumentDeletion(
        moderatorIdentityId: Identifier,
        contractId: Identifier,
        documentTypeName: String,
        documentId: Identifier,
        reason: ContractModerationReasonInput,
        signer: KeychainSigner
    ) async throws -> ContractTeamActionSignature {
        let moderator = try Self.moderationIdentifier(moderatorIdentityId, "moderatorIdentityId")
        let contract = try Self.moderationIdentifier(contractId, "contractId")
        let document = try Self.moderationIdentifier(documentId, "documentId")
        let reasonDocument = try reason.reasonDocumentId.map { try Self.moderationIdentifier($0, "reasonDocumentId") }
        let reasonText = reason.text
        let json = try await moderationCall("contract_document_propose_settled_deletion", signer: signer) { handle, signerHandle, out in
            withOptionalBytes(reasonDocument) { reasonDocumentPtr in
                reasonText.withCString { reasonTextPtr in
                    documentTypeName.withCString { typePtr in
                        moderator.withUnsafeBufferPointer { moderatorBp in
                            contract.withUnsafeBufferPointer { contractBp in
                                document.withUnsafeBufferPointer { documentBp in
                                    platform_wallet_contract_document_propose_settled_deletion(
                                        handle,
                                        moderatorBp.baseAddress!,
                                        contractBp.baseAddress!,
                                        typePtr,
                                        documentBp.baseAddress!,
                                        reasonTextPtr,
                                        reasonDocumentPtr,
                                        signerHandle,
                                        out
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
        return try ContractTeamActionSignature.decode(fromJSON: json)
    }

    /// Approve, as a member of the seated team of `contractId`, team action
    /// `actionId` another member proposed, signed by `moderatorIdentityId`,
    /// an identity this wallet holds. Returns the action's status: `active`
    /// while approvals still fall short, `closed` once it ran.
    ///
    /// - Throws: as ``moderateContractUser``.
    public func approveContractTeamAction(
        moderatorIdentityId: Identifier,
        contractId: Identifier,
        actionId: Identifier,
        signer: KeychainSigner
    ) async throws -> ContractTeamActionSignature {
        let moderator = try Self.moderationIdentifier(moderatorIdentityId, "moderatorIdentityId")
        let contract = try Self.moderationIdentifier(contractId, "contractId")
        let action = try Self.moderationIdentifier(actionId, "actionId")
        let json = try await moderationCall("contract_team_action_approve", signer: signer) { handle, signerHandle, out in
            moderator.withUnsafeBufferPointer { moderatorBp in
                contract.withUnsafeBufferPointer { contractBp in
                    action.withUnsafeBufferPointer { actionBp in
                        platform_wallet_contract_team_action_approve(
                            handle,
                            moderatorBp.baseAddress!,
                            contractBp.baseAddress!,
                            actionBp.baseAddress!,
                            signerHandle,
                            out
                        )
                    }
                }
            }
        }
        return try ContractTeamActionSignature.decode(fromJSON: json)
    }

    // MARK: - Marshalling

    /// The bytes of a 32-byte identifier, which the FFI reads exactly.
    private static func moderationIdentifier(_ id: Identifier, _ name: String) throws -> [UInt8] {
        guard id.count == 32 else {
            throw PlatformWalletError.invalidParameter("\(name) must be exactly 32 bytes")
        }
        return [UInt8](id)
    }

    /// Run one moderation FFI call off the caller's actor and return the JSON
    /// it wrote.
    ///
    /// Lifetime contract: the wallet and `signer` are pinned with
    /// `withExtendedLifetime` across the synchronous FFI call: a wallet deinit
    /// mid-call would destroy the handle the call uses, and Rust holds a
    /// `passUnretained` ctx pointer to the signer. `call` gets the signer's
    /// handle, `nil` without a signer. The JSON Rust hands over is freed with
    /// `platform_wallet_string_free`.
    private func moderationCall(
        _ name: String,
        signer: KeychainSigner?,
        _ call: @escaping @Sendable (
            Handle,
            OpaquePointer?,
            UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>
        ) -> PlatformWalletFFIResult
    ) async throws -> String {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { [self] in
            var jsonPtr: UnsafeMutablePointer<CChar>? = nil
            defer { if let p = jsonPtr { platform_wallet_string_free(p) } }

            // The signer handle is read here, inside the task, and handed to
            // `call`: an `OpaquePointer` is not `Sendable`, so it is not
            // captured across the task boundary.
            let result = withExtendedLifetime((self, signer)) {
                call(handle, signer?.handle, &jsonPtr)
            }

            try result.check()
            // On success the Rust side always writes the JSON; a null pointer
            // is an FFI contract violation.
            guard let jsonPtr else {
                throw PlatformWalletError.walletOperation("\(name) returned no JSON")
            }
            return String(cString: jsonPtr)
        }.value
    }
}

/// `body` with a pointer to `value`, or null when there is none.
private func withOptionalPointer<T, R>(to value: T?, _ body: (UnsafePointer<T>?) -> R) -> R {
    guard var value else { return body(nil) }
    return withUnsafePointer(to: &value) { body($0) }
}

/// `body` with a pointer to `bytes`, or null when there are none.
private func withOptionalBytes<R>(_ bytes: [UInt8]?, _ body: (UnsafePointer<UInt8>?) -> R) -> R {
    guard let bytes else { return body(nil) }
    return bytes.withUnsafeBufferPointer { body($0.baseAddress) }
}
