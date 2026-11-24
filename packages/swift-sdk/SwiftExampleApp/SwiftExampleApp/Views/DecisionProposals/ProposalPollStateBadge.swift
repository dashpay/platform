import SwiftUI

/// The state of a proposal's poll in a listing row, as Platform reported it
/// on the row's own read. The listings carry no poll state, and the device
/// clock does not decide it either: only a read of the poll does.
enum ProposalPollBadgeState: Equatable {
    /// Platform reports no result yet.
    case voting
    /// Platform reports a result; `detail` adds how it ended, when known.
    case closed(detail: String?)
}

/// A small capsule showing ``ProposalPollBadgeState``; nothing while the
/// state is unknown (loading, failed, or a poll that was never opened).
struct ProposalPollStateBadge: View {
    let state: ProposalPollBadgeState?
    let accessibilityIdentifier: String

    var body: some View {
        if let state {
            Label(text(for: state), systemImage: icon(for: state))
                .font(.caption2.weight(.semibold))
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(color(for: state).opacity(0.15))
                .foregroundColor(color(for: state))
                .clipShape(Capsule())
                .fixedSize()
                .accessibilityIdentifier(accessibilityIdentifier)
        }
    }

    private func text(for state: ProposalPollBadgeState) -> String {
        switch state {
        case .voting:
            return "Voting"
        case .closed(let detail?):
            return "Closed: \(detail)"
        case .closed(nil):
            return "Closed"
        }
    }

    private func icon(for state: ProposalPollBadgeState) -> String {
        switch state {
        case .voting:
            return "hourglass"
        case .closed:
            return "lock.fill"
        }
    }

    private func color(for state: ProposalPollBadgeState) -> Color {
        switch state {
        case .voting:
            return .blue
        case .closed:
            return .secondary
        }
    }
}
