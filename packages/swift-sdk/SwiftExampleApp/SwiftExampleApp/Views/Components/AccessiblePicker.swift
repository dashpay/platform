// AccessiblePicker.swift
// SwiftExampleApp
//
// TESTABILITY helper — NOT product logic. SwiftUI's default menu-style
// `Picker` (the Form default, or an explicit `.pickerStyle(.menu)` /
// `MenuPickerStyle()`) renders its option list as a UIKit context-menu
// popover. idb's accessibility tree only sees that popover as a single
// element labelled "Dismiss context menu" — the individual option rows
// are NOT exposed, so UI automation can neither query nor tap a specific
// option.
//
// These modifiers swap the presentation to a row-exposing style without
// touching any selection / binding / onChange behaviour:
//
//   - `.accessibleFormPicker(_:)` — for pickers inside a `Form` / `List`.
//     Applies `.pickerStyle(.navigationLink)`, which pushes a real
//     `List` of selectable rows onto the navigation stack. idb traverses
//     those rows directly.
//   - `.accessibleInlinePicker(_:)` — for pickers NOT inside a Form
//     (navigationLink renders poorly there). Applies
//     `.pickerStyle(.inline)`, which lays the rows out in place as part
//     of the view hierarchy idb can see.
//
// Both also stamp the picker container with an `accessibilityIdentifier`
// so the picker element itself is addressable. Each *option row* should
// also set its own `.accessibilityIdentifier(...)` at the call site —
// but how that surfaces to idb differs by style (verified on-device):
//
//   - navigationLink: the pushed list renders each option as a distinct
//     row and the per-row identifiers DO surface — e.g. an option row
//     reports `id=topup.fundingSource.account.0`. Prefer this for the
//     Form pickers; it gives stable, unique, tappable row ids.
//   - inline: SwiftUI rebuilds the options as its own selection rows and
//     does NOT surface their per-row identifiers — every row instead
//     inherits the container identifier. So inline option rows are
//     addressed by their visible label (`AXLabel`), not by a per-row id.
//     The per-row `.accessibilityIdentifier(...)` calls are kept anyway:
//     they document intent and take effect if the picker is ever moved
//     into a Form/navigationLink context. Because inline rows share the
//     container id, give each `.accessibleInlinePicker(_:)` a container
//     id distinct from any sibling so a label+container query stays
//     unambiguous.

import SwiftUI

extension View {
    /// Form/List picker presentation that exposes option rows to the
    /// accessibility tree. Use on a `Picker` that lives inside a `Form`
    /// or `List`. Sets `.pickerStyle(.navigationLink)` and tags the
    /// picker container with `identifier`.
    ///
    /// Does not alter selection, bindings, or `onChange` — purely a
    /// presentation + accessibility change.
    func accessibleFormPicker(_ identifier: String) -> some View {
        self
            .pickerStyle(.navigationLink)
            .accessibilityIdentifier(identifier)
    }

    /// Inline picker presentation for pickers that are NOT inside a
    /// `Form`/`List`, where `.navigationLink` renders poorly. Sets
    /// `.pickerStyle(.inline)` so the rows are laid out in place (and
    /// thus exposed to idb) and tags the picker container with
    /// `identifier`.
    ///
    /// Does not alter selection, bindings, or `onChange` — purely a
    /// presentation + accessibility change.
    func accessibleInlinePicker(_ identifier: String) -> some View {
        self
            .pickerStyle(.inline)
            .accessibilityIdentifier(identifier)
    }
}
