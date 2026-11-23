import DashSDKFFI
import Foundation

/// Whom a shielded tip to a username pays: the identity the name resolves
/// to and the raw 43-byte Orchard address its DashPay profile publishes.
/// `reviewShieldedTip` returns it and `sendShieldedTip` takes it back
/// unchanged, so a name or profile that changed in between cannot redirect
/// the payment.
public struct ShieldedTipRecipient: Sendable, Equatable {
    /// The 32-byte identity id the username resolves to.
    public let identityId: Data
    /// The raw 43-byte Orchard address the identity's profile publishes.
    public let address: Data

    public init(identityId: Data, address: Data) {
        self.identityId = identityId
        self.address = address
    }
}

/// How a tip recipient compares with the one this wallet last confirmed for
/// the same name.
public enum ShieldedTipRecipientChange: Sendable, Equatable {
    /// This wallet has not sent a tip to the name before.
    case new
    /// The same identity and address as last confirmed.
    case unchanged
    /// The name now resolves to another identity, or its profile publishes
    /// another address. `previous` is the recipient last confirmed.
    case changed(previous: ShieldedTipRecipient)
}

/// What the user confirms before tipping a name.
public struct ShieldedTipReview: Sendable, Equatable {
    /// Whom the name pays now. Pass it back to `sendShieldedTip` unchanged.
    public let recipient: ShieldedTipRecipient
    /// How it compares with the recipient last confirmed for the name.
    public let change: ShieldedTipRecipientChange
    /// An earlier shielded payment from this wallet to the recipient may
    /// still execute, so a send is refused (`shieldedTipPending`) until a
    /// shielded sync resolves it.
    public let unresolved: Bool

    public init(recipient: ShieldedTipRecipient, change: ShieldedTipRecipientChange, unresolved: Bool) {
        self.recipient = recipient
        self.change = change
        self.unresolved = unresolved
    }

    init(ffi: ShieldedTipReviewFFI) throws {
        let previous = ShieldedTipRecipient(
            identityId: withUnsafeBytes(of: ffi.previous_identity_id) { Data($0) },
            address: withUnsafeBytes(of: ffi.previous_address) { Data($0) })
        let change: ShieldedTipRecipientChange
        switch ffi.change {
        case SHIELDED_TIP_RECIPIENT_CHANGE_FFI_NEW: change = .new
        case SHIELDED_TIP_RECIPIENT_CHANGE_FFI_UNCHANGED: change = .unchanged
        case SHIELDED_TIP_RECIPIENT_CHANGE_FFI_CHANGED: change = .changed(previous: previous)
        default:
            throw PlatformWalletError.deserialization("Unknown shielded tip recipient change")
        }
        self.init(
            recipient: ShieldedTipRecipient(
                identityId: withUnsafeBytes(of: ffi.recipient_identity_id) { Data($0) },
                address: withUnsafeBytes(of: ffi.recipient_address) { Data($0) }),
            change: change,
            unresolved: ffi.unresolved)
    }
}

/// The account a shielded tip is paid from.
public enum ShieldedTipSource: Sendable, Equatable {
    /// The 32-byte id of a wallet identity: its own shielded account pays,
    /// proved with the seed when the tip is sent.
    case identity(Data)
    /// A shielded account named by its number. Rust refuses the identity
    /// account of a slot that holds an identity; pay from `.identity` then.
    case account(UInt32)
}

/// The shielded balance of a wallet identity's own account, as far as the
/// wallet can vouch for it. Show these states; do not add up stored notes.
public enum ShieldedIdentityBalance: Sendable, Equatable {
    /// The seed has not proved in this session which account is the
    /// identity's. `prepareShieldedIdentityAddress` proves it.
    case unverified
    /// The account is proved, but its ledger is not established (not bound,
    /// or neither restored nor scanned): the balance is not known, which is
    /// not zero.
    case unavailable(account: UInt32)
    /// Spendable credits of the identity's account.
    case available(account: UInt32, credits: UInt64)

    init(ffi: ShieldedIdentityBalanceFFI) throws {
        switch ffi.state {
        case SHIELDED_IDENTITY_BALANCE_STATE_FFI_UNVERIFIED: self = .unverified
        case SHIELDED_IDENTITY_BALANCE_STATE_FFI_UNAVAILABLE: self = .unavailable(account: ffi.account)
        case SHIELDED_IDENTITY_BALANCE_STATE_FFI_AVAILABLE:
            self = .available(account: ffi.account, credits: ffi.credits)
        default:
            throw PlatformWalletError.deserialization("Unknown shielded identity balance state")
        }
    }
}

extension PlatformWalletManager {
    /// Parse a tip amount the user typed in DASH into exact credits. Rust
    /// owns the rules (one optional `.` or `,` separator, no signs, grouping
    /// or exponents, nonzero, no digits finer than one credit). Show the
    /// parsed amount in the confirmation, not the typed text: `1,000` is one
    /// DASH.
    ///
    /// - Throws: `PlatformWalletError.invalidParameter` with Rust's reason for
    ///   a refused amount.
    nonisolated public static func parseShieldedTipAmount(_ text: String) throws -> UInt64 {
        var credits: UInt64 = 0
        try text.withCString { textPtr in
            try platform_wallet_parse_shielded_tip_amount(textPtr, &credits).check()
        }
        return credits
    }

    /// Resolve `username` for the user to confirm a shielded tip: whom it
    /// pays now, how that compares with the recipient this wallet last
    /// confirmed for the name, and whether an earlier shielded payment to it
    /// is still unresolved. Changes nothing; fetches the name and the
    /// profile from the network with proof.
    ///
    /// Throws `walletOperation` with Rust's reason for a name that does not
    /// exist, a profile that publishes no valid shielded address, or a
    /// wallet whose shielded accounts are not bound;
    /// `shieldedRecoveryCorrupted` / `shieldedRecoveryKeysRequired` when the
    /// wallet cannot tell whom a saved shielded spend pays.
    public func reviewShieldedTip(walletId: Data, username: String) async throws -> ShieldedTipReview {
        try ensureConfigured()
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId must be exactly 32 bytes")
        }
        return try await withAdmittedNativeOperation("reviewShieldedTip") { handle in
            var out = ShieldedTipReviewFFI()
            try walletId.withUnsafeBytes { widRaw in
                guard let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    throw PlatformWalletError.invalidParameter("walletId baseAddress is nil")
                }
                try username.withCString { usernamePtr in
                    try platform_wallet_manager_review_shielded_tip(
                        handle, widPtr, usernamePtr, &out
                    ).check()
                }
            }
            return try ShieldedTipReview(ffi: out)
        }
    }

    /// Send a shielded tip of `amount` credits to `username` from `source`,
    /// paying `expectedRecipient`, which must be the `recipient` of the
    /// review the user confirmed, passed back unchanged. Rust resolves the
    /// name again and refuses a recipient that changed. `memo` is an
    /// optional UTF-8 text on the recipient's note, encoded and checked
    /// (at most 32 bytes) by Rust.
    ///
    /// `resolver` supplies the Orchard spend authority for this one call,
    /// as for `shieldedTransfer`. The proof runs on a detached task.
    ///
    /// Throws:
    /// - `shieldedSpendUnconfirmed`: the tip may have left the wallet and
    ///   may still execute. Never send it again; a shielded sync resolves
    ///   it. (`transactionBroadcastUnconfirmed` means the same.)
    /// - `shieldedTipPending`: refused, nothing sent. Another tip is being
    ///   sent, or an earlier payment to the recipient is unresolved; a
    ///   shielded sync resolves it.
    /// - `identityNotFound`: the wallet holds no identity `source` names.
    ///   Nothing sent.
    /// - Anything else: the tip was not sent.
    public func sendShieldedTip(
        walletId: Data,
        source: ShieldedTipSource,
        username: String,
        expectedRecipient: ShieldedTipRecipient,
        amount: UInt64,
        memo: String? = nil,
        resolver: MnemonicResolver
    ) async throws {
        try ensureConfigured()
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId must be exactly 32 bytes")
        }
        guard expectedRecipient.identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("expectedRecipient.identityId must be exactly 32 bytes")
        }
        guard expectedRecipient.address.count == 43 else {
            throw PlatformWalletError.invalidParameter("expectedRecipient.address must be exactly 43 bytes")
        }
        // `.identity` crosses as a non-null id and `.account` as a null id
        // with the account number, which is how the C export reads them.
        let sourceIdentity: Data?
        let sourceAccount: UInt32
        switch source {
        case .identity(let identityId):
            guard identityId.count == 32 else {
                throw PlatformWalletError.invalidParameter("source identity id must be exactly 32 bytes")
            }
            sourceIdentity = identityId
            sourceAccount = 0
        case .account(let account):
            sourceIdentity = nil
            sourceAccount = account
        }
        guard let resolverHandle = resolver.handle else {
            throw PlatformWalletError.invalidParameter("MnemonicResolver has no handle")
        }
        // Counted as the other native manager operations are, so a teardown
        // waits for this call to return before it frees the manager.
        try admitNativeOp("sendShieldedTip")
        defer { finishNativeOp() }
        let handle = self.handle
        try await Task.detached(priority: .userInitiated) {
            // Guaranteed keepalive of the resolver across the FFI call, whose
            // vtable callback fires during it: same rationale as
            // `shieldedTransfer`.
            try withExtendedLifetime(resolver) {
                try PlatformWalletManager.withBytes(walletId) { widPtr in
                    try PlatformWalletManager.withBytes(sourceIdentity) { sourcePtr in
                        try PlatformWalletManager.withBytes(expectedRecipient.identityId) { expectedIdPtr in
                            try PlatformWalletManager.withBytes(expectedRecipient.address) { expectedAddressPtr in
                                try username.withCString { usernamePtr in
                                    let send: (UnsafePointer<CChar>?) throws -> Void = { memoPtr in
                                        try platform_wallet_manager_send_shielded_tip(
                                            handle, widPtr, resolverHandle, sourcePtr,
                                            sourceAccount, usernamePtr, expectedIdPtr,
                                            expectedAddressPtr, amount, memoPtr
                                        ).check()
                                    }
                                    if let memo {
                                        try memo.withCString { try send($0) }
                                    } else {
                                        try send(nil)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }.value
    }

    /// The shielded balance of the wallet identity `identityId`'s own
    /// account, from local state only. Show the state: `.unverified` until
    /// `prepareShieldedIdentityAddress` proves the identity's account in this
    /// session, `.unavailable` while its ledger is not established.
    ///
    /// Rust waits at most 100 ms for the shielded store; while a scan holds
    /// it this throws `walletOperation` (busy): keep the last state and read
    /// again later. Throws `identityNotFound` when the wallet holds no
    /// identity `identityId`.
    public func shieldedIdentityBalance(walletId: Data, identityId: Data) async throws -> ShieldedIdentityBalance {
        try ensureConfigured()
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter("walletId must be exactly 32 bytes")
        }
        guard identityId.count == 32 else {
            throw PlatformWalletError.invalidParameter("identityId must be exactly 32 bytes")
        }
        return try await withAdmittedNativeOperation("shieldedIdentityBalance") { handle in
            var out = ShieldedIdentityBalanceFFI()
            try PlatformWalletManager.withBytes(walletId) { widPtr in
                try PlatformWalletManager.withBytes(identityId) { idPtr in
                    try platform_wallet_manager_shielded_identity_balance(
                        handle, widPtr, idPtr, &out
                    ).check()
                }
            }
            return try ShieldedIdentityBalance(ffi: out)
        }
    }

    /// Run `body` with a pointer to `data`'s bytes, or with `nil` when
    /// `data` is `nil`. A non-nil `data` must not be empty.
    nonisolated private static func withBytes<R>(
        _ data: Data?,
        _ body: (UnsafePointer<UInt8>?) throws -> R
    ) throws -> R {
        guard let data else { return try body(nil) }
        return try data.withUnsafeBytes { raw in
            guard let pointer = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                throw PlatformWalletError.invalidParameter("byte buffer baseAddress is nil")
            }
            return try body(pointer)
        }
    }
}
