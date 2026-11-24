// MachineTextEntry.swift
// SwiftExampleApp
//
// Inputs for text a machine reads back: JSON, identifiers (base58, hex),
// addresses, keys, DPNS labels, index and type names. iOS keyboard
// conveniences corrupt that text: autocorrection rewrites ids, auto
// capitalization changes the first letter, and smart punctuation turns a
// typed `"` into a curly quote, so JSON stops parsing.
//
// - `.machineTextEntry()` turns off autocorrection and auto capitalization
//   on a single-line `TextField`.
// - `PlainTextEditor` replaces `TextEditor` for JSON and identifier lists,
//   and `PlainTextField` replaces a one-line JSON `TextField`.
//   SwiftUI has no modifier for smart quotes, dashes or insert/delete, and
//   turning autocorrection off does not reliably turn them off, so this
//   wraps a `UITextView` and sets every one of those traits to "no".

import SwiftUI
import UIKit

extension View {
    /// Turns off autocorrection and auto capitalization for a field that
    /// holds machine-read text. Pass `false` to keep the system defaults
    /// (for a generic field that sometimes holds prose).
    func machineTextEntry(_ enabled: Bool = true) -> some View {
        self
            .autocorrectionDisabled(enabled)
            .textInputAutocapitalization(enabled ? .never : nil)
    }
}

/// A `TextEditor` replacement for JSON and identifier lists: no smart
/// quotes, smart dashes, smart insert/delete, autocorrection, auto
/// capitalization or spell checking.
///
/// The accessibility identifier is a parameter, not a modifier, so it
/// lands on the `UITextView` itself where UI automation finds it.
/// `.disabled(_:)` is honoured through the SwiftUI environment.
struct PlainTextEditor: View {
    @Binding var text: String
    var textStyle: UIFont.TextStyle = .body
    var monospaced: Bool = true
    var placeholder: String?
    var accessibilityIdentifier: String?

    var body: some View {
        PlainTextView(
            text: $text,
            textStyle: textStyle,
            monospaced: monospaced,
            accessibilityIdentifier: accessibilityIdentifier
        )
        .overlay(alignment: .topLeading) {
            if let placeholder, text.isEmpty {
                Text(placeholder)
                    .font(placeholderFont)
                    .foregroundColor(Color(uiColor: .placeholderText))
                    // Matches UITextView's default text container inset
                    // (8pt top) and line fragment padding (5pt).
                    .padding(.top, 8)
                    .padding(.leading, 5)
                    .allowsHitTesting(false)
                    .accessibilityHidden(true)
            }
        }
    }

    private var placeholderFont: Font {
        Font(PlainTextView.font(textStyle: textStyle, monospaced: monospaced))
    }
}

/// The single-line counterpart of ``PlainTextEditor``, for a one-line
/// JSON field (a `where` clause, a path or keys array): a `UITextField`
/// with smart quotes, smart dashes, smart insert/delete, autocorrection,
/// auto capitalization and spell checking all off.
///
/// The accessibility identifier and label are parameters so they land on
/// the `UITextField` itself. `.disabled(_:)` is honoured through the
/// SwiftUI environment.
struct PlainTextField: UIViewRepresentable {
    let placeholder: String
    @Binding var text: String
    var textStyle: UIFont.TextStyle = .footnote
    var monospaced: Bool = true
    var accessibilityIdentifier: String?
    var accessibilityLabel: String?
    /// `.roundedRect` matches a `TextField` styled with
    /// `RoundedBorderTextFieldStyle()`; `.none` suits a Form row.
    var borderStyle: UITextField.BorderStyle = .none

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    func makeUIView(context: Context) -> UITextField {
        let field = UITextField()
        field.delegate = context.coordinator
        field.placeholder = placeholder
        field.borderStyle = borderStyle
        field.font = PlainTextView.font(textStyle: textStyle, monospaced: monospaced)
        field.adjustsFontForContentSizeCategory = true
        field.smartQuotesType = .no
        field.smartDashesType = .no
        field.smartInsertDeleteType = .no
        field.autocorrectionType = .no
        field.autocapitalizationType = .none
        field.spellCheckingType = .no
        field.inlinePredictionType = .no
        field.returnKeyType = .done
        field.text = text
        field.addTarget(
            context.coordinator,
            action: #selector(Coordinator.editingChanged(_:)),
            for: .editingChanged
        )
        // Fill the row's width instead of sizing to the text.
        field.setContentHuggingPriority(.defaultLow, for: .horizontal)
        field.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return field
    }

    func updateUIView(_ uiView: UITextField, context: Context) {
        context.coordinator.text = $text
        // Assign only on a real change: resetting `text` moves the cursor.
        // Byte comparison: `String ==` treats canonically equivalent
        // sequences as equal and would swallow a byte-only change.
        if !(uiView.text ?? "").utf8.elementsEqual(text.utf8) {
            uiView.text = text
        }
        uiView.isEnabled = context.environment.isEnabled
        uiView.textColor = context.environment.isEnabled ? .label : .secondaryLabel
        uiView.accessibilityIdentifier = accessibilityIdentifier
        uiView.accessibilityLabel = accessibilityLabel
    }

    final class Coordinator: NSObject, UITextFieldDelegate {
        var text: Binding<String>

        init(text: Binding<String>) {
            self.text = text
        }

        @objc func editingChanged(_ field: UITextField) {
            let value = field.text ?? ""
            if !text.wrappedValue.utf8.elementsEqual(value.utf8) {
                text.wrappedValue = value
            }
        }

        func textFieldShouldReturn(_ textField: UITextField) -> Bool {
            textField.resignFirstResponder()
            return true
        }
    }
}

private struct PlainTextView: UIViewRepresentable {
    @Binding var text: String
    let textStyle: UIFont.TextStyle
    let monospaced: Bool
    let accessibilityIdentifier: String?

    static func font(textStyle: UIFont.TextStyle, monospaced: Bool) -> UIFont {
        guard monospaced else {
            return UIFont.preferredFont(forTextStyle: textStyle)
        }
        // Scale the monospaced face with Dynamic Type like the
        // `.font(.system(style, design: .monospaced))` editors it replaces.
        let base = UIFont.preferredFont(
            forTextStyle: textStyle,
            compatibleWith: UITraitCollection(preferredContentSizeCategory: .large)
        )
        let mono = UIFont.monospacedSystemFont(ofSize: base.pointSize, weight: .regular)
        return UIFontMetrics(forTextStyle: textStyle).scaledFont(for: mono)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    func makeUIView(context: Context) -> UITextView {
        let view = UITextView()
        view.delegate = context.coordinator
        view.backgroundColor = .clear
        view.font = Self.font(textStyle: textStyle, monospaced: monospaced)
        view.adjustsFontForContentSizeCategory = true
        view.smartQuotesType = .no
        view.smartDashesType = .no
        view.smartInsertDeleteType = .no
        view.autocorrectionType = .no
        view.autocapitalizationType = .none
        view.spellCheckingType = .no
        view.inlinePredictionType = .no
        view.text = text
        // Let SwiftUI pick the height (callers set a min height) instead
        // of the text's intrinsic size forcing the row taller or shorter.
        view.setContentCompressionResistancePriority(.defaultLow, for: .vertical)
        view.setContentHuggingPriority(.defaultLow, for: .vertical)
        return view
    }

    func updateUIView(_ uiView: UITextView, context: Context) {
        context.coordinator.text = $text
        // Assign only on a real change: resetting `text` moves the cursor
        // to the end on every keystroke.
        // Byte comparison: `String ==` treats canonically equivalent
        // sequences as equal and would swallow a byte-only change.
        if !(uiView.text ?? "").utf8.elementsEqual(text.utf8) {
            uiView.text = text
        }
        let isEnabled = context.environment.isEnabled
        uiView.isEditable = isEnabled
        uiView.textColor = isEnabled ? .label : .secondaryLabel
        uiView.accessibilityIdentifier = accessibilityIdentifier
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var text: Binding<String>

        init(text: Binding<String>) {
            self.text = text
        }

        func textViewDidChange(_ textView: UITextView) {
            if !text.wrappedValue.utf8.elementsEqual((textView.text ?? "").utf8) {
                text.wrappedValue = textView.text
            }
        }
    }
}
