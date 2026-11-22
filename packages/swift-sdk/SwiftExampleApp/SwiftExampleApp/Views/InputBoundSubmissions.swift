import Foundation

/// The submissions of a form whose result answers the inputs it was read
/// for, such as the contract moderation reads of the Platform Queries
/// catalog: a result that comes back after the inputs were edited, or after
/// a newer submission, answers inputs no longer on screen and is dropped.
///
/// ``submit()`` hands out the token of a read; ``invalidate()`` is called
/// when an input is edited; a completion is shown only while
/// ``isCurrent(_:)`` holds for its token. Cancelling the Swift task would
/// not stop an FFI read already running, so the token is what keeps a late
/// answer off the screen.
@MainActor
final class InputBoundSubmissions {
    private var generation = 0

    /// The token of a read submitted now; any earlier read stops being
    /// current.
    func submit() -> Int {
        generation += 1
        return generation
    }

    /// The inputs were edited: no read running answers them any more.
    func invalidate() {
        generation += 1
    }

    /// Whether the read of `token` still answers the inputs on screen.
    func isCurrent(_ token: Int) -> Bool {
        token == generation
    }
}
