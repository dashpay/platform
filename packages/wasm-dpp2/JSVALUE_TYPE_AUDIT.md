# JsValue Type Audit for wasm-dpp2

This document lists all places where `JsValue` is used without strict typing.
Add your recommendations under each item.

---

## Category 2: Return Types Using JsValue

### Token Configuration

- [ ] [tokens/configuration/distribution_recipient.rs:62](src/tokens/configuration/distribution_recipient.rs#L62) - `value() -> JsValue`
  - **Suggested:** `DistributionRecipientValue` (union type)
  - **Recommendation:**

---

## Notes

- Use `#[wasm_bindgen(unchecked_param_type = "...")]` for parameter type hints
- Use `#[wasm_bindgen(typescript_custom_section)]` to define TypeScript interfaces for return types
- Consider defining common type aliases in a shared location (e.g., `types.rs`)
