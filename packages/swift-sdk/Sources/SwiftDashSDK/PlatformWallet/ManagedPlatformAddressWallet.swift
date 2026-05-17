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
        /// `0 = P2PKH`, `1 = P2SH`. Mirrors the Rust-side `PlatformAddress` discriminant.
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

    /// Destination address for the change output the wrapper appends to
    /// every transfer. Carries no `credits` field — the wrapper computes
    /// the change amount itself as `sum(inputs) - sum(outputs)`.
    public struct ChangeAddress: Sendable {
        /// `0 = P2PKH`, `1 = P2SH`. Mirrors the Rust-side `PlatformAddress` discriminant.
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data

        public init(addressType: UInt8, hash: Data) {
            self.addressType = addressType
            self.hash = hash
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
    /// The `AddressFundsTransferTransition` protocol requires
    /// `sum(inputs) == sum(outputs)` and forbids any output address from
    /// also appearing as an input. This wrapper handles both invariants:
    /// - Picks the smallest set of inputs (largest-balance first) that
    ///   covers `sum(outputs) + feeBuffer`.
    /// - Routes the change to `changeAddress` if supplied — typically a
    ///   fresh, unused HD address pulled from the SwiftData address
    ///   pool (lowest `addressIndex` with `isUsed == false`, the same
    ///   selection rule the Receive screen uses). When `changeAddress`
    ///   is `nil` the wrapper falls back to reserving the smallest
    ///   non-recipient balance-bearing address — workable but it
    ///   accumulates change onto an existing address.
    ///
    /// Fee strategy is `ReduceOutput(change_index)` so each recipient
    /// gets exactly its requested amount and the change output absorbs
    /// the on-chain fee.
    ///
    /// The signer must be able to sign for the selected inputs — i.e.
    /// the `KeychainSigner` resolves their derivation paths via
    /// SwiftData + the wallet mnemonic (the `0xFF` branch in
    /// `KeychainSigner.swift`).
    /// Cushion held back so the change output stays positive after the on-chain
    /// fee is deducted (the transfer uses `ReduceOutput(change_index)`).
    /// Observed fee for a 1-input/2-output transition is ~6.5M credits;
    /// this is intentionally an order of magnitude larger so estimation
    /// drift doesn't force an "insufficient" failure in normal use.
    private static let feeBuffer: UInt64 = 100_000_000

    @discardableResult
    public func transfer(
        accountIndex: UInt32,
        outputs: [TransferOutput],
        changeAddress: ChangeAddress? = nil,
        signer: KeychainSigner
    ) async throws -> [UpdatedBalance] {
        guard !outputs.isEmpty else {
            throw PlatformWalletError.invalidParameter("outputs is empty")
        }

        // Reject duplicate recipient addresses. Rust's parse_outputs
        // (platform_address_types.rs:189-204) inserts into a
        // `BTreeMap<PlatformAddress, Credits>` keyed by address, so a
        // duplicate would silently overwrite earlier entries — Swift
        // would still sum every output toward the change calculation,
        // leaving the transition misbalanced. We key on `hash` only:
        // P2PKH/P2SH share the same 20-byte hash space, so the same
        // hash with different `addressType` is still ambiguous.
        let outputHashes = Set(outputs.map { $0.hash })
        guard outputHashes.count == outputs.count else {
            throw PlatformWalletError.invalidParameter(
                "Duplicate recipient addresses in outputs"
            )
        }

        // Sum recipient amounts. Reject overflow rather than silently
        // wrapping (would let a caller smuggle bogus amounts past the
        // protocol's sum check).
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

        // Read available balances. We pick inputs from balance-bearing
        // addresses; the change destination must differ from both the
        // recipient set and the chosen inputs.
        let recipientHashes = Set(outputs.map { $0.hash })

        // Validate explicit change-address up front if the caller supplied one.
        if let cc = changeAddress {
            guard cc.hash.count == 20 else {
                throw PlatformWalletError.walletOperation(
                    "changeAddress.hash must be exactly 20 bytes (got \(cc.hash.count))"
                )
            }
            guard !recipientHashes.contains(cc.hash) else {
                throw PlatformWalletError.walletOperation(
                    "changeAddress collides with a recipient address."
                )
            }
        }

        let balanced = try addressesWithBalances()
            .filter { $0.balance > 0 }
            .filter { !recipientHashes.contains($0.hash) }
            .sorted { $0.balance > $1.balance }

        // Pick the smallest set of inputs (largest-first) that covers
        // recipients + fee buffer. When the caller hasn't supplied a
        // dedicated change address, leave the last balance-bearing
        // candidate aside so we can use it as change.
        let reserveOneForChange = (changeAddress == nil)
        var selectedInputs: [AddressBalance] = []
        var totalInputs: UInt64 = 0
        for b in balanced {
            if reserveOneForChange && selectedInputs.count == balanced.count - 1 { break }
            // Skip if this address is the explicit change destination.
            if let cc = changeAddress, b.hash == cc.hash { continue }
            selectedInputs.append(b)
            totalInputs += b.balance
            if totalInputs >= totalRecipientCredits + Self.feeBuffer { break }
        }
        guard totalInputs >= totalRecipientCredits + Self.feeBuffer else {
            throw PlatformWalletError.walletOperation(
                "Insufficient platform balance: have \(totalInputs) credits across \(selectedInputs.count) input(s), need at least \(totalRecipientCredits + Self.feeBuffer)"
            )
        }
        let selectedHashes = Set(selectedInputs.map { $0.hash })

        // Resolve the change destination: caller-supplied address wins
        // (fresh HD address from the unused pool); otherwise reserve a
        // balance-bearing address that's neither input nor recipient.
        let resolvedChange: (addressType: UInt8, hash: Data)
        if let cc = changeAddress {
            guard !selectedHashes.contains(cc.hash) else {
                throw PlatformWalletError.walletOperation(
                    "changeAddress collides with a selected input address."
                )
            }
            resolvedChange = (cc.addressType, cc.hash)
        } else {
            guard let fallback = balanced.first(where: { !selectedHashes.contains($0.hash) }) else {
                throw PlatformWalletError.walletOperation(
                    "Could not find a wallet address distinct from inputs to use as the change destination — pass a fresh HD address via `changeAddress`."
                )
            }
            resolvedChange = (fallback.addressType, fallback.hash)
        }

        // Marshal explicit inputs.
        var ffiInputs: [ExplicitInputFFI] = []
        ffiInputs.reserveCapacity(selectedInputs.count)
        for inp in selectedInputs {
            let inputTuple = Self.hashTuple(from: inp.hash)
            ffiInputs.append(
                ExplicitInputFFI(
                    address: PlatformAddressFFI(address_type: inp.addressType, hash: inputTuple),
                    balance: inp.balance
                )
            )
        }

        // Marshal recipient outputs.
        var ffiOutputs: [AddressBalanceEntryFFI] = []
        ffiOutputs.reserveCapacity(outputs.count + 1)
        for out in outputs {
            let outTuple = Self.hashTuple(from: out.hash)
            ffiOutputs.append(
                AddressBalanceEntryFFI(
                    address: PlatformAddressFFI(address_type: out.addressType, hash: outTuple),
                    balance: out.credits,
                    nonce: 0,
                    account_index: 0,
                    address_index: 0
                )
            )
        }

        // Append the change output to the resolved change address —
        // guaranteed to be distinct from every input and recipient
        // (the protocol rejects outputs that also appear as inputs).
        let changeAmount = totalInputs - totalRecipientCredits
        let changeTuple = Self.hashTuple(from: resolvedChange.hash)
        ffiOutputs.append(
            AddressBalanceEntryFFI(
                address: PlatformAddressFFI(address_type: resolvedChange.addressType, hash: changeTuple),
                balance: changeAmount,
                nonce: 0,
                account_index: 0,
                address_index: 0
            )
        )

        // Fee strategy: take the fee out of the change output (last index)
        // so recipients get their requested amounts unchanged.
        let changeIndex = UInt16(ffiOutputs.count - 1)
        let feeStrategy: [FeeStrategyStepFFI] = [
            FeeStrategyStepFFI(step_type: 1, index: changeIndex)  // 1 = ReduceOutput
        ]

        let handle = self.handle
        let signerHandle = signer.handle
        let inRows = ffiInputs
        let outRows = ffiOutputs
        let feeRows = feeStrategy

        return try await Task.detached(priority: .userInitiated) { () -> [UpdatedBalance] in
            _ = signer
            var changeset = PlatformAddressChangeSetFFI(updated: nil, updated_count: 0)
            let result = inRows.withUnsafeBufferPointer {
                inBp -> PlatformWalletFFIResult in
                outRows.withUnsafeBufferPointer { outBp in
                    feeRows.withUnsafeBufferPointer { feeBp in
                        platform_address_wallet_transfer(
                            handle,
                            accountIndex,
                            INPUT_SELECTION_TYPE_EXPLICIT,
                            inBp.baseAddress,
                            UInt(inBp.count),
                            nil,
                            0,
                            outBp.baseAddress,
                            UInt(outBp.count),
                            feeBp.baseAddress,
                            UInt(feeBp.count),
                            signerHandle,
                            &changeset
                        )
                    }
                }
            }
            try result.check()

            defer { platform_address_wallet_free_changeset(&changeset) }

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
}
