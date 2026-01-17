# JsValue Type Audit for wasm-dpp2

This document lists all places where `JsValue` is used without strict typing.
Add your recommendations under each item.

---

## Category 2: Return Types Using JsValue

### State Transitions

- [ ] [state_transitions/batch/token_pricing_schedule.rs:71](src/state_transitions/batch/token_pricing_schedule.rs#L71) - `value() -> JsValue`
  - **Suggested:** `Price`
  - **Recommendation:**

- [ ] [state_transitions/batch/token_transitions/set_price_for_direct_purchase.rs:74](src/state_transitions/batch/token_transitions/set_price_for_direct_purchase.rs#L74) - `price() -> JsValue`
  - **Suggested:** `Price`
  - **Recommendation:**

### Token Configuration

- [ ] [tokens/configuration/action_taker.rs:71](src/tokens/configuration/action_taker.rs#L71) - `value() -> JsValue`
  - **Suggested:** `ActionTakerValue` (union type)
  - **Recommendation:**

- [ ] [tokens/configuration/authorized_action_takers.rs:74](src/tokens/configuration/authorized_action_takers.rs#L74) - `value() -> JsValue`
  - **Suggested:** `AuthorizedActionTakersValue`
  - **Recommendation:**

- [ ] [tokens/configuration/distribution_function.rs:243](src/tokens/configuration/distribution_function.rs#L243) - `function_value() -> JsValue`
  - **Suggested:** Define `DistributionFunctionValue` interface
  - **Recommendation:**

- [ ] [tokens/configuration/distribution_recipient.rs:62](src/tokens/configuration/distribution_recipient.rs#L62) - `value() -> JsValue`
  - **Suggested:** `DistributionRecipientValue` (union type)
  - **Recommendation:**

- [ ] [tokens/configuration/group.rs:139](src/tokens/configuration/group.rs#L139) - `to_json() -> WasmDppResult<JsValue>`
  - **Suggested:** Define `GroupJSON` interface
  - **Recommendation:**

---

## Notes

- Use `#[wasm_bindgen(unchecked_param_type = "...")]` for parameter type hints
- Use `#[wasm_bindgen(typescript_custom_section)]` to define TypeScript interfaces for return types
- Consider defining common type aliases in a shared location (e.g., `types.rs`)
