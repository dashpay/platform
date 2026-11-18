import Foundation
import DashSDKFFI

/// One wallet's shielded balance state as Rust pushed it through
/// `on_shielded_balance_changed_fn`. Rust reads the wallet's state at
/// points such as a first bind, a scan ending or failing, a spend, a
/// rebind, a Clear or unregister, and emits an update when what it read
/// differs from what it last emitted. Rust counts an update as emitted
/// once it has called the host callback; it does not know whether the
/// host accepted it, and it does not resend an unchanged state. An update
/// therefore carries the latest state Rust read, not every transition: a
/// spend that reserves notes and releases them again before Rust next
/// reads produces no update. The latest accepted update is the wallet's
/// current state as far as Rust has read it, and a host holds no balance
/// of its own.
public struct ShieldedBalanceUpdate: Sendable, Equatable {
    public let walletId: Data
    /// Starts at 1 per wallet and strictly increases for the life of the
    /// native manager. It identifies an emitted state, not a ledger
    /// change: a higher revision was emitted later.
    public let revision: UInt64
    public let state: ShieldedBalanceState

    public init(walletId: Data, revision: UInt64, state: ShieldedBalanceState) {
        self.walletId = walletId
        self.revision = revision
        self.state = state
    }

    /// The wallet's shielded balance (its ordinary accounts; identity
    /// accounts left out), or `nil` when it is not a known balance: the
    /// state is not ready, or the ready ledger is not established.
    public var ordinaryBalance: UInt64? {
        guard case .ready(let balance) = state else { return nil }
        return balance.ordinaryBalance
    }

    /// What one account can spend, or `nil` when it is not a known
    /// balance: the state is not ready, the account is not bound, or the
    /// account has no history yet.
    public func accountBalance(_ account: UInt32) -> UInt64? {
        guard case .ready(let balance) = state else { return nil }
        return balance.accountBalance(account)
    }
}

public enum ShieldedBalanceState: Sendable, Equatable {
    /// No shielded accounts are bound for the wallet. Rust read nothing,
    /// so this is not a known zero.
    case unbound
    /// Accounts are bound, but their local ledger is not available yet.
    case restoreIncomplete
    case ready(ShieldedReadyBalance)
}

/// The balance carried by a ready update.
public struct ShieldedReadyBalance: Sendable, Equatable {
    /// Spendable credits of the ordinary accounts. Not the wallet's
    /// balance unless `ordinaryBalanceEstablished`; read
    /// [`ordinaryBalance`] instead of this field.
    public let ordinarySpendableCredits: UInt64
    /// Whether every ordinary account rests on restored notes or a
    /// completed scan. False for a wallet restored without history, whose
    /// credits read 0 before its first scan.
    public let ordinaryBalanceEstablished: Bool
    /// Every bound account, identity accounts included.
    public let accounts: [UInt32: ShieldedBalanceAccount]

    public init(
        ordinarySpendableCredits: UInt64,
        ordinaryBalanceEstablished: Bool,
        accounts: [UInt32: ShieldedBalanceAccount]
    ) {
        self.ordinarySpendableCredits = ordinarySpendableCredits
        self.ordinaryBalanceEstablished = ordinaryBalanceEstablished
        self.accounts = accounts
    }

    /// The wallet's shielded balance, or `nil` while not established.
    public var ordinaryBalance: UInt64? {
        ordinaryBalanceEstablished ? ordinarySpendableCredits : nil
    }

    /// One account's spendable credits, or `nil` when the account is not
    /// bound or Rust did not establish its amount (`established`).
    public func accountBalance(_ account: UInt32) -> UInt64? {
        guard let entry = accounts[account], entry.established else { return nil }
        return entry.spendableCredits
    }
}

/// One bound account in a pushed update.
public struct ShieldedBalanceAccount: Sendable, Equatable {
    /// Credits a send from this account can select now. A known amount
    /// only when `established`.
    public let spendableCredits: UInt64
    /// Whether `spendableCredits` is a known amount for this account. This
    /// is Rust's decision, sent per account; Swift does not derive it from
    /// `source` or anything else.
    public let established: Bool
    /// The account's local scan coverage. A persisted index of zero is
    /// distinct from absent scan history (`nil`).
    public let lastScannedIndex: UInt64?
    /// What backs the amount, for diagnostics only.
    public let source: ShieldedBalanceSource

    public init(
        spendableCredits: UInt64,
        established: Bool,
        lastScannedIndex: UInt64?,
        source: ShieldedBalanceSource
    ) {
        self.spendableCredits = spendableCredits
        self.established = established
        self.lastScannedIndex = lastScannedIndex
        self.source = source
    }
}

extension ShieldedBalanceUpdate {
    /// Copies a native update into Swift values. `ffi.accounts` is borrowed
    /// for the callback only, so this runs inside the callback.
    nonisolated static func decode(_ ffi: ShieldedBalanceUpdateFFI) throws -> ShieldedBalanceUpdate {
        let walletId = withUnsafeBytes(of: ffi.wallet_id) { Data($0) }
        let state: ShieldedBalanceState
        switch ffi.status {
        case SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND:
            guard ffi.accounts_count == 0 else {
                throw PlatformWalletError.deserialization("Unbound shielded balance update contains accounts")
            }
            state = .unbound
        case SHIELDED_LOCAL_BALANCE_STATUS_FFI_RESTORE_INCOMPLETE:
            guard ffi.accounts_count == 0 else {
                throw PlatformWalletError.deserialization("Incomplete shielded balance update contains accounts")
            }
            state = .restoreIncomplete
        case SHIELDED_LOCAL_BALANCE_STATUS_FFI_READY:
            // Rust reports an empty registration as unbound, so a ready
            // update without accounts is malformed, not a known zero.
            guard ffi.accounts_count > 0 else {
                throw PlatformWalletError.deserialization("Ready shielded balance update has no accounts")
            }
            state = .ready(ShieldedReadyBalance(
                ordinarySpendableCredits: ffi.ordinary_spendable_credits,
                ordinaryBalanceEstablished: ffi.ordinary_balance_established,
                accounts: try decodeAccounts(ffi.accounts, count: ffi.accounts_count)))
        default:
            throw PlatformWalletError.deserialization("Unknown shielded balance update status")
        }
        return ShieldedBalanceUpdate(walletId: walletId, revision: ffi.revision, state: state)
    }

    nonisolated private static func decodeAccounts(
        _ pointer: UnsafePointer<ShieldedBalanceAccountFFI>?,
        count rawCount: UInt
    ) throws -> [UInt32: ShieldedBalanceAccount] {
        guard let count = Int(exactly: rawCount),
              count <= Int.max / MemoryLayout<ShieldedBalanceAccountFFI>.stride else {
            throw PlatformWalletError.deserialization("Shielded balance update has invalid account storage")
        }
        guard count > 0 else { return [:] }
        guard let pointer else {
            throw PlatformWalletError.deserialization("Shielded balance update has invalid account storage")
        }
        var accounts: [UInt32: ShieldedBalanceAccount] = [:]
        for entry in UnsafeBufferPointer(start: pointer, count: count) {
            let source: ShieldedBalanceSource
            switch entry.source {
            case SHIELDED_BALANCE_SOURCE_FFI_NO_HISTORY: source = .noHistory
            case SHIELDED_BALANCE_SOURCE_FFI_RESTORED: source = .restored
            case SHIELDED_BALANCE_SOURCE_FFI_SCANNED_THIS_SESSION: source = .scannedThisSession
            default:
                throw PlatformWalletError.deserialization("Unknown shielded balance source")
            }
            guard accounts[entry.account_index] == nil else {
                throw PlatformWalletError.deserialization("Duplicate shielded account in balance update")
            }
            accounts[entry.account_index] = ShieldedBalanceAccount(
                spendableCredits: entry.spendable_credits,
                established: entry.established,
                lastScannedIndex: entry.has_last_scanned_index ? entry.last_scanned_index : nil,
                source: source)
        }
        return accounts
    }

    /// Whether `incoming` replaces the update held for its wallet: only a
    /// higher revision does, so an update that reaches the main actor
    /// after a newer one is dropped.
    nonisolated static func shouldReplace(
        held: ShieldedBalanceUpdate?,
        with incoming: ShieldedBalanceUpdate
    ) -> Bool {
        guard let held else { return true }
        return incoming.revision > held.revision
    }
}

/// C trampoline for
/// `EventHandlerCallbacksExtension.on_shielded_balance_changed_fn`. Runs on
/// a Rust thread. The update, including its borrowed account array, is
/// copied here before the main-actor hop.
func shieldedBalanceChangedCallback(
    context: UnsafeMutableRawPointer?,
    update: UnsafePointer<ShieldedBalanceUpdateFFI>?
) {
    guard let context, let update else { return }
    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let decoded: ShieldedBalanceUpdate
    do {
        decoded = try ShieldedBalanceUpdate.decode(update.pointee)
    } catch {
        SDKLogger.event(
            "shielded_balance_update_rejected",
            category: .shielded,
            severity: .error,
            error: error
        )
        return
    }
    Task { @MainActor [weak manager = handler.manager] in
        manager?.handleShieldedBalanceChanged(decoded)
    }
}

extension PlatformWalletManager {
    /// Publishes `update` unless the manager is shutting down or not
    /// configured, or a newer update for the wallet is already held.
    ///
    /// A manager is configured once and never again after shutdown, so
    /// its native manager is the only one that can call back. Rust
    /// emits nothing after its shutdown returned; an update it emitted
    /// before is still queued for the main actor and is dropped here
    /// because `shutdownRequested` is already set. Rust already counts a
    /// dropped update as emitted and does not resend it. Stopping
    /// shielded sync and Clear do not filter updates: the balance state
    /// stays Rust's, and Rust sends the post-Clear state itself.
    func handleShieldedBalanceChanged(_ update: ShieldedBalanceUpdate) {
        guard !shutdownRequested, isConfigured else { return }
        guard ShieldedBalanceUpdate.shouldReplace(held: shieldedBalances[update.walletId], with: update) else {
            return
        }
        shieldedBalances[update.walletId] = update
    }
}
