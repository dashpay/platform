import Foundation
import SwiftDashSDK

/// The SDK a moderation read goes out on, with the generation `AppState`
/// gave it (``AppState/sdkGeneration``): a result is shown only while that
/// generation is still the current one.
struct ModerationConnection {
    let sdk: SDK
    let generation: Int
}

extension AppState {
    /// The current SDK and its generation; `nil` without an SDK.
    var moderationConnection: ModerationConnection? {
        sdk.map { ModerationConnection(sdk: $0, generation: sdkGeneration) }
    }

    /// Whether `connection` is still the current one.
    func isCurrent(_ connection: ModerationConnection) -> Bool {
        sdk != nil && sdkGeneration == connection.generation
    }
}

/// One page a paged moderation read answered: its rows and the cursor of the
/// next page, `nil` on the last.
struct ModerationPage<Item: Sendable, Cursor: Sendable>: Sendable {
    let items: [Item]
    let nextCursor: Cursor?
}

/// The rows of a paged moderation read (a list's entries, a type's removal
/// records, the team actions), loaded a page at a time by the cursor each
/// page returns.
///
/// Only the current request writes: every ``reload(_:)`` starts a new
/// generation, and a completion from an older one (a "Load More" or a lookup
/// still in flight when the user refreshed) is dropped, rows, cursor, error
/// and loading state alike. Cancelling the Swift task would not stop the FFI
/// read already running, so the generation is what keeps results apart.
@MainActor
final class ModerationPagedLoader<Item: Sendable, Cursor: Sendable>: ObservableObject {
    /// Reads the page after `cursor` (`nil` for the first); `nil` when the
    /// SDK returned no data.
    typealias Fetch = @Sendable (Cursor?) async throws -> ModerationPage<Item, Cursor>?

    @Published private(set) var items: [Item] = []
    @Published private(set) var nextCursor: Cursor?
    @Published private(set) var isLoading = false
    @Published private(set) var errorMessage: String?
    /// Whether the current source answered at least once.
    @Published private(set) var loadedOnce = false

    private var generation = 0
    private var fetch: Fetch?
    /// Whether the connection the current fetch reads through is still the
    /// app's; a completion after it was replaced is dropped, and Load More
    /// does not query it.
    private var connectionIsCurrent: @MainActor () -> Bool = { true }

    /// Start over from the first page of `fetch`, dropping what is shown and
    /// whatever an older request still brings back.
    func reload(
        _ fetch: @escaping Fetch,
        connectionIsCurrent: @escaping @MainActor () -> Bool = { true }
    ) async {
        generation += 1
        self.fetch = fetch
        self.connectionIsCurrent = connectionIsCurrent
        items = []
        nextCursor = nil
        errorMessage = nil
        loadedOnce = false
        await load(fetch, cursor: nil, generation: generation)
    }

    /// Load the page after the last one shown, unless a request is running
    /// or the last page was the last.
    func loadMore() async {
        guard let fetch, let cursor = nextCursor, !isLoading, connectionIsCurrent() else { return }
        await load(fetch, cursor: cursor, generation: generation)
    }

    /// A new read is about to start: drop the rows, cursor and error shown
    /// and whatever a running request brings back, and show loading at once,
    /// before the read itself is queued.
    func begin() {
        generation += 1
        fetch = nil
        items = []
        nextCursor = nil
        errorMessage = nil
        loadedOnce = false
        isLoading = true
    }

    /// Show nothing, as before the first read, and drop whatever a running
    /// request brings back.
    func clear() {
        generation += 1
        fetch = nil
        items = []
        nextCursor = nil
        isLoading = false
        errorMessage = nil
        loadedOnce = false
    }

    /// Show `message` instead of any rows, ending whatever is in flight.
    func fail(_ message: String) {
        generation += 1
        fetch = nil
        items = []
        nextCursor = nil
        isLoading = false
        errorMessage = message
        loadedOnce = true
    }

    private func load(_ fetch: Fetch, cursor: Cursor?, generation: Int) async {
        isLoading = true
        errorMessage = nil
        do {
            let page = try await fetch(cursor)
            guard generation == self.generation, connectionIsCurrent() else { return }
            items.append(contentsOf: page?.items ?? [])
            nextCursor = page?.nextCursor
        } catch {
            guard generation == self.generation, connectionIsCurrent() else { return }
            errorMessage = moderationErrorText(error)
        }
        isLoading = false
        loadedOnce = true
    }
}

/// The answer of a single moderation read (the fee pots, the action counts,
/// a team action's signers) that a reload, a pull-to-refresh and the first
/// appearance all start. As with ``ModerationPagedLoader``, every
/// ``reload(_:)`` starts a new generation and only the current one writes, so
/// an older read finishing late cannot replace a newer answer with stale
/// values or an error.
@MainActor
final class ModerationSingleLoader<Value: Sendable>: ObservableObject {
    @Published private(set) var load: ModerationLoad<Value> = .idle

    private var generation = 0

    /// Read again with `fetch`, dropping whatever an older read brings back,
    /// and an answer that arrives after the connection it was read through
    /// stopped being current.
    func reload(
        _ fetch: @Sendable () async throws -> Value,
        connectionIsCurrent: @MainActor () -> Bool = { true }
    ) async {
        generation += 1
        let current = generation
        load = .loading
        do {
            let value = try await fetch()
            guard current == generation, connectionIsCurrent() else { return }
            load = .loaded(value)
        } catch {
            guard current == generation, connectionIsCurrent() else { return }
            load = .failed(moderationErrorText(error))
        }
    }

    /// Show nothing, as before the first read, and drop whatever a running
    /// read brings back: the connection was replaced.
    func clear() {
        generation += 1
        load = .idle
    }

    /// Show `message`, ending whatever is in flight.
    func fail(_ message: String) {
        generation += 1
        load = .failed(message)
    }
}
