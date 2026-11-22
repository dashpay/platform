import Foundation

/// The submissions of a form whose result answers the inputs it was read
/// for, such as the contract moderation reads of the Platform Queries
/// catalog: a result that comes back after the inputs were edited, or after
/// a newer submission, answers inputs no longer on screen and is dropped.
///
/// ``submit(_:)`` takes the inputs as they are at Execute and hands out a
/// ``InputBoundSubmission`` carrying them with its token, so the read is
/// built from that snapshot and not from a form edited since; ``invalidate()``
/// is called when an input is edited; a completion is shown only while
/// ``isCurrent(_:)`` holds for its submission. Cancelling the Swift task would
/// not stop an FFI read already running, so the token is what keeps a late
/// answer off the screen.
@MainActor
final class InputBoundSubmissions {
    private var generation = 0

    /// A read submitted now with `inputs`; any earlier read stops being
    /// current.
    func submit(_ inputs: [String: String] = [:]) -> InputBoundSubmission {
        generation += 1
        return InputBoundSubmission(token: generation, inputs: inputs)
    }

    /// The inputs were edited: no read running answers them any more.
    func invalidate() {
        generation += 1
    }

    /// Whether `submission` still answers the inputs on screen.
    func isCurrent(_ submission: InputBoundSubmission) -> Bool {
        submission.token == generation
    }
}

/// One submitted read: its token and the inputs it was submitted with.
struct InputBoundSubmission: Sendable, Equatable {
    let token: Int
    let inputs: [String: String]
}
