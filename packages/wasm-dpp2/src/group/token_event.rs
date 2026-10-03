use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use dpp::tokens::token_event::TokenEvent;
use wasm_bindgen::prelude::wasm_bindgen;

// The custom section is a wasm-target artifact, so only the attribute is gated
// on it. The constant itself stays visible to Rust, which lets the field names
// this doc promises be checked against the ones the serializer emits.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(typescript_custom_section))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const TS_TYPES: &str = r#"
/**
 * TokenEvent serialized as a plain object.
 *
 * Custom Serialize emits an internally-tagged flat shape: `$type:` is the
 * variant discriminator, positional tuple fields are mapped to named JSON
 * keys per variant. No `data` wrapper.
 *
 * Common per-variant payloads:
 *   - Mint:    { $type: "mint",    amount, recipient, publicNote }
 *   - Burn:    { $type: "burn",    amount, burnFromIdentifier, publicNote }
 *   - Freeze:  { $type: "freeze",  frozenIdentifier, publicNote }
 *   - Unfreeze:{ $type: "unfreeze",frozenIdentifier, publicNote }
 *   - DestroyFrozenFunds:        { $type, frozenIdentifier, amount, publicNote }
 *   - Transfer:{ $type, recipient, publicNote, sharedEncryptedNote,
 *                privateEncryptedNote, amount }
 *   - Claim:   { $type, distributionType, amount, publicNote }
 *   - EmergencyAction:           { $type, action, publicNote }
 *   - ConfigUpdate:              { $type, configurationChange, publicNote }
 *   - ChangePriceForDirectPurchase: { $type, pricingSchedule, publicNote }
 *   - DirectPurchase: { $type, amount, credits }
 *   - Shield:  { $type: "shield", amount }
 *   - Unshield:{ $type: "unshield", recipient, amount }
 *   - ShieldedTransfer: { $type: "shieldedTransfer" }
 *   - MintToPool: { $type: "mintToPool", amount, actionsDigest, publicNote }
 *   - BurnFromPool: { $type: "burnFromPool", amount, actionsDigest, publicNote }
 *   - ClaimToPool: { $type: "claimToPool", amount }
 *   - DirectPurchaseToPool: { $type: "directPurchaseToPool", amount, credits }
 *
 * `amount`/`credits` are routed through json_safe_u64 — small numbers, JS
 * BigInt-safe stringification above 2^53. Identifier fields use base58 in
 * JSON, Uint8Array in toObject().
 */
export interface TokenEventObject {
    $type: string;
    [field: string]: unknown;
}

/**
 * TokenEvent serialized as JSON. Same shape as TokenEventObject with
 * Identifier fields rendered as base58 strings.
 */
export interface TokenEventJSON {
    $type: string;
    [field: string]: unknown;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenEventObject")]
    pub type TokenEventObjectJs;

    #[wasm_bindgen(typescript_type = "TokenEventJSON")]
    pub type TokenEventJSONJs;
}

/// TypeScript enum for TokenEvent variants
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenEventVariant {
    Mint = 0,
    Burn = 1,
    Freeze = 2,
    Unfreeze = 3,
    DestroyFrozenFunds = 4,
    Transfer = 5,
    Claim = 6,
    EmergencyAction = 7,
    ConfigUpdate = 8,
    ChangePriceForDirectPurchase = 9,
    DirectPurchase = 10,
    Shield = 11,
    Unshield = 12,
    ShieldedTransfer = 13,
    MintToPool = 14,
    BurnFromPool = 15,
    ClaimToPool = 16,
    DirectPurchaseToPool = 17,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "TokenEvent")]
pub struct TokenEventWasm(pub(crate) TokenEvent);

impl From<TokenEvent> for TokenEventWasm {
    fn from(event: TokenEvent) -> Self {
        TokenEventWasm(event)
    }
}

impl From<TokenEventWasm> for TokenEvent {
    fn from(event: TokenEventWasm) -> Self {
        event.0
    }
}

#[wasm_bindgen(js_class = TokenEvent)]
impl TokenEventWasm {
    #[wasm_bindgen(getter = "variant")]
    pub fn variant(&self) -> TokenEventVariant {
        match &self.0 {
            TokenEvent::Mint(..) => TokenEventVariant::Mint,
            TokenEvent::Burn(..) => TokenEventVariant::Burn,
            TokenEvent::Freeze(..) => TokenEventVariant::Freeze,
            TokenEvent::Unfreeze(..) => TokenEventVariant::Unfreeze,
            TokenEvent::DestroyFrozenFunds(..) => TokenEventVariant::DestroyFrozenFunds,
            TokenEvent::Transfer(..) => TokenEventVariant::Transfer,
            TokenEvent::Claim(..) => TokenEventVariant::Claim,
            TokenEvent::EmergencyAction(..) => TokenEventVariant::EmergencyAction,
            TokenEvent::ConfigUpdate(..) => TokenEventVariant::ConfigUpdate,
            TokenEvent::ChangePriceForDirectPurchase(..) => {
                TokenEventVariant::ChangePriceForDirectPurchase
            }
            TokenEvent::DirectPurchase(..) => TokenEventVariant::DirectPurchase,
            TokenEvent::Shield(..) => TokenEventVariant::Shield,
            TokenEvent::Unshield(..) => TokenEventVariant::Unshield,
            TokenEvent::ShieldedTransfer => TokenEventVariant::ShieldedTransfer,
            TokenEvent::MintToPool(..) => TokenEventVariant::MintToPool,
            TokenEvent::BurnFromPool(..) => TokenEventVariant::BurnFromPool,
            TokenEvent::ClaimToPool(..) => TokenEventVariant::ClaimToPool,
            TokenEvent::DirectPurchaseToPool(..) => TokenEventVariant::DirectPurchaseToPool,
        }
    }
}

impl_wasm_conversions_inner!(
    TokenEventWasm,
    TokenEvent,
    TokenEvent,
    TokenEventObjectJs,
    TokenEventJSONJs
);
impl_wasm_type_info!(TokenEventWasm, TokenEvent);

#[cfg(test)]
mod tests {
    use super::TS_TYPES;
    use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionTypeWithResolvedRecipient;
    use dpp::identifier::Identifier;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use dpp::tokens::token_event::TokenEvent;

    /// `TokenEventObject` is an index signature, so the per-variant field names
    /// a TypeScript consumer works from live only in the doc block's prose and
    /// nothing checks them. This reads the field list the doc promises for each
    /// variant back out of it and compares that with the keys `TokenEvent`'s
    /// `Serialize` actually emits. A name that exists only in the doc — a
    /// property a JS consumer would read as `undefined` — fails here, and so
    /// does renaming a serialized field without updating the doc.
    #[test]
    fn the_documented_field_names_are_the_ones_the_serializer_emits() {
        // A comment block: entries wrap across lines, each carrying a leading
        // `*`. Flatten it so one entry is one contiguous span of text.
        let doc = TS_TYPES.replace(['\n', '*'], " ");

        let mut disagreements = Vec::new();
        for event in every_variant() {
            let serialized = serde_json::to_value(&event).expect("a TokenEvent serializes");
            let object = serialized
                .as_object()
                .expect("a TokenEvent serializes as a map keyed by field name");
            let mut emitted: Vec<String> = object
                .keys()
                .filter(|key| key.as_str() != "$type")
                .cloned()
                .collect();
            emitted.sort();

            let variant = variant_name(&event);
            let mut documented = documented_fields(&doc, variant);
            documented.sort();

            if documented != emitted {
                disagreements.push(format!(
                    "  {variant}: doc says {documented:?}, serializer emits {emitted:?}"
                ));
            }
        }

        assert!(
            disagreements.is_empty(),
            "the TypeScript doc block promises field names `TokenEvent`'s Serialize \
             does not emit:\n{}",
            disagreements.join("\n")
        );
    }

    /// The payload field names the doc block promises for one variant, read out
    /// of its `- Variant: { $type, field, ... }` entry. `$type` is the
    /// discriminator rather than a payload field, so it is dropped.
    fn documented_fields(doc: &str, variant: &str) -> Vec<String> {
        let entry = doc
            .split_once(&format!("- {variant}:"))
            .unwrap_or_else(|| panic!("the doc block has an entry for `{variant}`"))
            .1;
        let fields = entry
            .split_once('{')
            .expect("the entry opens its field list")
            .1
            .split_once('}')
            .expect("the entry closes its field list")
            .0;
        fields
            .split(',')
            .map(str::trim)
            .filter(|field| !field.is_empty() && !field.starts_with("$type"))
            .map(str::to_string)
            .collect()
    }

    /// The label the doc block uses for a variant's entry. Exhaustive, so a new
    /// variant stops compiling until it is documented and added below.
    fn variant_name(event: &TokenEvent) -> &'static str {
        match event {
            TokenEvent::Mint(..) => "Mint",
            TokenEvent::Burn(..) => "Burn",
            TokenEvent::Freeze(..) => "Freeze",
            TokenEvent::Unfreeze(..) => "Unfreeze",
            TokenEvent::DestroyFrozenFunds(..) => "DestroyFrozenFunds",
            TokenEvent::Transfer(..) => "Transfer",
            TokenEvent::Claim(..) => "Claim",
            TokenEvent::EmergencyAction(..) => "EmergencyAction",
            TokenEvent::ConfigUpdate(..) => "ConfigUpdate",
            TokenEvent::ChangePriceForDirectPurchase(..) => "ChangePriceForDirectPurchase",
            TokenEvent::DirectPurchase(..) => "DirectPurchase",
            TokenEvent::Shield(..) => "Shield",
            TokenEvent::Unshield(..) => "Unshield",
            TokenEvent::ShieldedTransfer => "ShieldedTransfer",
            TokenEvent::MintToPool(..) => "MintToPool",
            TokenEvent::BurnFromPool(..) => "BurnFromPool",
            TokenEvent::ClaimToPool(..) => "ClaimToPool",
            TokenEvent::DirectPurchaseToPool(..) => "DirectPurchaseToPool",
        }
    }

    /// One instance of every variant. Which keys the serializer emits depends
    /// on the variant alone, never on the payload, so the values are stand-ins.
    fn every_variant() -> Vec<TokenEvent> {
        let id = Identifier::new([1; 32]);
        vec![
            TokenEvent::Mint(1, id, None),
            TokenEvent::Burn(1, id, None),
            TokenEvent::Freeze(id, None),
            TokenEvent::Unfreeze(id, None),
            TokenEvent::DestroyFrozenFunds(id, 1, None),
            TokenEvent::Transfer(id, None, None, None, 1),
            TokenEvent::Claim(
                TokenDistributionTypeWithResolvedRecipient::PreProgrammed(id),
                1,
                None,
            ),
            TokenEvent::EmergencyAction(TokenEmergencyAction::Pause, None),
            TokenEvent::ConfigUpdate(
                TokenConfigurationChangeItem::TokenConfigurationNoChange,
                None,
            ),
            TokenEvent::ChangePriceForDirectPurchase(None, None),
            TokenEvent::DirectPurchase(1, 1),
            TokenEvent::Shield(1),
            TokenEvent::Unshield(id, 1),
            TokenEvent::ShieldedTransfer,
            TokenEvent::MintToPool(1, id, None),
            TokenEvent::BurnFromPool(1, id, None),
            TokenEvent::ClaimToPool(1),
            TokenEvent::DirectPurchaseToPool(1, 1),
        ]
    }
}
