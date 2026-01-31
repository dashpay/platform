import Foundation
import SwiftUI

/// Base ViewModel with common loading, error, and result state.
/// Subclass for form/action ViewModels that need consistent UX.
@MainActor
class BaseViewModel: ObservableObject {

    @Published var isLoading = false
    @Published var errorMessage: String?
    @Published var showResult = false

    /// Set error message and show result section.
    func handleError(_ error: Error) {
        errorMessage = error.localizedDescription
        showResult = true
    }

    /// Clear error and result visibility. Override to also clear form fields and result data.
    func reset() {
        errorMessage = nil
        showResult = false
    }
}
