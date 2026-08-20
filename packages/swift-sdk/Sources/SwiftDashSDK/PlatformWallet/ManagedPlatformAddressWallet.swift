import Foundation
import DashSDKFFI

/// Platform address wallet for balance queries, transfers, and withdrawals.
///
/// Obtained via `ManagedPlatformWallet.platformAddressWallet()`.
/// Background BLAST syncing is owned by `PlatformWalletManager`.
///
/// `@unchecked Sendable`: the underlying Rust handle is guarded by
/// `platform-wallet-ffi`'s internal `HandleStorage` (parking_lot RwLock),
/// so dispatching method calls across threads is safe. The Swift side
/// only borrows the opaque pointer.
public final class ManagedPlatformAddressWallet: @unchecked Sendable {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        platform_address_wallet_destroy(handle).discard()
    }

    // MARK: - Balance queries

    /// Platform address with its credit balance.
    public struct AddressBalance: Sendable {
        /// Address type (0 = P2PKH).
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Credit balance.
        public let balance: UInt64
    }

    /// Get total platform credits across all addresses.
    public func totalCredits() throws -> UInt64 {
        var credits: UInt64 = 0
        try platform_address_wallet_total_credits(handle, &credits).check()
        return credits
    }

    /// The per-input minimum credit amount (`min_input_amount`) the chain
    /// enforces for address-funds transitions, read from the wallet's
    /// current platform version on the Rust side.
    ///
    /// This is the same floor the Rust transfer/withdraw auto-selectors use
    /// to drop sub-minimum "dust" inputs (DPP rejects any address-funds
    /// input below it, so an address whose balance is under this can't be
    /// spent on its own). UI that gates a transfer/withdraw should sum only
    /// balances `>= this` so the enabled/disabled decision matches what
    /// Rust will actually consume — rather than mirroring the `100_000`
    /// protocol constant in Swift, which would drift if the version
    /// changed it.
    public func minInputAmount() throws -> UInt64 {
        var amount: UInt64 = 0
        try platform_address_wallet_min_input_amount(handle, &amount).check()
        return amount
    }

    /// The per-output minimum credit amount (`min_output_amount`) the chain
    /// enforces for address-funds transitions, read from the wallet's
    /// current platform version on the Rust side.
    ///
    /// DPP rejects any address-funds output below this floor, so a transfer
    /// that sends a single output under it fails structure validation after
    /// submit. UI that gates a transfer should require the requested amount
    /// to reach this (and explain why when it doesn't) so the
    /// enabled/disabled decision matches what DPP will accept — rather than
    /// mirroring the protocol constant in Swift, which would drift if the
    /// version changed it. Companion to `minInputAmount()`.
    public func minOutputAmount() throws -> UInt64 {
        var amount: UInt64 = 0
        try platform_address_wallet_min_output_amount(handle, &amount).check()
        return amount
    }

    /// Get all platform addresses with their cached balances.
    public func addressesWithBalances() throws -> [AddressBalance] {
        var entriesPtr: UnsafeMutablePointer<AddressBalanceEntryFFI>?
        var count: UInt = 0
        try platform_address_wallet_addresses_with_balances(handle, &entriesPtr, &count).check()

        defer {
            platform_address_wallet_free_address_balances(entriesPtr, count)
        }

        guard let entries = entriesPtr, count > 0 else {
            return []
        }

        return (0..<Int(count)).map { i in
            let entry = entries[i]
            let hashData = withUnsafeBytes(of: entry.address.hash) { Data($0) }
            return AddressBalance(
                addressType: entry.address.address_type,
                hash: hashData,
                balance: entry.balance
            )
        }
    }

    // MARK: - Transfer

    /// One recipient row for `transfer(...)`.
    public struct TransferOutput: Sendable {
        /// Must be `0` (P2PKH). The platform-address transfer surface
        /// supports P2PKH only: the Rust `TryFrom<PlatformAddressFFI>`
        /// backing `parse_outputs`/`parse_explicit_inputs*` rejects
        /// `address_type = 1` (P2SH) because inputs are signed by a
        /// P2PKH-only `Signer<PlatformAddress>` and recipients on this
        /// surface are always P2PKH. The field stays `UInt8` to mirror the
        /// Rust discriminant layout, but `1` will be rejected by the FFI.
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Credits to send to this address.
        public let credits: UInt64

        public init(addressType: UInt8, hash: Data, credits: UInt64) {
            self.addressType = addressType
            self.hash = hash
            self.credits = credits
        }
    }

    /// Updated balance for an address after a transfer.
    public struct UpdatedBalance: Sendable {
        public let addressType: UInt8
        public let hash: Data
        public let balance: UInt64
        public let nonce: UInt32
        public let accountIndex: UInt32
        public let addressIndex: UInt32
    }

    /// Transfer credits between platform addresses.
    ///
    /// Input selection, the `Σ inputs == Σ outputs` balancing, fee strategy,
    /// and nonce resolution all happen inside the Rust `platform-wallet` crate.
    /// This wrapper marshals the recipient outputs and drives
    /// `platform_address_wallet_transfer` with `INPUT_SELECTION_TYPE_AUTO`:
    ///
    /// - The Rust auto-selector resolves the requested `accountIndex` (key
    ///   class 0), picks the smallest balance-descending covering prefix of its
    ///   funded addresses (skipping sub-`min_input_amount` dust and any address
    ///   that is also a recipient — DPP forbids the same address as both input
    ///   and output), and **partially consumes** the last selected input so the
    ///   surplus stays on the source address. There is no separate change
    ///   output and no change address: the credit-balance model leaves residual
    ///   credits in place on the source addresses.
    /// - The fee strategy is left empty (`nil, 0`); the FFI maps that to
    ///   `[DeductFromInput(0)]`, which the Auto path supports — each recipient
    ///   gets exactly its requested amount and the on-chain fee is deducted from
    ///   the lex-smallest selected input's remaining balance (the selector
    ///   reserves the fee headroom on that input).
    ///
    /// The wrapper still rejects empty/duplicate recipients and a recipient sum
    /// that overflows `UInt64` before the Task detach, so callers get a
    /// synchronous error; the Rust side enforces the same invariants plus typed
    /// diagnostics for insufficient / dust-only / output-collision balances.
    ///
    /// The signer must be able to sign for the auto-selected inputs — i.e.
    /// the `KeychainSigner` resolves their derivation paths via SwiftData + the
    /// wallet mnemonic (the `0xFF` branch in `KeychainSigner.swift`).
    @discardableResult
    public func transfer(
        accountIndex: UInt32,
        outputs: [TransferOutput],
        signer: KeychainSigner
    ) async throws -> [UpdatedBalance] {
        guard !outputs.isEmpty else {
            throw PlatformWalletError.invalidParameter("outputs is empty")
        }

        // Reject duplicate recipient addresses. Rust's parse_outputs inserts
        // into a `BTreeMap<PlatformAddress, Credits>` keyed by address, so a
        // duplicate would silently overwrite earlier entries. We key on `hash`
        // only: P2PKH/P2SH share the same 20-byte hash space, so the same hash
        // with different `addressType` is still ambiguous.
        let outputHashes = Set(outputs.map { $0.hash })
        guard outputHashes.count == outputs.count else {
            throw PlatformWalletError.invalidParameter(
                "Duplicate recipient addresses in outputs"
            )
        }

        // Validate hashes + reject a recipient sum that overflows UInt64
        // before the Task detach (the protocol's sum check would otherwise
        // reject it Rust-side after the detach + signer pin).
        var totalRecipientCredits: UInt64 = 0
        for out in outputs {
            guard out.hash.count == 20 else {
                throw PlatformWalletError.walletOperation(
                    "TransferOutput.hash must be exactly 20 bytes (got \(out.hash.count))"
                )
            }
            let sum = totalRecipientCredits.addingReportingOverflow(out.credits)
            if sum.overflow {
                throw PlatformWalletError.invalidParameter(
                    "Output credits sum overflowed UInt64"
                )
            }
            totalRecipientCredits = sum.partialValue
        }

        // Marshal the recipient outputs. Rust canonicalises to a
        // `BTreeMap<PlatformAddress, Credits>`, so insertion order here is
        // irrelevant — the Auto path resolves its own fee index against the
        // lex-sorted map.
        let ffiOutputs: [AddressBalanceEntryFFI] = outputs.map { out in
            AddressBalanceEntryFFI(
                address: PlatformAddressFFI(
                    address_type: out.addressType,
                    hash: Self.hashTuple(from: out.hash)
                ),
                balance: out.credits,
                nonce: 0,
                account_index: 0,
                address_index: 0,
                // Request path: names an output amount, carries no
                // persisted balance — the height pin is meaningless here.
                as_of_height: 0
            )
        }

        let handle = self.handle
        let signerHandle = signer.handle
        let outRows = ffiOutputs

        return try await Task.detached(priority: .userInitiated) { () -> [UpdatedBalance] in
            var changeset = PlatformAddressChangeSetFFI(updated: nil, updated_count: 0)
            // `withExtendedLifetime(signer)` pins the resolver-backed signer for
            // the entire FFI call. `KeychainSigner` registers its vtable ctx via
            // `Unmanaged.passUnretained(self)`, so a bare `_ = signer` can be
            // elided by the -O optimizer and drop the signer mid-call, causing a
            // use-after-free in the synchronous Rust vtable callback. Mirrors the
            // `withdraw` / `fundFromAssetLock` wrappers.
            //
            // The AUTO path owns input selection and its own fee strategy
            // (`[DeductFromInput(0)]`), so inputs are `nil, 0` (auto-select) and
            // the fee strategy is `nil, 0` (FFI defaults it to
            // `[DeductFromInput(0)]`); mirrors the `withdraw` wrapper.
            let result = withExtendedLifetime(signer) {
                outRows.withUnsafeBufferPointer { outBp -> PlatformWalletFFIResult in
                    platform_address_wallet_transfer(
                        handle,
                        accountIndex,
                        INPUT_SELECTION_TYPE_AUTO,
                        nil,
                        0,
                        nil,
                        0,
                        outBp.baseAddress,
                        UInt(outBp.count),
                        nil,
                        0,
                        signerHandle,
                        &changeset
                    )
                }
            }
            try result.check()

            defer { platform_address_wallet_free_changeset(&changeset) }
            return Self.decodeChangeset(&changeset)
        }.value
    }

    /// Copy a 20-byte `Data` into the fixed-size tuple shape the FFI expects.
    private static func hashTuple(
        from data: Data
    ) -> (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    ) {
        let b = [UInt8](data)
        precondition(b.count == 20, "hashTuple requires a 20-byte Data")
        return (
            b[0], b[1], b[2], b[3], b[4],
            b[5], b[6], b[7], b[8], b[9],
            b[10], b[11], b[12], b[13], b[14],
            b[15], b[16], b[17], b[18], b[19]
        )
    }

    // MARK: - Withdraw

    /// Result of `preflightWithdrawal(accountIndex:coreFeePerByte:)`: whether an
    /// AUTO withdrawal of the account can succeed, and — when it can — the net
    /// credits paid out and the reserved transition fee.
    public struct WithdrawalPreflight: Sendable {
        /// `true` when the account can fund an AUTO withdrawal at the current
        /// platform version; `false` for any "can't fund" case (dust-only,
        /// largest input can't cover the fee, net below the minimum
        /// withdrawal, too many inputs, or net above the maximum withdrawal).
        /// The numeric fields are `0` when `false`.
        public let canWithdraw: Bool
        /// Net credits the chain would pay out (`Σ withdrawable inputs −
        /// estimatedFee`). `0` when `canWithdraw == false`.
        public let netWithdrawable: UInt64
        /// The address-credit-withdrawal transition fee reserved on the
        /// fee-source input. `0` when `canWithdraw == false`.
        public let estimatedFee: UInt64
        /// The Rust side's user-presentable explanation of *why*
        /// `canWithdraw == false`, surfaced verbatim from the FFI result
        /// message (`PlatformWalletError`'s `Display`). Covers both the
        /// permanent "can't fund" cases (dust-only, fee/minimum headroom, too
        /// many inputs, above the maximum) AND the transient "can't confirm
        /// right now" case (the on-chain balance query failed to reach a node
        /// or verify — retryable). `nil` when `canWithdraw == true`. This is the
        /// single source of the reason — the UI must show it rather than
        /// re-deriving a classification in Swift (which would mean mirroring
        /// protocol decisions on the wrong side of the FFI boundary).
        public let reason: String?
    }

    /// Preflight an AUTO withdrawal of a platform-payment account WITHOUT
    /// signing, broadcasting, or consuming a Core receive address.
    ///
    /// This runs the **same** Rust planning phase the real `withdraw(...)` path
    /// executes (`PlatformAddressWallet::preflight_withdrawal`): it drops
    /// sub-`min_input_amount` dust, estimates the transition fee from the
    /// selected input count, reserves it on the largest-balance input, and
    /// verifies the net clears the minimum withdrawal amount. Gating a UI
    /// submit button on `canWithdraw` keeps it in lockstep with what the spend
    /// path will accept — a small-but-non-dust account that can't cover the fee
    /// reports `canWithdraw == false` here instead of failing after sign.
    ///
    /// The fee estimate depends only on the input/output **counts**, not on any
    /// destination script, so this needs no Core address and touches no receive
    /// pool. It does, however, issue **one DAPI proof query**
    /// (`AddressInfo::fetch_many`) to read the account's authoritative on-chain
    /// balances — the SAME balances the spend path re-fetches and hard-checks —
    /// so the gate can never approve what the spend then rejects, even when the
    /// wallet's cached balance is stale or doubled. Because it makes a network
    /// round trip and verifies a GroveDB proof, it **must not run on the main
    /// actor**; this method is `async` and dispatches the FFI call onto a
    /// detached task. Callers driving it from input changes (account/fee
    /// selection) should cancel-and-replace any in-flight call so a slow
    /// response for the previous input can't clobber a newer one.
    ///
    /// A genuine "can't fund" is a normal result (`canWithdraw == false`), NOT
    /// a thrown error. A **transient** network / proof-verification failure is
    /// likewise reported as `canWithdraw == false` (with the SDK error's
    /// message as `reason`) rather than a thrown error — from the UI's
    /// perspective it's "can't confirm right now, retry," not a broken handle.
    /// Only a structural failure — a bad handle or a missing account at
    /// `accountIndex` — throws.
    ///
    /// `coreFeePerByte` is accepted for symmetry with `withdraw(...)`; the
    /// platform-side transition fee the preflight reserves does not depend on
    /// it (it sizes the eventual L1 payout, not the credit-side fee), but
    /// threading it keeps the call sites parallel.
    public func preflightWithdrawal(
        accountIndex: UInt32,
        coreFeePerByte: UInt32 = 1
    ) async throws -> WithdrawalPreflight {
        // Pin the handle so the detached task's capture is a plain scalar
        // (`Handle` is a raw pointer/int); the caller guarantees this
        // `ManagedPlatformAddressWallet` outlives the awaited call.
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) {
            () -> WithdrawalPreflight in
            var out = WithdrawalPreflightFFI(
                can_withdraw: false,
                net_withdrawable: 0,
                estimated_fee: 0
            )
            // The can-withdraw, can't-fund, and can't-confirm-right-now (transient
            // network/proof) outcomes all return a Success-coded result; only a
            // structural failure (bad handle / missing account) is an error code.
            // The non-withdrawable results carry the Rust reason in `message`,
            // which we surface verbatim as `reason`. We construct the result
            // wrapper explicitly (rather than `.check()`) so we can read `.message`
            // BEFORE the wrapper deinits and frees the underlying C string;
            // `throwIfError()` still throws on the structural-failure codes.
            let result = PlatformWalletResult(
                platform_address_wallet_preflight_withdrawal(
                    handle,
                    accountIndex,
                    coreFeePerByte,
                    &out
                )
            )
            try result.throwIfError()
            // Read the message while the wrapper is still alive; only attach it as
            // the reason (a `canWithdraw == true` result has no reason).
            let reason = out.can_withdraw ? nil : result.message
            return WithdrawalPreflight(
                canWithdraw: out.can_withdraw,
                netWithdrawable: out.net_withdrawable,
                estimatedFee: out.estimated_fee,
                reason: reason
            )
        }.value
    }

    /// Withdraw this platform-payment account's withdrawable credit
    /// balance to a Core L1 address (less the transition fee).
    ///
    /// `AddressCreditWithdrawalTransition` has no change output, so the
    /// on-chain fee is deducted from the inputs. We drive the Rust side
    /// with `INPUT_SELECTION_TYPE_AUTO`, which on `accountIndex` selects
    /// every funded address whose balance clears the per-input minimum
    /// (sub-minimum "dust" addresses are skipped so they can't sink the
    /// whole transition) and **owns its own fee strategy**: it picks the
    /// largest-balance selected input as the fee source and emits the
    /// matching `DeductFromInput(<that input's index>)`, ignoring whatever
    /// fee strategy the caller passes. We therefore pass an empty strategy
    /// here — anything we passed would be discarded. The auto-selector
    /// withdraws `balance − estimated_fee` from the fee-source input and
    /// the full balance from every other input; without that reservation a
    /// full-balance withdraw would leave zero remaining on the fee-source
    /// input and the chain would reject the transition with
    /// `fee_fully_covered = false`.
    ///
    /// `coreAddress` is a base58 Core address (e.g. `yXV…` on testnet,
    /// `X…` on mainnet). It is parsed **and network-checked on the
    /// Rust side** against the wallet's own network by
    /// `platform_address_wallet_withdraw_to_address` — a wrong-network
    /// address fails fast with a typed error before any signing.
    ///
    /// `coreFeePerByte` is the Core L1 fee rate (duffs/byte) used to
    /// size the eventual L1 payout transaction; `1` is the usual
    /// default.
    ///
    /// The signer must be able to sign for the selected inputs — i.e.
    /// the `KeychainSigner` resolves their derivation paths via
    /// SwiftData + the wallet mnemonic (the `0xFF` branch in
    /// `KeychainSigner.swift`). Pass `KeychainSigner(modelContainer:)`.
    ///
    /// Returns the per-address `UpdatedBalance`s the Rust changeset
    /// reports (drained inputs read `0`; the fee-source input retains the
    /// reserved-fee remainder until the chain books the fee).
    @discardableResult
    public func withdraw(
        accountIndex: UInt32,
        coreAddress: String,
        coreFeePerByte: UInt32 = 1,
        signer: KeychainSigner
    ) async throws -> [UpdatedBalance] {
        let trimmed = coreAddress.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw PlatformWalletError.invalidParameter("coreAddress is empty")
        }
        // A Swift String can carry an embedded NUL that survives into the
        // `withCString` buffer; Rust's `CStr::from_ptr(...).to_str()` stops
        // at the first NUL, so `valid\0suffix` would withdraw to `valid`
        // while the Swift value differs. Reject it before it can diverge.
        guard !trimmed.utf8.contains(0) else {
            throw PlatformWalletError.invalidParameter(
                "coreAddress contains an embedded NUL byte"
            )
        }

        let handle = self.handle
        let signerHandle = signer.handle
        let address = trimmed
        let feePerByte = coreFeePerByte

        return try await Task.detached(priority: .userInitiated) {
            () -> [UpdatedBalance] in
            // The AUTO path owns its own fee strategy (largest-balance
            // input as the fee source) and ignores the caller's, so we
            // pass an empty strategy (`nil, 0`); see this wrapper's doc
            // comment.
            var changeset = PlatformAddressChangeSetFFI(updated: nil, updated_count: 0)
            // `withExtendedLifetime(signer)` pins the resolver-backed
            // signer for the entire FFI call. Mirrors the
            // `fundFromAssetLock` wrapper: a bare `_ = signer` can be
            // elided by the -O optimizer and drop the signer mid-call,
            // causing a use-after-free in the vtable callback.
            let result = withExtendedLifetime(signer) {
                address.withCString { addrCStr in
                    platform_address_wallet_withdraw_to_address(
                        handle,
                        accountIndex,
                        INPUT_SELECTION_TYPE_AUTO,
                        nil,
                        0,
                        nil,
                        0,
                        addrCStr,
                        feePerByte,
                        nil,
                        0,
                        signerHandle,
                        &changeset
                    )
                }
            }
            try result.check()

            defer { platform_address_wallet_free_changeset(&changeset) }
            return Self.decodeChangeset(&changeset)
        }.value
    }

    /// Explicit withdrawal input for the partial-amount `withdraw` overload:
    /// withdraw exactly `credits` from this address. The transition fee is
    /// deducted by the chain from the address's REMAINING on-chain balance
    /// (`balance − credits`), so the fee-source input (index 0 under the
    /// default `DeductFromInput(0)` strategy) must keep at least the fee
    /// after the withdrawal — an exact-balance input is rejected with
    /// `fee_fully_covered = false`.
    public struct WithdrawalInput: Sendable {
        /// Must be `0` (P2PKH) — same Rust `TryFrom<PlatformAddressFFI>`
        /// constraint as transfer inputs.
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Credits to withdraw from this address (its payout contribution).
        public let credits: UInt64

        public init(addressType: UInt8, hash: Data, credits: UInt64) {
            self.addressType = addressType
            self.hash = hash
            self.credits = credits
        }
    }

    /// Partial-amount withdrawal: pay out exactly `Σ inputs.credits` to
    /// `coreAddress`, drawing the listed amounts from the listed addresses
    /// (`INPUT_SELECTION_TYPE_EXPLICIT`). Unlike the AUTO overload above,
    /// the payout equals the requested amount — the transition fee comes out
    /// of the fee-source input's remaining balance, not the payout. The Rust
    /// side re-fetches and hard-checks every input's on-chain balance before
    /// signing, so a stale caller-side balance fails with a typed error
    /// rather than a wrong-amount withdrawal.
    ///
    /// Same `coreAddress` parsing/network rules and signer requirements as
    /// the AUTO overload.
    @discardableResult
    public func withdraw(
        accountIndex: UInt32,
        coreAddress: String,
        inputs: [WithdrawalInput],
        coreFeePerByte: UInt32 = 1,
        signer: KeychainSigner
    ) async throws -> [UpdatedBalance] {
        let trimmed = coreAddress.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw PlatformWalletError.invalidParameter("coreAddress is empty")
        }
        // Same embedded-NUL guard as the AUTO overload: Rust's
        // `CStr::from_ptr(...).to_str()` stops at the first NUL, so a
        // `valid\0suffix` value would silently withdraw to `valid`.
        guard !trimmed.utf8.contains(0) else {
            throw PlatformWalletError.invalidParameter(
                "coreAddress contains an embedded NUL byte"
            )
        }
        guard !inputs.isEmpty else {
            throw PlatformWalletError.invalidParameter("inputs is empty")
        }
        for input in inputs {
            guard input.addressType == 0 else {
                throw PlatformWalletError.invalidParameter(
                    "only P2PKH (addressType 0) withdrawal inputs are supported"
                )
            }
            guard input.hash.count == 20 else {
                throw PlatformWalletError.invalidParameter(
                    "withdrawal input hash must be 20 bytes, got \(input.hash.count)"
                )
            }
            guard input.credits > 0 else {
                throw PlatformWalletError.invalidParameter(
                    "withdrawal input credits must be > 0"
                )
            }
        }

        let handle = self.handle
        let signerHandle = signer.handle
        let address = trimmed
        let feePerByte = coreFeePerByte
        let inputRows = inputs

        return try await Task.detached(priority: .userInitiated) {
            () -> [UpdatedBalance] in
            let ffiInputs = inputRows.map { input -> ExplicitInputFFI in
                ExplicitInputFFI(
                    address: PlatformAddressFFI(
                        address_type: input.addressType,
                        hash: Self.hashTuple(from: input.hash)
                    ),
                    balance: input.credits
                )
            }
            var changeset = PlatformAddressChangeSetFFI(updated: nil, updated_count: 0)
            // Fee strategy nil/0 → the Rust default `DeductFromInput(0)`:
            // the first input is the fee source. Callers order inputs so
            // index 0 carries the fee headroom.
            let result = withExtendedLifetime(signer) {
                address.withCString { addrCStr in
                    ffiInputs.withUnsafeBufferPointer { inputBp in
                        platform_address_wallet_withdraw_to_address(
                            handle,
                            accountIndex,
                            INPUT_SELECTION_TYPE_EXPLICIT,
                            inputBp.baseAddress,
                            UInt(inputBp.count),
                            nil,
                            0,
                            addrCStr,
                            feePerByte,
                            nil,
                            0,
                            signerHandle,
                            &changeset
                        )
                    }
                }
            }
            try result.check()

            defer { platform_address_wallet_free_changeset(&changeset) }
            return Self.decodeChangeset(&changeset)
        }.value
    }

    // MARK: - Fund from Core asset lock

    /// Recipient entry for `fundFromAssetLock(...)`.
    ///
    /// Exactly one entry per call must have `credits = nil` — that
    /// address receives the remainder after explicit outputs and fees
    /// (the asset lock is consumed in full, so a remainder bucket is
    /// mandatory).
    public struct FundFromAssetLockRecipient: Sendable {
        /// Must be `0` (P2PKH). Funding recipients flow through the same
        /// P2PKH-only Rust `TryFrom<PlatformAddressFFI>` as transfer
        /// inputs/outputs, so `1` (P2SH) is rejected (also enforced
        /// up-front by `fundFromAssetLockPreflight`). The field stays
        /// `UInt8` to mirror the Rust discriminant layout.
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Explicit credit amount, or `nil` to receive the remainder.
        public let credits: UInt64?

        public init(addressType: UInt8, hash: Data, credits: UInt64?) {
            self.addressType = addressType
            self.hash = hash
            self.credits = credits
        }
    }

    /// Expected fee (in credits) the network will actually charge for an
    /// `AddressFundingFromAssetLockTransition` with the given input and
    /// output counts — a display/planning estimate of the metered
    /// execution fee, NOT the (several-times-larger) consensus minimum
    /// the locked value must cover. Pure computation on the Rust side
    /// (no handle, no network); the constants are pinned against real
    /// execution by the drive-abci calibration tests. The canonical
    /// wallet topup is `(inputCount: 0, outputCount: 1)` — the single
    /// remainder recipient.
    public static func estimateAddressFundingFee(
        inputCount: Int = 0,
        outputCount: Int = 1
    ) throws -> UInt64 {
        var fee: UInt64 = 0
        // Counts are `usize` on the Rust side → imported as `UInt`.
        try platform_wallet_address_funding_estimate_fee(
            UInt(inputCount),
            UInt(outputCount),
            &fee
        ).check()
        return fee
    }

    /// Fund this wallet's platform addresses from a Core L1 asset lock,
    /// orchestrated entirely on the Rust side (build asset-lock tx →
    /// wait for IS-lock or fall back to ChainLock → submit
    /// `AddressFundingFromAssetLockTransition` → consume the lock on
    /// success). The asset-lock private key never crosses the FFI
    /// boundary — both Core-side derivation and the outer ST signature
    /// route through a local `MnemonicResolver`.
    ///
    /// - Parameters:
    ///   - amountDuffs: Amount to lock in Core duffs.
    ///   - fundingAccountIndex: standard-family source index — the asset
    ///     lock POOLS the BIP44 and BIP32 accounts at that index together
    ///     with every DashPay receiving account (change returns to BIP44);
    ///     the index does not restrict which DashPay receiving accounts
    ///     contribute. CoinJoin remains drain-only and separate.
    ///   - platformAccountIndex: Platform-payment account containing
    ///     `recipients`. Used for the membership pre-flight on the Rust
    ///     side and the post-success balance write.
    ///   - recipients: Destination addresses. Exactly one must carry
    ///     `credits = nil` (remainder recipient). The Rust side
    ///     enforces both invariants and returns a typed error if
    ///     either is violated.
    ///   - signer: `KeychainSigner` used for any input-witness
    ///     signatures on the address-funding transition. The same
    ///     wallet's `MnemonicResolver` (constructed internally) signs
    ///     the asset-lock proof's outer signature.
    ///
    /// Returns the list of `UpdatedBalance`s for each recipient, with
    /// the new proof-attested credit balance.
    @discardableResult
    public func fundFromAssetLock(
        amountDuffs: UInt64,
        fundingAccountIndex: UInt32,
        platformAccountIndex: UInt32,
        recipients: [FundFromAssetLockRecipient],
        signer: KeychainSigner
    ) async throws -> [UpdatedBalance] {
        try fundFromAssetLockPreflight(recipients: recipients)
        let handle = self.handle
        let signerHandle = signer.handle
        let recipientRows = recipients
        // Constructed on the calling actor so it lives for the entire
        // detached Task. Released after `withExtendedLifetime` returns.
        // See `ManagedPlatformWallet.registerIdentityWithFunding` for the
        // rationale on why `_ = signer` is NOT a substitute here — the
        // -O optimizer can elide the discard and drop the resolver mid-
        // FFI-call, leading to a use-after-free in the vtable callback.
        let coreSigner = MnemonicResolver()

        return try await Task.detached(priority: .userInitiated) {
            () -> [UpdatedBalance] in
            let ffiAddresses = recipientRows.map { r -> FundingAddressEntryFFI in
                FundingAddressEntryFFI(
                    address: PlatformAddressFFI(
                        address_type: r.addressType,
                        hash: Self.hashTuple(from: r.hash)
                    ),
                    has_balance: r.credits != nil,
                    balance: r.credits ?? 0
                )
            }
            // Take the fee out of the remainder output. Today the
            // `None` recipient is structurally the same as the
            // "change" output on a transfer — it absorbs whatever's
            // left after explicit outputs + fees.
            let remainderIndex = UInt16(
                ffiAddresses.firstIndex(where: { !$0.has_balance }) ?? 0
            )
            let feeRows: [FeeStrategyStepFFI] = [
                FeeStrategyStepFFI(step_type: 1, index: remainderIndex)  // 1 = ReduceOutput
            ]
            var changeset = PlatformAddressChangeSetFFI(updated: nil, updated_count: 0)
            let result = withExtendedLifetime((signer, coreSigner)) {
                ffiAddresses.withUnsafeBufferPointer { addrBp in
                    feeRows.withUnsafeBufferPointer { feeBp in
                        platform_address_wallet_fund_from_asset_lock_signer(
                            handle,
                            amountDuffs,
                            fundingAccountIndex,
                            platformAccountIndex,
                            addrBp.baseAddress,
                            UInt(addrBp.count),
                            feeBp.baseAddress,
                            UInt(feeBp.count),
                            signerHandle,
                            coreSigner.handle,
                            &changeset
                        )
                    }
                }
            }
            try result.check()

            defer { platform_address_wallet_free_changeset(&changeset) }
            return Self.decodeChangeset(&changeset)
        }.value
    }

    /// Resume a stuck platform-address funding flow from an already-
    /// tracked asset lock by outpoint.
    ///
    /// Sibling to [`fundFromAssetLock`]: the wallet-balance variant
    /// builds a fresh asset-lock transaction; this variant picks up a
    /// lock that's already tracked (Broadcast / InstantSendLocked /
    /// ChainLocked) and drives whatever stages remain. Use case mirrors
    /// the identity-side `resumeIdentityWithAssetLock` — a prior
    /// attempt left the lock in storage but the address-funding ST
    /// never landed, and the user picks the lock from a "Resumable
    /// Funding" surface.
    ///
    /// - Parameters:
    ///   - outPointTxid: 32-byte raw txid (little-endian wire order,
    ///     same as `OutPointFFI.txid`). The caller decodes
    ///     `PersistentAssetLock.outPointHex` back from display-order
    ///     hex before passing in.
    ///   - outPointVout: Funding output index (always 0 for asset
    ///     locks built by this wallet, but kept for generality).
    @discardableResult
    public func resumeFundFromAssetLock(
        outPointTxid: Data,
        outPointVout: UInt32,
        platformAccountIndex: UInt32,
        recipients: [FundFromAssetLockRecipient],
        signer: KeychainSigner
    ) async throws -> [UpdatedBalance] {
        guard outPointTxid.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "outPointTxid must be exactly 32 bytes (was \(outPointTxid.count))"
            )
        }
        try fundFromAssetLockPreflight(recipients: recipients)
        let handle = self.handle
        let signerHandle = signer.handle
        let recipientRows = recipients
        let coreSigner = MnemonicResolver()

        return try await Task.detached(priority: .userInitiated) {
            () -> [UpdatedBalance] in
            var txidTuple: (
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
            ) = (
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            )
            outPointTxid.withUnsafeBytes { src in
                Swift.withUnsafeMutableBytes(of: &txidTuple) { dst in
                    dst.copyMemory(from: src)
                }
            }
            var outPoint = OutPointFFI(txid: txidTuple, vout: outPointVout)
            let ffiAddresses = recipientRows.map { r -> FundingAddressEntryFFI in
                FundingAddressEntryFFI(
                    address: PlatformAddressFFI(
                        address_type: r.addressType,
                        hash: Self.hashTuple(from: r.hash)
                    ),
                    has_balance: r.credits != nil,
                    balance: r.credits ?? 0
                )
            }
            let remainderIndex = UInt16(
                ffiAddresses.firstIndex(where: { !$0.has_balance }) ?? 0
            )
            let feeRows: [FeeStrategyStepFFI] = [
                FeeStrategyStepFFI(step_type: 1, index: remainderIndex)  // 1 = ReduceOutput
            ]
            var changeset = PlatformAddressChangeSetFFI(updated: nil, updated_count: 0)
            let result = withExtendedLifetime((signer, coreSigner)) {
                ffiAddresses.withUnsafeBufferPointer { addrBp in
                    feeRows.withUnsafeBufferPointer { feeBp in
                        platform_address_wallet_resume_fund_from_asset_lock_signer(
                            handle,
                            &outPoint,
                            platformAccountIndex,
                            addrBp.baseAddress,
                            UInt(addrBp.count),
                            feeBp.baseAddress,
                            UInt(feeBp.count),
                            signerHandle,
                            coreSigner.handle,
                            &changeset
                        )
                    }
                }
            }
            try result.check()

            defer { platform_address_wallet_free_changeset(&changeset) }
            return Self.decodeChangeset(&changeset)
        }.value
    }

    /// Validate the recipient list before the FFI sees it. The Rust
    /// side enforces the same invariants — we duplicate them here so
    /// the user gets a synchronous error before paying for the Task
    /// detach + handle marshaling.
    private func fundFromAssetLockPreflight(
        recipients: [FundFromAssetLockRecipient]
    ) throws {
        guard !recipients.isEmpty else {
            throw PlatformWalletError.invalidParameter("recipients is empty")
        }
        let noneCount = recipients.filter { $0.credits == nil }.count
        guard noneCount == 1 else {
            throw PlatformWalletError.invalidParameter(
                "Exactly one recipient must have credits = nil (the remainder recipient), found \(noneCount)"
            )
        }
        for r in recipients {
            // The Rust FFI accepts only addressType == 0 (P2PKH) — see
            // `impl TryFrom<PlatformAddressFFI> for PlatformAddress` in
            // packages/rs-platform-wallet-ffi/src/platform_address_types.rs,
            // which rejects P2SH with a type-specific message. Catch the
            // P2SH discriminant here too so the caller gets a synchronous
            // error instead of paying for the Task detach + signer pin
            // and then receiving the same rejection from Rust.
            guard r.addressType == 0 else {
                throw PlatformWalletError.invalidParameter(
                    "FundFromAssetLockRecipient.addressType must be 0 (P2PKH) for platform-address funding (got \(r.addressType))"
                )
            }
            guard r.hash.count == 20 else {
                throw PlatformWalletError.invalidParameter(
                    "FundFromAssetLockRecipient.hash must be exactly 20 bytes (got \(r.hash.count))"
                )
            }
        }
    }

    /// Convert an FFI changeset into the Swift-facing `[UpdatedBalance]`.
    /// Shared between the wallet-balance and resume entry points so
    /// both produce identically-shaped results.
    private static func decodeChangeset(
        _ changeset: inout PlatformAddressChangeSetFFI
    ) -> [UpdatedBalance] {
        guard let updatedPtr = changeset.updated, changeset.updated_count > 0 else {
            return []
        }
        return (0..<Int(changeset.updated_count)).map { i in
            let entry = updatedPtr[i]
            let hashData = withUnsafeBytes(of: entry.address.hash) { Data($0) }
            return UpdatedBalance(
                addressType: entry.address.address_type,
                hash: hashData,
                balance: entry.balance,
                nonce: entry.nonce,
                accountIndex: entry.account_index,
                addressIndex: entry.address_index
            )
        }
    }
}
