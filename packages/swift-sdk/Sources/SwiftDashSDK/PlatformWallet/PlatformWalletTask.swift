import Foundation
import DashSDKFFI

/// Per-operation context, captured before detaching from the caller's task.
/// The native scope exists only during a synchronous FFI call, never an await.
enum PlatformWalletTask {
    @TaskLocal static var permits: [PlatformWritePermit] = []

    static func detached<T: Sendable>(
        priority: TaskPriority? = nil,
        operation: sending @escaping () throws -> T
    ) -> Task<T, Error> {
        let confirmedPermits = permits
        return Task.detached(priority: priority) {
            try $permits.withValue(confirmedPermits) {
                try withNativeScope(confirmedPermits) {
                    Result { try operation() }
                }.get()
            }
        }
    }

    static func detached<T: Sendable>(
        priority: TaskPriority? = nil,
        operation: sending @escaping () -> T
    ) -> Task<T, Never> {
        let confirmedPermits = permits
        return Task.detached(priority: priority) {
            $permits.withValue(confirmedPermits) {
                withNativeScope(confirmedPermits, operation: operation)
            }
        }
    }

    private static func withNativeScope<T>(
        _ permits: [PlatformWritePermit],
        operation: () -> T
    ) -> T {
        guard let permit = permits.first else { return operation() }
        return withoutActuallyEscaping(operation) { operation in
            var output: T?
            let context = PlatformWalletScopedCallback {
                output = withNativeScope(Array(permits.dropFirst()), operation: operation)
            }
            platform_wallet_with_write_permit(
                permit.instanceID,
                permit.generation,
                permit.protocolVersion,
                { pointer in
                    guard let pointer else { return }
                    Unmanaged<PlatformWalletScopedCallback>
                        .fromOpaque(pointer).takeUnretainedValue().operation()
                },
                Unmanaged.passUnretained(context).toOpaque()
            )
            // Rust invokes the callback synchronously exactly once, and never
            // retains the unowned context. Preserve it through the entire call.
            withExtendedLifetime(context) {}
            return output!
        }
    }
}

private final class PlatformWalletScopedCallback {
    let operation: () -> Void

    init(operation: @escaping () -> Void) {
        self.operation = operation
    }
}

extension SDK {
    /// Keep an original preparation attached to async Platform wallet writes.
    ///
    /// Wallet wrappers carry this context across their detached Swift tasks and
    /// native worker queues. Rust checks it at preparation and broadcast guards.
    /// Reads and legacy callers retain their existing behavior. This scope does
    /// not wrap the separate low-level SDK transition APIs.
    public func withPlatformWritePermit<T>(
        _ permit: PlatformWritePermit,
        isolation: isolated (any Actor)? = #isolation,
        operation: () async throws -> T
    ) async rethrows -> T {
        try await PlatformWalletTask.$permits.withValue(
            PlatformWalletTask.permits + [permit],
            operation: operation,
            isolation: isolation
        )
    }
}
