import SwiftUI

/// The confirmed release of a screen's held payment failure that has no lock
/// to resume (`PaymentFailure.releasableAfterReconciliation`): the screen
/// keeps refusing a fresh payment until the user confirms they checked the
/// original one in the history. Shows nothing for any other failure (a held
/// lock is resumed or reclaimed, a `.notSent` one never held).
struct ReleasePaymentHoldButton: View {
    let failure: PaymentFailure
    let onRelease: () -> Void

    @State private var confirming = false

    var body: some View {
        if failure.releasableAfterReconciliation {
            Button {
                confirming = true
            } label: {
                Text("Payment checked in history — release")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.bordered)
            .accessibilityIdentifier("paymentHold.release")
            .confirmationDialog(
                "Release only after the transaction history shows this payment as completed or failed. A new payment from this screen may otherwise pay twice.",
                isPresented: $confirming,
                titleVisibility: .visible
            ) {
                Button("Release", role: .destructive, action: onRelease)
            }
        }
    }
}
