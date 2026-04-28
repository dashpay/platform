import SwiftUI
import SwiftDashSDK

/// Holds temporary state for state transitions — document pricing, purchase
/// eligibility, and the most recent purchase error. Injected as an
/// `@EnvironmentObject` so that transition flows can share state across views.
@MainActor
class TransitionState: ObservableObject {
    @Published var documentPrice: UInt64?
    @Published var canPurchaseDocument: Bool = false
    @Published var documentPurchaseError: String?

    func reset() {
        documentPrice = nil
        canPurchaseDocument = false
        documentPurchaseError = nil
    }
}
