import Combine
import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - Presentation

/// Plain-language text for a durable identity-funded shield (Type 21)
/// record. The status itself is decided in Rust; this only labels it.
///
/// A record whose signed bytes do not decode has no `identityId`. The
/// wallet cannot resend it, and until it is stopped it blocks new identity
/// shields from every identity in the wallet, so it gets its own wording.
enum ShieldedIdentityDebitRecoveryDisplay {
    static func statusTitle(
        _ status: ShieldedIdentityDebitRecoveryStatus,
        identityId: Data?
    ) -> String {
        if identityId == nil, status != .unknown { return "Unreadable record" }
        switch status {
        case .retrying: return "Retrying"
        case .parked: return "Waiting for the note"
        case .unknown: return "Outcome unknown"
        }
    }

    static func statusDetail(
        _ status: ShieldedIdentityDebitRecoveryStatus,
        identityId: Data?
    ) -> String {
        if identityId == nil {
            if status == .unknown {
                return "Automatic retry was stopped. The identity that funded it "
                    + "is unknown, and it may still have executed."
            }
            return "The identity that funded this shield is unknown and its saved "
                + "transaction cannot be resent. It blocks new identity shields in "
                + "this wallet until you stop retrying it."
        }
        switch status {
        case .retrying:
            return "Automatic retry is enabled. The wallet may resend the "
                + "original signed shield during a shielded sync."
        case .parked:
            return "Its identity nonce is used up, so it either already "
                + "executed or never will. The wallet is waiting for the "
                + "shielded scan to see its note."
        case .unknown:
            return "Automatic retry was stopped. It may still have executed; "
                + "the wallet keeps the record until the scan sees its note."
        }
    }

    /// Only an active record can be abandoned; `unknown` already was.
    static func canStopRetrying(_ status: ShieldedIdentityDebitRecoveryStatus) -> Bool {
        status != .unknown
    }

    /// Identity alias from the wallet's rows when known, else a short id.
    /// A record whose transition could not be read has no identity id.
    static func identityLabel(
        for record: ShieldedIdentityDebitRecoveryRecord,
        identities: [PersistentIdentity]
    ) -> String {
        identityLabel(identityId: record.identityId, identities: identities)
    }

    static func identityLabel(identityId: Data?, identities: [PersistentIdentity]) -> String {
        guard let identityId else { return "Unknown identity" }
        if let identity = identities.first(where: { $0.identityId == identityId }) {
            return identity.displayName
        }
        let hex = identityId.map { String(format: "%02x", $0) }.joined()
        return String(hex.prefix(12)) + "…"
    }

    static func amountText(_ credits: UInt64) -> String {
        formatCreditsAsDash(credits)
    }

    /// Shown before Stop Retrying. Stopping never cancels an earlier
    /// broadcast, whether or not the record could be read.
    static func stopRetryingWarning(identityId: Data?) -> String {
        let cancel = "This does not cancel the payment. It may already have "
            + "executed, or it may still execute later."
        if identityId == nil {
            return cancel + " The identity it belongs to is unknown, so a new "
                + "shield from any identity in this wallet may become an "
                + "additional debit if it comes from that identity."
        }
        return cancel + " The wallet stops resending it, and a new shield from "
            + "this identity may become an additional debit."
    }

    /// Footer of the section. Status-neutral on purpose: what a sync does
    /// depends on each record (only a retrying readable one is resent, and
    /// only an active one blocks new shields), so each row's status text
    /// carries that.
    static let footerText = "Shields from an identity's balance whose outcome "
        + "is not known yet. A shielded sync checks each shield for its note; "
        + "each row shows what the wallet can still do with it."
}

// MARK: - Read state

/// Result of reading the records: never read, last read succeeded, or last
/// read failed. A failed refresh keeps the last successful records, so an
/// in-progress shield never disappears because one read failed.
/// Generic over the record so the transitions are testable without FFI rows.
struct RecoveryReadState<Record: Equatable>: Equatable {
    enum Status: Equatable {
        case notLoaded
        case loaded
        case failed(String)
    }

    private(set) var status: Status = .notLoaded
    private(set) var records: [Record] = []

    mutating func applySuccess(_ records: [Record]) {
        self.records = records
        status = .loaded
    }

    mutating func applyFailure(_ message: String) {
        status = .failed(message)
    }

    /// The latest read succeeded and found nothing.
    var isKnownEmpty: Bool { status == .loaded && records.isEmpty }

    var failureMessage: String? {
        if case .failed(let message) = status { return message }
        return nil
    }
}

typealias ShieldedIdentityDebitRecoveryReadState = RecoveryReadState<ShieldedIdentityDebitRecoveryRecord>

// MARK: - Check Now

/// Result of a Check Now started on a screen, from that call's own outcome
/// rather than the shared `ShieldedService.lastError`, which other passes
/// also write.
enum ShieldedCheckNowStatus: Equatable {
    case idle
    case checking
    /// The sync was requested; whether this wallet's scan succeeded is unknown.
    case requested
    case failed(String)
    case didNotRun(String)

    static func after(_ outcome: ShieldedService.ManualSyncOutcome) -> ShieldedCheckNowStatus {
        switch outcome {
        case .requested: return .requested
        case .failed(let message): return .failed(message)
        case .alreadyRunning:
            return .didNotRun("A shielded sync was already running. This list updates when it finishes.")
        case .notStarted(let reason): return .didNotRun(reason)
        }
    }

    /// Line shown under Check Now; `nil` when there is nothing to say.
    var text: String? {
        switch self {
        case .idle, .checking: return nil
        case .requested:
            return "Sync requested. The list shows the wallet's current records; "
                + "this screen cannot tell whether the scan for this wallet succeeded."
        case .failed(let message): return "Check failed: \(message)"
        case .didNotRun(let reason): return "Check did not run. \(reason)"
        }
    }

    var isFailure: Bool {
        switch self {
        case .failed, .didNotRun: return true
        default: return false
        }
    }
}

/// The line under a Check Now button.
struct ShieldedCheckNowStatusLine: View {
    let status: ShieldedCheckNowStatus

    var body: some View {
        if let text = status.text {
            Text(text)
                .font(.caption)
                .foregroundColor(status.isFailure ? .red : .secondary)
                .accessibilityIdentifier("shielded.checkNow.status")
        }
    }
}

// MARK: - Sync completion

/// Whether a shielded sync completion event covered `walletId`, so
/// wallet-owned state shown for it (records, spendable credits) is re-read.
enum ShieldedSyncEventScope {
    static func covers(_ event: ShieldedSyncEvent?, walletId: Data) -> Bool {
        event?.result(for: walletId) != nil
    }
}

// MARK: - Read ordering

/// Orders the record reads of one wallet. Each read gets a generation; only
/// the newest started read may apply its result. While a read is in flight a
/// new request is coalesced into one follow-up read after it finishes. A
/// superseding read (after Stop Retrying) starts at once, and the older read
/// still in flight can no longer apply.
struct RecoveryReadCoordinator: Equatable {
    private(set) var latestStarted: UInt64 = 0
    private(set) var inFlight: UInt64?
    private(set) var followUpRequested = false

    /// The generation to start now, or `nil` when a read is already in
    /// flight (one follow-up is then remembered).
    mutating func request() -> UInt64? {
        guard inFlight == nil else {
            followUpRequested = true
            return nil
        }
        return start()
    }

    /// Start a read that supersedes any read in flight.
    mutating func startSuperseding() -> UInt64 {
        followUpRequested = false
        return start()
    }

    /// A read finished. `apply` says whether its result may be shown;
    /// `next` is a coalesced follow-up read to start now, if any.
    mutating func finish(_ generation: UInt64) -> (apply: Bool, next: UInt64?) {
        let apply = generation == latestStarted
        guard generation == inFlight else { return (apply, nil) }
        inFlight = nil
        guard followUpRequested else { return (apply, nil) }
        followUpRequested = false
        return (apply, start())
    }

    /// Forget every read (the wallet changed): none of them may apply.
    mutating func reset() {
        latestStarted += 1
        inFlight = nil
        followUpRequested = false
    }

    private mutating func start() -> UInt64 {
        latestStarted += 1
        inFlight = latestStarted
        return latestStarted
    }
}

/// The wallet manager instance and wallet whose records a list shows.
struct RecoveryReadScope: Equatable {
    let manager: ObjectIdentifier
    let walletId: Data
}

/// Read ordering for the scope a list currently shows. A new scope (another
/// wallet, or a replaced wallet manager) forgets every read started before
/// it, so a read on a replaced manager never applies. An invalidation for
/// the shown wallet starts a read that supersedes the ones in flight; one
/// for another wallet does nothing.
struct RecoveryReadTracker: Equatable {
    private(set) var scope: RecoveryReadScope?
    private(set) var reads = RecoveryReadCoordinator()

    /// Request a read for `scope`. `scopeChanged` tells the caller to drop
    /// the shown records; `start` is the generation to start now, or `nil`
    /// when a read for this scope is in flight (a follow-up is remembered).
    mutating func request(for scope: RecoveryReadScope) -> (scopeChanged: Bool, start: UInt64?) {
        var scopeChanged = false
        if self.scope != scope {
            self.scope = scope
            reads.reset()
            scopeChanged = true
        }
        return (scopeChanged, reads.request())
    }

    /// The records of `walletId` changed elsewhere (Stop Retrying in another
    /// window): the generation of a superseding read, or `nil` when another
    /// wallet is shown.
    mutating func invalidate(walletId: Data) -> UInt64? {
        guard let scope, scope.walletId == walletId else { return nil }
        return reads.startSuperseding()
    }

    /// Forget every read started so far (a shielded Clear).
    mutating func reset() {
        reads.reset()
    }

    mutating func finish(_ generation: UInt64) -> (apply: Bool, next: UInt64?) {
        reads.finish(generation)
    }
}

// MARK: - Model

/// Loads and abandons the per-wallet identity-funded shield records.
/// Failures stay local to this model so the activity list keeps working.
@MainActor
final class ShieldedIdentityDebitRecoveryModel: ObservableObject {
    @Published private(set) var state = ShieldedIdentityDebitRecoveryReadState()
    @Published var actionError: String?
    @Published private(set) var isAbandoning = false

    private var tracker = RecoveryReadTracker()
    /// The manager of the shown scope. Each read, a coalesced follow-up
    /// included, runs on this one, not on the one an earlier read captured.
    private var manager: PlatformWalletManager?

    typealias RecordsReader = @MainActor (PlatformWalletManager, Data) async throws
        -> [ShieldedIdentityDebitRecoveryRecord]
    /// Reads a wallet's records from Rust. Injected by tests to count reads.
    private let readRecords: RecordsReader

    typealias Abandoner = @MainActor (PlatformWalletManager, Data, UInt32, Data) async throws -> Void
    /// Stops automatic retry of one record (wallet, account, activity id),
    /// always with the user's acknowledgement. Injected by tests.
    private let abandon: Abandoner

    init(
        readRecords: @escaping RecordsReader = { manager, walletId in
            try await manager.shieldedIdentityDebitRecoveryRecords(walletId: walletId)
        },
        abandon: @escaping Abandoner = { manager, walletId, accountIndex, activityId in
            try await manager.abandonShieldedIdentityDebit(
                walletId: walletId,
                accountIndex: accountIndex,
                activityId: activityId,
                acknowledgePossibleExecution: true
            )
        }
    ) {
        self.readRecords = readRecords
        self.abandon = abandon
    }

    var records: [ShieldedIdentityDebitRecoveryRecord] { state.records }

    func load(walletManager: PlatformWalletManager, walletId: Data) async {
        manager = walletManager
        let request = tracker.request(for: RecoveryReadScope(
            manager: ObjectIdentifier(walletManager),
            walletId: walletId
        ))
        // Another wallet's or manager's records must never show here.
        if request.scopeChanged {
            state = ShieldedIdentityDebitRecoveryReadState()
        }
        guard let generation = request.start else { return }
        await runReads(from: generation, walletId: walletId)
    }

    /// The records of `walletId` changed elsewhere: re-read, superseding
    /// any read in flight. Ignored when another wallet is shown.
    func invalidate(walletId: Data) async {
        guard let generation = tracker.invalidate(walletId: walletId) else { return }
        await runReads(from: generation, walletId: walletId)
    }

    /// Forget the records and every read already started, so none of them
    /// can apply. Used after a shielded Clear, which purges the records
    /// without a sync event.
    func resetSnapshot() {
        state = ShieldedIdentityDebitRecoveryReadState()
        tracker.reset()
    }

    private var changeSubscription: AnyCancellable?

    /// Re-read whenever `service` publishes a recovery-records change (Stop
    /// Retrying in any window, or an identity shield from Send). A change
    /// for another wallet is ignored by `invalidate`. Calling it again
    /// replaces the subscription; the value present when subscribing is old
    /// and is skipped.
    func observeRecoveryChanges(from service: ShieldedService) {
        changeSubscription = service.$lastRecoveryRecordsChange
            .dropFirst()
            .compactMap { $0 }
            .sink { [weak self] change in
                Task { @MainActor in
                    await self?.invalidate(walletId: change.walletId)
                }
            }
    }

    /// Stop retrying one record and, only if that succeeded, publish the
    /// change through `publish` so every open list for the wallet, this one
    /// included, re-reads once from it. A failed abandon publishes nothing.
    func stopRetryingAndPublish(
        accountIndex: UInt32,
        activityId: Data,
        walletManager: PlatformWalletManager,
        walletId: Data,
        publish: @MainActor (Data) -> Void
    ) async {
        let abandoned = await stopRetrying(
            accountIndex: accountIndex,
            activityId: activityId,
            walletManager: walletManager,
            walletId: walletId
        )
        if abandoned {
            publish(walletId)
        }
    }

    /// Call only after the user confirmed `stopRetryingWarning`. Returns
    /// whether the record was abandoned. This does not re-read: the caller
    /// publishes the change and every open list for the wallet, this one
    /// included, re-reads once from that event.
    @discardableResult
    func stopRetrying(
        accountIndex: UInt32,
        activityId: Data,
        walletManager: PlatformWalletManager,
        walletId: Data
    ) async -> Bool {
        isAbandoning = true
        defer { isAbandoning = false }
        var abandoned = false
        do {
            try await abandon(walletManager, walletId, accountIndex, activityId)
            abandoned = true
        } catch {
            actionError = error.localizedDescription
        }
        return abandoned
    }

    private func runReads(from generation: UInt64, walletId: Data) async {
        var next: UInt64? = generation
        while let current = next {
            let result: Result<[ShieldedIdentityDebitRecoveryRecord], Error>
            if let manager {
                do {
                    result = .success(try await readRecords(manager, walletId))
                } catch {
                    result = .failure(error)
                }
            } else {
                result = .failure(PlatformWalletError.invalidHandle("No wallet manager"))
            }
            let outcome = tracker.finish(current)
            if outcome.apply {
                switch result {
                case .success(let records): state.applySuccess(records)
                case .failure(let error): state.applyFailure(error.localizedDescription)
                }
            }
            next = outcome.next
        }
    }
}

// MARK: - Section

/// "In-Progress Identity Shields": the durable Type 21 records, shown above
/// Pending so an interrupted shield is visible after a restart. Renders
/// nothing when there are no records and nothing failed.
struct ShieldedIdentityDebitRecoverySection: View {
    @ObservedObject var model: ShieldedIdentityDebitRecoveryModel
    let identities: [PersistentIdentity]
    let isChecking: Bool
    let canCheck: Bool
    let checkStatus: ShieldedCheckNowStatus
    let onCheckNow: () -> Void
    let onStopRetrying: (ShieldedIdentityDebitRecoveryRecord) -> Void

    @State private var pendingStop: ShieldedIdentityDebitRecoveryRecord?

    var body: some View {
        if !model.records.isEmpty {
            recordsSection
        }
        if let loadError = model.state.failureMessage {
            Section {
                Text("In-progress identity shields could not be read: \(loadError)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("shieldedActivity.identityShields.loadError")
            }
        }
    }

    private var recordsSection: some View {
        Section {
            ForEach(model.records, id: \.recoveryKey) { record in
                recordRow(record)
            }
            Button {
                onCheckNow()
            } label: {
                HStack {
                    Label("Check Now", systemImage: "arrow.clockwise")
                    if isChecking {
                        Spacer()
                        ProgressView().controlSize(.small)
                    }
                }
            }
            .disabled(isChecking || !canCheck)
            .accessibilityIdentifier("shieldedActivity.identityShields.checkNow")
            ShieldedCheckNowStatusLine(status: checkStatus)
        } header: {
            Text("In-Progress Identity Shields")
        } footer: {
            Text(ShieldedIdentityDebitRecoveryDisplay.footerText)
        }
        .confirmationDialog(
            "Stop retrying this shield?",
            isPresented: Binding(
                get: { pendingStop != nil },
                set: { if !$0 { pendingStop = nil } }
            ),
            titleVisibility: .visible,
            presenting: pendingStop
        ) { record in
            Button("Stop Retrying", role: .destructive) {
                onStopRetrying(record)
            }
            .accessibilityIdentifier("shieldedActivity.identityShields.confirmStop")
            Button("Cancel", role: .cancel) {}
        } message: { record in
            Text(ShieldedIdentityDebitRecoveryDisplay.stopRetryingWarning(
                identityId: record.identityId
            ))
        }
    }

    @ViewBuilder
    private func recordRow(_ record: ShieldedIdentityDebitRecoveryRecord) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Image(systemName: "person.crop.circle.badge.clock")
                    .foregroundColor(.orange)
                Text(ShieldedIdentityDebitRecoveryDisplay.identityLabel(
                    for: record,
                    identities: identities
                ))
                .font(.subheadline.weight(.medium))
                .lineLimit(1)
                .truncationMode(.middle)
                Spacer()
                if let amount = record.amount {
                    Text(ShieldedIdentityDebitRecoveryDisplay.amountText(amount))
                        .font(.subheadline.weight(.semibold))
                }
            }
            Text(ShieldedIdentityDebitRecoveryDisplay.statusTitle(
                record.status,
                identityId: record.identityId
            ))
                .font(.caption.weight(.medium))
                .foregroundColor(.orange)
            Text(ShieldedIdentityDebitRecoveryDisplay.statusDetail(
                record.status,
                identityId: record.identityId
            ))
                .font(.caption)
                .foregroundColor(.secondary)
            if ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(record.status) {
                Button("Stop Retrying", role: .destructive) {
                    pendingStop = record
                }
                .buttonStyle(.borderless)
                .font(.caption)
                .disabled(model.isAbandoning)
                .accessibilityIdentifier("shieldedActivity.identityShields.stopRetrying")
            }
        }
        .padding(.vertical, 2)
        .accessibilityIdentifier("shieldedActivity.identityShields.row")
    }
}

private extension ShieldedIdentityDebitRecoveryRecord {
    /// A record is identified by its account and activity id within a wallet.
    var recoveryKey: String {
        "\(accountIndex)-" + activityId.map { String(format: "%02x", $0) }.joined()
    }
}
