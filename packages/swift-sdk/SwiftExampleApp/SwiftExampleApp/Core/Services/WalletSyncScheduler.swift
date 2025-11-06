import Foundation

// Debounces wallet sync requests and coalesces by walletId
public actor WalletSyncScheduler {
    public typealias FlushHandler = @Sendable (Set<String>) async -> Void

    private let debounceInterval: TimeInterval
    private let handler: FlushHandler

    private var pending: Set<String> = []
    private var scheduledTask: Task<Void, Never>?

    public init(debounce: TimeInterval = 0.5, handler: @escaping FlushHandler) {
        self.debounceInterval = debounce
        self.handler = handler
    }

    public func enqueue(walletId: String) {
        pending.insert(walletId)
        schedule()
    }

    public func enqueueMany(walletIds: [String]) {
        for id in walletIds { pending.insert(id) }
        schedule()
    }

    private func schedule() {
        guard scheduledTask == nil else { return }
        let nanos = UInt64(debounceInterval * 1_000_000_000)
        scheduledTask = Task { [weak self] in
            // Simple debounce: wait for interval, then flush on the actor
            try? await Task.sleep(nanoseconds: nanos)
            await self?.flushCurrent()
        }
    }

    private func flushCurrent() async {
        let toFlush = pending
        pending.removeAll()
        scheduledTask = nil
        if !toFlush.isEmpty {
            await handler(toFlush)
        }
    }

    // Exposed manual flush (useful for tests)
    public func flushNow() async {
        await flushCurrent()
    }
}
