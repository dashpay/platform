import Foundation

// MARK: - Token distribution type

/// Mirrors
/// `dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType`.
/// The raw values are the FFI discriminants accepted by
/// `platform_wallet_token_claim`.
public enum TokenDistributionType: UInt8, Equatable, CaseIterable, Sendable {
    case preProgrammed = 0
    case perpetual = 1
}

// MARK: - Group action mode

/// How a token state-transition that supports group authorization
/// should be submitted. Most actions on a non-group-gated token use
/// `.none`; group-gated actions (e.g. burn under `MainGroup`)
/// must propose a new group action or sign on an existing one.
public enum GroupActionMode: Equatable, Sendable {
    /// Single-signer execution. The default.
    case none
    /// Propose a brand-new group action at `position`. The Rust side
    /// generates a fresh `action_id` and broadcasts the proposal.
    case propose(position: UInt16)
    /// Sign on an existing proposal (`actionId`). `position` is the
    /// caller's seat in the group; `actionIsProposer` reflects whether
    /// the caller originally proposed the action.
    case signExisting(position: UInt16, actionId: Data, actionIsProposer: Bool)
}

// MARK: - Token configuration change

/// One variant of `dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem`
/// surfaceable through the FFI.
///
/// The Rust enum has 32 variants; this Swift mirror only carries the
/// ones the FFI currently knows how to decode. New variants are added
/// here alongside a matching `change_item_tag` on the Rust side; the
/// `(tag, payloadJSON)` shape lets the FFI grow without changing its
/// parameter list.
public enum TokenConfigChange: Sendable {
    /// MaxSupply(Option<TokenAmount>). `nil` removes the cap entirely.
    case maxSupply(newValue: UInt64?)

    /// FFI tag for `change_item_tag`. Must agree with the constants in
    /// `packages/rs-platform-wallet-ffi/src/tokens/update_config.rs`.
    public var ffiTag: UInt8 {
        switch self {
        case .maxSupply: return 0
        }
    }

    /// JSON object the FFI parses for this variant. The shape is
    /// per-tag; for MaxSupply the field is `newMaxSupply` (string-encoded
    /// u64 or null).
    public func payloadJSON() -> String {
        switch self {
        case .maxSupply(let newValue):
            if let v = newValue {
                return "{\"newMaxSupply\":\"\(v)\"}"
            }
            return "{\"newMaxSupply\":null}"
        }
    }
}

// MARK: - ManagedPlatformWallet token actions

extension ManagedPlatformWallet {
    /// Transfer `amount` of the token at `tokenPosition` on
    /// `contractId` from `identityId` to `recipient`.
    ///
    /// The data contract is fetched server-side and the signing key
    /// is selected on the Rust side (first AUTHENTICATION /
    /// MASTER-or-HIGH / ECDSA_SECP256K1). The `signingKeyId` argument
    /// is reserved for a future "explicit key id" mode and is ignored
    /// by the current FFI.
    ///
    /// Lifetime contract: same as `createDataContract` — the
    /// `signer` instance MUST stay alive for the entire `await`.
    public func tokenTransfer(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        recipient: Identifier,
        amount: UInt64,
        publicNote: String?,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let recipientBytes = recipient.toFFIByteArray()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    recipientBytes.withUnsafeBufferPointer { recipientBp in
                        ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                            platform_wallet_token_transfer(
                                handle,
                                idBp.baseAddress!,
                                contractBp.baseAddress!,
                                tokenPosition,
                                recipientBp.baseAddress!,
                                amount,
                                notePtr,
                                signingKeyId,
                                signerHandle,
                                &error
                            )
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Burn `amount` of the token at `tokenPosition` on `contractId`
    /// from `identityId`. `groupAction` selects single-signer vs
    /// group-action semantics.
    ///
    /// The data contract is fetched server-side and the signing key
    /// is selected on the Rust side (first AUTHENTICATION /
    /// MASTER-or-HIGH / ECDSA_SECP256K1). The `signingKeyId` argument
    /// is reserved for a future "explicit key id" mode and is ignored
    /// by the current FFI.
    public func tokenBurn(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        amount: UInt64,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                        ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                            platform_wallet_token_burn(
                                handle,
                                idBp.baseAddress!,
                                contractBp.baseAddress!,
                                tokenPosition,
                                amount,
                                notePtr,
                                groupTuple.kind,
                                groupTuple.position,
                                actionIdPtr,
                                groupTuple.actionIsProposer,
                                signingKeyId,
                                signerHandle,
                                &error
                            )
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Mint `amount` of the token at `tokenPosition` on `contractId`
    /// to `recipient` (or `identityId` itself when `recipient` is nil).
    /// `groupAction` selects single-signer vs group-action semantics.
    ///
    /// Same signing-key resolution and lifetime contract as
    /// `tokenTransfer` / `tokenBurn`.
    public func tokenMint(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        recipient: Identifier?,
        amount: UInt64,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let recipientBytes: [UInt8]? = recipient.map { $0.toFFIByteArray() }
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    ManagedPlatformWallet.tokenWithOptionalRecipient(recipientBytes) { recipientPtr in
                        ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                            ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                                platform_wallet_token_mint(
                                    handle,
                                    idBp.baseAddress!,
                                    contractBp.baseAddress!,
                                    tokenPosition,
                                    recipientPtr,
                                    amount,
                                    notePtr,
                                    groupTuple.kind,
                                    groupTuple.position,
                                    actionIdPtr,
                                    groupTuple.actionIsProposer,
                                    signingKeyId,
                                    signerHandle,
                                    &error
                                )
                            }
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Claim a distribution payout of `distributionType` from the
    /// token at `tokenPosition` on `contractId` for `identityId`.
    ///
    /// Claim is not group-gated. Same signing-key resolution and
    /// lifetime contract as `tokenTransfer`.
    public func tokenClaim(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        distributionType: TokenDistributionType,
        publicNote: String?,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let distRaw = distributionType.rawValue

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                        platform_wallet_token_claim(
                            handle,
                            idBp.baseAddress!,
                            contractBp.baseAddress!,
                            tokenPosition,
                            distRaw,
                            notePtr,
                            signingKeyId,
                            signerHandle,
                            &error
                        )
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Freeze the entire balance of `frozenIdentityId` for the token at
    /// `tokenPosition` on `contractId`. `groupAction` selects
    /// single-signer vs group-action semantics.
    ///
    /// Same signing-key resolution and lifetime contract as
    /// `tokenTransfer` / `tokenBurn`.
    public func tokenFreeze(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        frozenIdentityId: Identifier,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let frozenBytes = frozenIdentityId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    frozenBytes.withUnsafeBufferPointer { frozenBp in
                        ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                            ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                                platform_wallet_token_freeze(
                                    handle,
                                    idBp.baseAddress!,
                                    contractBp.baseAddress!,
                                    tokenPosition,
                                    frozenBp.baseAddress!,
                                    notePtr,
                                    groupTuple.kind,
                                    groupTuple.position,
                                    actionIdPtr,
                                    groupTuple.actionIsProposer,
                                    signingKeyId,
                                    signerHandle,
                                    &error
                                )
                            }
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Unfreeze the entire frozen balance of `frozenIdentityId` for the
    /// token at `tokenPosition` on `contractId`. `groupAction` selects
    /// single-signer vs group-action semantics.
    public func tokenUnfreeze(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        frozenIdentityId: Identifier,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let frozenBytes = frozenIdentityId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    frozenBytes.withUnsafeBufferPointer { frozenBp in
                        ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                            ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                                platform_wallet_token_unfreeze(
                                    handle,
                                    idBp.baseAddress!,
                                    contractBp.baseAddress!,
                                    tokenPosition,
                                    frozenBp.baseAddress!,
                                    notePtr,
                                    groupTuple.kind,
                                    groupTuple.position,
                                    actionIdPtr,
                                    groupTuple.actionIsProposer,
                                    signingKeyId,
                                    signerHandle,
                                    &error
                                )
                            }
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Destroy the entire frozen balance of `frozenIdentityId` for the
    /// token at `tokenPosition` on `contractId`. `groupAction` selects
    /// single-signer vs group-action semantics. The Rust builder takes
    /// no `amount` — destroying always affects the full frozen balance.
    public func tokenDestroyFrozenFunds(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        frozenIdentityId: Identifier,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let frozenBytes = frozenIdentityId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    frozenBytes.withUnsafeBufferPointer { frozenBp in
                        ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                            ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                                platform_wallet_token_destroy_frozen_funds(
                                    handle,
                                    idBp.baseAddress!,
                                    contractBp.baseAddress!,
                                    tokenPosition,
                                    frozenBp.baseAddress!,
                                    notePtr,
                                    groupTuple.kind,
                                    groupTuple.position,
                                    actionIdPtr,
                                    groupTuple.actionIsProposer,
                                    signingKeyId,
                                    signerHandle,
                                    &error
                                )
                            }
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Pause all operations for the token at `tokenPosition` on
    /// `contractId`. Pause is an emergency action and is group-capable;
    /// it targets the token itself — there is no per-identity target.
    ///
    /// Same signing-key resolution and lifetime contract as
    /// `tokenTransfer` / `tokenBurn`.
    public func tokenPause(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                        ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                            platform_wallet_token_pause(
                                handle,
                                idBp.baseAddress!,
                                contractBp.baseAddress!,
                                tokenPosition,
                                notePtr,
                                groupTuple.kind,
                                groupTuple.position,
                                actionIdPtr,
                                groupTuple.actionIsProposer,
                                signingKeyId,
                                signerHandle,
                                &error
                            )
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Resume all operations for the token at `tokenPosition` on
    /// `contractId`. Resume is an emergency action and is group-capable;
    /// it targets the token itself — there is no per-identity target.
    ///
    /// Same signing-key resolution and lifetime contract as
    /// `tokenTransfer` / `tokenBurn`.
    public func tokenResume(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                        ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                            platform_wallet_token_resume(
                                handle,
                                idBp.baseAddress!,
                                contractBp.baseAddress!,
                                tokenPosition,
                                notePtr,
                                groupTuple.kind,
                                groupTuple.position,
                                actionIdPtr,
                                groupTuple.actionIsProposer,
                                signingKeyId,
                                signerHandle,
                                &error
                            )
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Configure the direct-purchase price for the token at
    /// `tokenPosition` on `contractId`. `pricePerToken == 0` clears
    /// the schedule on the Rust side, disabling direct purchase.
    /// `groupAction` selects single-signer vs group-action semantics.
    ///
    /// Only the flat-price form is exposed at this layer; tiered
    /// (`SetPrices`) schedules are intentionally unsupported until a
    /// future wave surfaces them.
    ///
    /// Same signing-key resolution and lifetime contract as
    /// `tokenTransfer` / `tokenBurn`.
    public func tokenSetPrice(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        pricePerToken: UInt64,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                        ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                            platform_wallet_token_set_price(
                                handle,
                                idBp.baseAddress!,
                                contractBp.baseAddress!,
                                tokenPosition,
                                pricePerToken,
                                notePtr,
                                groupTuple.kind,
                                groupTuple.position,
                                actionIdPtr,
                                groupTuple.actionIsProposer,
                                signingKeyId,
                                signerHandle,
                                &error
                            )
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Purchase `amount` of the token at `tokenPosition` on
    /// `contractId`, debiting `expectedTotalCost` credits from
    /// `identityId`. Direct Purchase is not group-gated, so there is
    /// no `groupAction` parameter.
    ///
    /// `expectedTotalCost` is the credits the caller agrees to pay in
    /// total — Platform rejects the transition if the on-chain
    /// pricing schedule disagrees, protecting the buyer from
    /// price-change races between fetch and submit.
    ///
    /// Same signing-key resolution and lifetime contract as
    /// `tokenTransfer` / `tokenBurn`. The underlying builder takes no
    /// `public_note`, so none is surfaced here.
    public func tokenPurchase(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        amount: UInt64,
        expectedTotalCost: UInt64,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    platform_wallet_token_purchase(
                        handle,
                        idBp.baseAddress!,
                        contractBp.baseAddress!,
                        tokenPosition,
                        amount,
                        expectedTotalCost,
                        signingKeyId,
                        signerHandle,
                        &error
                    )
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    /// Update the configuration of the token at `tokenPosition` on
    /// `contractId`. Wave 7 only surfaces the `MaxSupply` variant via
    /// `TokenConfigChange.maxSupply(newValue:)`; other variants will
    /// be added as the FFI's tag table grows. `groupAction` selects
    /// single-signer vs group-action semantics — config updates on
    /// group-gated tokens must be proposed under the rule that
    /// governs the specific change being made.
    ///
    /// Same signing-key resolution and lifetime contract as
    /// `tokenTransfer` / `tokenBurn`.
    public func tokenUpdateConfig(
        identityId: Identifier,
        contractId: Identifier,
        tokenPosition: UInt16,
        change: TokenConfigChange,
        publicNote: String?,
        groupAction: GroupActionMode = .none,
        signingKeyId: UInt32 = 0,
        signer: KeychainSigner
    ) async throws {
        let handle = self.handle
        let signerHandle = signer.handle
        let identityBytes = identityId.toFFIByteArray()
        let contractBytes = contractId.toFFIByteArray()
        let groupTuple = groupAction.toFFITuple()
        let tag = change.ffiTag
        let payload = change.payloadJSON()

        try await Task.detached(priority: .userInitiated) {
            _ = signer
            var error = PlatformWalletFFIError()
            let result = identityBytes.withUnsafeBufferPointer { idBp -> PlatformWalletFFIResult in
                contractBytes.withUnsafeBufferPointer { contractBp in
                    payload.withCString { payloadPtr in
                        ManagedPlatformWallet.tokenWithOptionalCString(publicNote) { notePtr in
                            ManagedPlatformWallet.tokenWithOptionalActionId(groupTuple.actionId) { actionIdPtr in
                                platform_wallet_token_update_config(
                                    handle,
                                    idBp.baseAddress!,
                                    contractBp.baseAddress!,
                                    tokenPosition,
                                    tag,
                                    payloadPtr,
                                    notePtr,
                                    groupTuple.kind,
                                    groupTuple.position,
                                    actionIdPtr,
                                    groupTuple.actionIsProposer,
                                    signingKeyId,
                                    signerHandle,
                                    &error
                                )
                            }
                        }
                    }
                }
            }
            guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
                throw PlatformWalletError(result: result, error: error)
            }
        }.value
    }

    // MARK: Helpers

    /// Run `body` with a NUL-terminated C string for `value`, or NULL
    /// when `value` is nil. Mirrors `createDataContract`'s
    /// `withOptionalCString`.
    fileprivate static func tokenWithOptionalCString<R>(
        _ value: String?,
        _ body: (UnsafePointer<CChar>?) -> R
    ) -> R {
        if let value = value {
            return value.withCString { body($0) }
        }
        return body(nil)
    }

    /// Run `body` with a `*const u8` to the 32-byte action id, or
    /// NULL when no action id is present (single-signer / proposer
    /// modes).
    fileprivate static func tokenWithOptionalActionId<R>(
        _ value: Data?,
        _ body: (UnsafePointer<UInt8>?) -> R
    ) -> R {
        guard let value = value else {
            return body(nil)
        }
        return value.withUnsafeBytes { raw in
            body(raw.bindMemory(to: UInt8.self).baseAddress)
        }
    }

    /// Run `body` with a `*const u8` to the 32-byte recipient
    /// identity bytes, or NULL when no recipient is provided
    /// (mint-to-self).
    fileprivate static func tokenWithOptionalRecipient<R>(
        _ value: [UInt8]?,
        _ body: (UnsafePointer<UInt8>?) -> R
    ) -> R {
        guard let value = value else {
            return body(nil)
        }
        return value.withUnsafeBufferPointer { bp in
            body(bp.baseAddress)
        }
    }
}

// MARK: - Identifier helper

private extension Data {
    /// Materialise the 32-byte payload as a `[UInt8]` for use with
    /// `withUnsafeBufferPointer`. Identifiers on Platform are always
    /// exactly 32 bytes; the precondition surfaces the drift loudly
    /// rather than letting Rust read garbage on dereference.
    func toFFIByteArray() -> [UInt8] {
        precondition(count == 32, "identifier must be 32 bytes, got \(count)")
        return Array(self)
    }
}

// MARK: - GroupActionMode lowering

extension GroupActionMode {
    /// Decompose into the flat `(kind, position, action_id, action_is_proposer)`
    /// tuple expected by the FFI. `kind` matches
    /// `decode_group_info` in
    /// `packages/rs-platform-wallet-ffi/src/tokens/group_info.rs`.
    func toFFITuple() -> (kind: UInt8, position: UInt16, actionId: Data?, actionIsProposer: Bool) {
        switch self {
        case .none:
            return (0, 0, nil, false)
        case .propose(let position):
            return (1, position, nil, false)
        case .signExisting(let position, let actionId, let actionIsProposer):
            precondition(actionId.count == 32, "actionId must be 32 bytes, got \(actionId.count)")
            return (2, position, actionId, actionIsProposer)
        }
    }
}
