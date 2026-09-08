import Foundation
import SwiftData
@testable import SwiftDashSDK

/// `ModelFetching` seam double: serves every read live except the one
/// model type it is told to fault (none by default), and records the
/// reads it saw so a test can prove which fetch failed — or, with no
/// fault, count how many reads a code path issued.
final class FetchFaultInjector: ModelFetching, @unchecked Sendable {
    struct ReadFault: Error {}

    private let live = LiveModelFetcher()
    private let faulted: ObjectIdentifier?
    private let lock = NSLock()
    private var reads: [String] = []

    init(faulting model: (any PersistentModel.Type)? = nil) {
        faulted = model.map { ObjectIdentifier($0) }
    }

    /// Model names in the order they were read, the faulted one included.
    var observedReads: [String] {
        lock.lock()
        defer { lock.unlock() }
        return reads
    }

    func fetch<T: PersistentModel>(
        _ descriptor: FetchDescriptor<T>,
        in context: ModelContext
    ) throws -> [T] {
        lock.lock()
        reads.append(String(describing: T.self))
        lock.unlock()
        guard ObjectIdentifier(T.self) != faulted else { throw ReadFault() }
        return try live.fetch(descriptor, in: context)
    }
}
