import Foundation
import DashSDKFFI

// MARK: - Contract fee claims (protocol version 14)
//
// Claiming a contract's fee pot and asking who may claim it are platform
// wallet operations (`platform_wallet_contract_fee_claim` /
// `platform_wallet_contract_fee_claim_eligibility`, over
// `IdentityWallet::claim_contract_fees_with_signer` /
// `contract_fee_claim_eligibility`). rs-sdk, the wallet and Platform decide
// everything: the signing key, who may claim which pot, whether the pot was
// already paid out this epoch and how it is split. These wrappers marshal the
// arguments in and the JSON out; the signing key never crosses into Swift
// logic, the `KeychainSigner` trampoline services the signature on demand.

extension ManagedPlatformWallet {
    /// Which of this wallet's identities may claim which fee pot of
    /// `contractId`.
    ///
    /// Rust reads the contract and, when it declares elected moderation, the
    /// team of its seated charter: the owner pot is the contract owner's, the
    /// moderators pot the seated team's (its leader and active members) or,
    /// without one, the team the contract's declaration names. A host offers
    /// a claim only to the identities this names; Platform still checks the
    /// claim when it is sent.
    ///
    /// It names recipients, not keys: a named identity whose signer holds no
    /// CRITICAL authentication key without contract bounds is still named,
    /// and `claimContractFees` throws for it at claim time; show that error
    /// where the claim was asked for.
    ///
    /// Lifetime contract: the wallet is pinned with `withExtendedLifetime`
    /// across the synchronous FFI call, as its deinit destroys the handle the
    /// call uses.
    ///
    /// - Parameter contractId: The contract's 32-byte id.
    /// - Throws: `PlatformWalletError.invalidParameter` for an id that is not
    ///   32 bytes; `PlatformWalletError.notFound` when Platform holds no such
    ///   contract; a failed read as the error its result code maps to.
    public func contractFeeClaimEligibility(
        contractId: Identifier
    ) async throws -> ContractFeeClaimEligibility {
        // The FFI reads exactly 32 bytes of the identifier.
        guard contractId.count == 32 else {
            throw PlatformWalletError.invalidParameter("contractId must be exactly 32 bytes")
        }
        let handle = self.handle
        let contractBytes = [UInt8](contractId)
        return try await Task.detached(priority: .userInitiated) { [self] in
            // Receives an owned JSON C string on success; freed with
            // `platform_wallet_string_free` below.
            var jsonPtr: UnsafeMutablePointer<CChar>? = nil
            defer { if let p = jsonPtr { platform_wallet_string_free(p) } }

            let result = withExtendedLifetime(self) {
                contractBytes.withUnsafeBufferPointer { contractBp in
                    platform_wallet_contract_fee_claim_eligibility(
                        handle,
                        contractBp.baseAddress!,
                        &jsonPtr
                    )
                }
            }

            try result.check()
            // On success the Rust side always writes the JSON; a null pointer
            // is an FFI contract violation.
            guard let jsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "contract_fee_claim_eligibility returned no JSON"
                )
            }
            return try ContractFeeClaimEligibility.decode(fromJSON: String(cString: jsonPtr))
        }.value
    }

    /// Pay out `pot` of `contractId`, signed by `claimantIdentityId`, an
    /// identity this wallet holds, and return the claim as its proof shows
    /// it: the pot as the claim left it and the balances it paid.
    ///
    /// rs-sdk signs with the claimant's first CRITICAL authentication key
    /// without contract bounds that `signer` holds, fetches the contract
    /// again for the proof check, broadcasts and waits for the proved pot and
    /// balances; the wallet keeps the balance of each of its identities the
    /// proof shows, so they reach the persister like any proved balance.
    ///
    /// The proof of a moderators pot claim by a member of an elected
    /// contract's seated team shows the claimant's balance alone, so the
    /// other team members this wallet holds keep the balances they had
    /// before the claim until their balances are read again.
    ///
    /// Lifetime contract: the wallet and the `signer` are pinned with
    /// `withExtendedLifetime` across the synchronous FFI call (Rust holds a
    /// `passUnretained` ctx pointer to the signer).
    ///
    /// - Parameters:
    ///   - claimantIdentityId: The claimant's 32-byte identity id.
    ///   - contractId: The contract's 32-byte id.
    ///   - pot: The pot to pay out.
    ///   - signer: Signs the claim.
    /// - Throws: `PlatformWalletError.invalidParameter` for an id that is not
    ///   32 bytes; `PlatformWalletError.identityNotFound` when this wallet
    ///   holds no such signing identity; an SDK error when `signer` holds no
    ///   CRITICAL authentication key of the claimant; a refusal by Platform as
    ///   `PlatformWalletError.consensusRejection`, with its consensus code: a
    ///   claimant that is not a recipient of the pot, a pot already paid out
    ///   this epoch (41111) or one too small to pay everyone a credit.
    public func claimContractFees(
        claimantIdentityId: Identifier,
        contractId: Identifier,
        pot: ContractFeePot,
        signer: KeychainSigner
    ) async throws -> ClaimedContractFees {
        // The FFI reads exactly 32 bytes of each identifier.
        guard claimantIdentityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("claimantIdentityId must be exactly 32 bytes")
        }
        guard contractId.count == 32 else {
            throw PlatformWalletError.invalidParameter("contractId must be exactly 32 bytes")
        }
        let handle = self.handle
        let signerHandle = signer.handle
        let claimantBytes = [UInt8](claimantIdentityId)
        let contractBytes = [UInt8](contractId)
        let potTag = pot.ffiTag
        return try await Task.detached(priority: .userInitiated) { [self] in
            var jsonPtr: UnsafeMutablePointer<CChar>? = nil
            defer { if let p = jsonPtr { platform_wallet_string_free(p) } }

            // Pin the wallet and `signer` for the whole FFI call (see
            // `createDocument` for why a bare `_ = signer` is unreliable
            // under -O): a wallet deinit mid-call would destroy the handle
            // the FFI is still using.
            let result = withExtendedLifetime((self, signer)) {
                claimantBytes.withUnsafeBufferPointer { claimantBp in
                    contractBytes.withUnsafeBufferPointer { contractBp in
                        platform_wallet_contract_fee_claim(
                            handle,
                            claimantBp.baseAddress!,
                            contractBp.baseAddress!,
                            potTag,
                            signerHandle,
                            &jsonPtr
                        )
                    }
                }
            }

            try result.check()
            guard let jsonPtr else {
                throw PlatformWalletError.walletOperation(
                    "contract_fee_claim returned no JSON"
                )
            }
            return try ClaimedContractFees.decode(fromJSON: String(cString: jsonPtr))
        }.value
    }
}
