import SwiftUI

struct SeedBackupView: View {
    let mnemonic: String
    let onConfirm: () -> Void
    
    @Environment(\.dismiss) private var dismiss
    @State private var wroteItDown: Bool = false
    @State private var isSubmitting: Bool = false
    
    private var words: [String] {
        mnemonic.split(separator: " ").map(String.init)
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Recovery Phrase")
                .font(.title2.bold())
            
            Text("Write down these 12 words in order and store them somewhere safe. Do not take screenshots or share them with anyone.")
                .font(.subheadline)
                .foregroundColor(.secondary)
            
            // Display words in a grid with indices
            let columns = [GridItem(.flexible()), GridItem(.flexible())]
            LazyVGrid(columns: columns, spacing: 8) {
                ForEach(Array(words.enumerated()), id: \.offset) { idx, word in
                    HStack(spacing: 8) {
                        Text(String(format: "%2d.", idx + 1))
                            .font(.body.monospacedDigit())
                            .foregroundColor(.secondary)
                            .frame(width: 28, alignment: .trailing)
                        Text(word)
                            .font(.body)
                            .textSelection(.enabled)
                        Spacer()
                    }
                    .padding(8)
                    .background(Color(.secondarySystemBackground))
                    .cornerRadius(8)
                }
            }
            .padding(.top, 8)
            
            Toggle(isOn: $wroteItDown) {
                Text("I wrote it down")
                    .font(.body)
            }
            .padding(.top, 8)
            
            Spacer()
            
            HStack {
                Button("Back") {
                    dismiss()
                }
                .padding(.vertical, 10)
                .frame(maxWidth: .infinity)
                .background(Color(.secondarySystemBackground))
                .cornerRadius(10)
                
                Button("Create Wallet") {
                    guard !isSubmitting else { return }
                    isSubmitting = true
                    onConfirm()
                }
                .padding(.vertical, 10)
                .frame(maxWidth: .infinity)
                .background((wroteItDown && !isSubmitting) ? Color.blue : Color.gray)
                .foregroundColor(.white)
                .cornerRadius(10)
                .disabled(!wroteItDown || isSubmitting)
            }
        }
        .padding()
        .navigationTitle("Backup Seed")
        .navigationBarTitleDisplayMode(.inline)
    }
}

struct SeedBackupView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            SeedBackupView(
                mnemonic: "abandon ability able about above absent absorb abstract absurd abuse access accident",
                onConfirm: {}
            )
        }
    }
}
