import Foundation
import SwiftUI
import SwiftDashSDK

// MARK: - Observable wrapper for DashPayService
// The SDK's DashPayService is Sendable but not ObservableObject.
// This wrapper provides ObservableObject conformance for SwiftUI.

@MainActor
public final class ObservableDashPayService: ObservableObject {
    private let service: DashPayService

    @Published public var isLoading = false
    @Published public var error: Error?

    public init() {
        self.service = DashPayService()
    }

    // TODO: Implement actual DashPay functionality by wrapping service methods
    // For now this is a stub that allows the app to compile
}
