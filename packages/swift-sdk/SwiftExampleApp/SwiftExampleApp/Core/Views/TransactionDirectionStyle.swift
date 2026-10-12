import SwiftUI
import SwiftDashSDK

/// Direction icon and colour shared by the transaction list and detail views.
enum TransactionDirectionStyle {
    static func icon(for direction: UInt32) -> String {
        switch direction {
        case CoreDirectionCode.incoming: return "arrow.down.circle.fill"
        case CoreDirectionCode.outgoing: return "arrow.up.circle.fill"
        case CoreDirectionCode.internalTransfer: return "arrow.triangle.2.circlepath"
        case CoreDirectionCode.coinJoin: return "shuffle.circle.fill"
        default: return "questionmark.circle"
        }
    }

    /// Internal transfers share the outgoing colour: they still pay a fee.
    static func color(for direction: UInt32) -> Color {
        switch direction {
        case CoreDirectionCode.incoming: return .green
        case CoreDirectionCode.outgoing, CoreDirectionCode.internalTransfer: return .red
        case CoreDirectionCode.coinJoin: return .blue
        default: return .secondary
        }
    }
}
