import SwiftUI
import SwiftDashSDK

/// Streamlined entry form for the most common register-a-contract
/// case: a single-token contract with no document types and the
/// minimum metadata Drive requires (singular / plural forms,
/// decimals, base supply). Everything more advanced — distribution
/// rules, control rules, change rules, groups — falls back to the
/// "Manual" path on the source picker.
///
/// The form doesn't submit on its own. "Continue" pushes the user
/// into `TransitionDetailView` with the synthesized `tokenSchemas`
/// JSON pre-filled, so identity selection, key resolution, signing,
/// and the actual `dataContractCreate` round-trip stay on the
/// existing submit pipeline. That keeps this view responsible for
/// the schema-shaping concern and nothing else.
struct QuickBasicTokenView: View {
    @Environment(\.dismiss) private var dismiss

    @State private var singular = "Coin"
    @State private var plural = "Coins"
    @State private var decimals = "8"
    @State private var baseSupply = "1000000"
    @State private var maxSupply = ""
    @State private var shouldCapitalize = true
    @State private var keywords = ""
    @State private var description = ""

    @State private var presentSubmit = false
    @State private var validationError: String?

    var body: some View {
        Form {
            Section("Names") {
                LabeledRow(label: "Singular") {
                    TextField("Coin", text: $singular)
                        .textInputAutocapitalization(.words)
                }
                LabeledRow(label: "Plural") {
                    TextField("Coins", text: $plural)
                        .textInputAutocapitalization(.words)
                }
                Toggle("Capitalize in display", isOn: $shouldCapitalize)
            }

            Section("Supply") {
                LabeledRow(label: "Decimals") {
                    TextField("8", text: $decimals)
                        .keyboardType(.numberPad)
                }
                LabeledRow(label: "Base Supply") {
                    TextField("1000000", text: $baseSupply)
                        .keyboardType(.numberPad)
                }
                LabeledRow(label: "Max Supply") {
                    TextField("Optional", text: $maxSupply)
                        .keyboardType(.numberPad)
                }
            }

            Section("Metadata (Optional)") {
                TextField("Keywords (comma separated)", text: $keywords)
                    .textInputAutocapitalization(.never)
                TextField("Description", text: $description)
            }

            Section {
                Button {
                    if validate() {
                        presentSubmit = true
                    }
                } label: {
                    Label("Continue", systemImage: "arrow.right.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .disabled(!isValid)

                if let validationError = validationError {
                    Text(validationError)
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
        }
        .navigationTitle("Quick Basic Token")
        .navigationBarTitleDisplayMode(.inline)
        .navigationDestination(isPresented: $presentSubmit) {
            TransitionDetailView(
                transitionKey: "dataContractCreate",
                transitionLabel: "Register Token Contract",
                initialInputs: synthesizedInputs()
            )
        }
    }

    // MARK: Validation

    /// Pre-flight check on the basic form fields. Mirrors what the
    /// downstream submit path expects but keeps the user from having
    /// to navigate into the heavier `TransitionDetailView` just to
    /// see the same error.
    private var isValid: Bool {
        guard !singular.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        guard !plural.trimmingCharacters(in: .whitespaces).isEmpty else { return false }
        guard let d = UInt8(decimals), d <= 18 else { return false }
        guard let _ = UInt64(baseSupply) else { return false }
        if !maxSupply.trimmingCharacters(in: .whitespaces).isEmpty,
           UInt64(maxSupply) == nil {
            return false
        }
        return true
    }

    private func validate() -> Bool {
        if singular.trimmingCharacters(in: .whitespaces).isEmpty
            || plural.trimmingCharacters(in: .whitespaces).isEmpty {
            validationError = "Singular and plural names are required."
            return false
        }
        guard let d = UInt8(decimals), d <= 18 else {
            validationError = "Decimals must be a whole number between 0 and 18."
            return false
        }
        guard UInt64(baseSupply) != nil else {
            validationError = "Base supply must be a whole, non-negative number."
            return false
        }
        if !maxSupply.trimmingCharacters(in: .whitespaces).isEmpty,
           UInt64(maxSupply) == nil {
            validationError = "Max supply must be a whole number, or blank for unlimited."
            return false
        }
        if let max = UInt64(maxSupply), let base = UInt64(baseSupply), max < base {
            validationError = "Max supply must be greater than or equal to base supply."
            return false
        }
        validationError = nil
        _ = d
        return true
    }

    // MARK: JSON synthesis

    /// Build the form-input dictionary that `TransitionDetailView`
    /// reads on appear. Returns the `tokenSchemas` JSON shaped for
    /// `parseTokenConfiguration` (decimals + localizations under
    /// `conventions`, top-level `baseSupply` / `maxSupply`), plus
    /// the optional metadata fields the user filled in.
    private func synthesizedInputs() -> [String: String] {
        let trimmedSingular = singular.trimmingCharacters(in: .whitespaces)
        let trimmedPlural = plural.trimmingCharacters(in: .whitespaces)
        let decimalsInt = Int(decimals) ?? 8
        let baseSupplyInt = UInt64(baseSupply) ?? 0
        let maxSupplyInt = UInt64(maxSupply.trimmingCharacters(in: .whitespaces))

        // `TokenConfiguration`, `TokenConfigurationConvention`, and
        // `TokenConfigurationLocalization` are all tagged enums with
        // `#[serde(tag = "$formatVersion")]` on the Rust side, so
        // every level needs an explicit `$formatVersion: "0"` for
        // serde's enum dispatch to pick the V0 variant. Every other
        // `TokenConfigurationV0` field has a `#[serde(default)]`,
        // so we only need to fill in `conventions` (no default) and
        // `baseSupply` / `maxSupply`.
        var tokenZero: [String: Any] = [
            "$formatVersion": "0",
            "conventions": [
                "$formatVersion": "0",
                "decimals": decimalsInt,
                "localizations": [
                    "en": [
                        "$formatVersion": "0",
                        "shouldCapitalize": shouldCapitalize,
                        "singularForm": trimmedSingular,
                        "pluralForm": trimmedPlural,
                    ]
                ]
            ],
            "baseSupply": baseSupplyInt,
        ]
        if let m = maxSupplyInt {
            tokenZero["maxSupply"] = m
        }

        let tokenSchemas: [String: Any] = ["0": tokenZero]
        var inputs: [String: String] = [:]
        if let str = jsonString(from: tokenSchemas) {
            inputs["tokenSchemas"] = str
        }
        // Token-only contracts: clear documentSchemas so the form's
        // "at least one schema" guard doesn't trip on the form's
        // default placeholder document type.
        inputs["documentSchemas"] = ""

        let trimmedKeywords = keywords.trimmingCharacters(in: .whitespaces)
        if !trimmedKeywords.isEmpty {
            inputs["keywords"] = trimmedKeywords
        }
        let trimmedDesc = description.trimmingCharacters(in: .whitespaces)
        if !trimmedDesc.isEmpty {
            inputs["description"] = trimmedDesc
        }

        return inputs
    }

    private func jsonString(from value: Any) -> String? {
        guard let data = try? JSONSerialization.data(
            withJSONObject: value,
            options: [.prettyPrinted, .sortedKeys]
        ),
              let str = String(data: data, encoding: .utf8) else {
            return nil
        }
        return str
    }
}

/// Tight `Form` row that pairs a leading caption with a trailing
/// editor control. The default `LabeledContent` aligns its trailing
/// content to the trailing edge with right-alignment, which makes
/// numeric `TextField`s render cramped against the trailing inset.
/// This wrapper gives the editor the full remaining width.
private struct LabeledRow<Content: View>: View {
    let label: String
    @ViewBuilder let content: () -> Content

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer(minLength: 12)
            content()
                .multilineTextAlignment(.trailing)
        }
    }
}
