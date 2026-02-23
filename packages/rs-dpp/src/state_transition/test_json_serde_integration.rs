#[cfg(all(
    test,
    feature = "state-transition-serde-conversion",
    feature = "state-transition-json-conversion"
))]
mod test_serde_json_serialization {
    use crate::address_funds::PlatformAddress;
    use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
    use crate::state_transition::{
        JsonStateTransitionSerializationOptions, StateTransition, StateTransitionJsonConvert,
    };
    use std::collections::BTreeMap;

    /// Recursively search a JSON value for a given key name.
    fn find_key_recursive<'a>(
        val: &'a serde_json::Value,
        key: &str,
    ) -> Option<&'a serde_json::Value> {
        match val {
            serde_json::Value::Object(map) => {
                if let Some(v) = map.get(key) {
                    return Some(v);
                }
                for v in map.values() {
                    if let Some(found) = find_key_recursive(v, key) {
                        return Some(found);
                    }
                }
                None
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    if let Some(found) = find_key_recursive(v, key) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Helper: build an AddressFundsTransfer StateTransition with known addresses.
    fn make_address_funds_transfer() -> StateTransition {
        let mut inputs = BTreeMap::new();
        let input_address = PlatformAddress::P2pkh([1u8; 20]);
        inputs.insert(input_address, (1u32, 1000u64));

        let mut outputs = BTreeMap::new();
        let output_address = PlatformAddress::P2pkh([2u8; 20]);
        outputs.insert(output_address, 900u64);

        let v0 = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            user_fee_increase: 0,
            fee_strategy: Default::default(),
            input_witnesses: vec![],
        };

        StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(v0))
    }

    #[test]
    fn test_variant_to_json() {
        // Test if the variant's to_json works (as claimed in the original issue)
        let mut inputs = BTreeMap::new();
        let input_address = PlatformAddress::P2pkh([1u8; 20]);
        inputs.insert(input_address, (1u32, 1000u64));

        let mut outputs = BTreeMap::new();
        let output_address = PlatformAddress::P2pkh([2u8; 20]);
        outputs.insert(output_address, 900u64);

        let transition_v0 = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            user_fee_increase: 0,
            fee_strategy: Default::default(),
            input_witnesses: vec![],
        };

        let transition = AddressFundsTransferTransition::V0(transition_v0);

        println!("\n=== Testing variant.to_json() ===");
        match transition.to_json(JsonStateTransitionSerializationOptions::default()) {
            Ok(json_value) => {
                println!("SUCCESS: variant to_json() worked!");
                println!(
                    "JSON:\n{}",
                    serde_json::to_string_pretty(&json_value).unwrap()
                );
            }
            Err(e) => {
                println!("FAILED: variant to_json() error: {:?}", e);
                panic!("Expected variant to_json to work but it failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_state_transition_enum_to_json() {
        // Test if the top-level StateTransition enum's to_json works
        let mut inputs = BTreeMap::new();
        let input_address = PlatformAddress::P2pkh([1u8; 20]);
        inputs.insert(input_address, (1u32, 1000u64));

        let mut outputs = BTreeMap::new();
        let output_address = PlatformAddress::P2pkh([2u8; 20]);
        outputs.insert(output_address, 900u64);

        let transition_v0 = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            user_fee_increase: 0,
            fee_strategy: Default::default(),
            input_witnesses: vec![],
        };

        let transition = AddressFundsTransferTransition::V0(transition_v0);
        let state_transition = StateTransition::AddressFundsTransfer(transition.clone());

        // Test 1: Try serde_json::to_string_pretty (this should fail with "key must be a string")
        println!("\n=== Testing serde_json::to_string_pretty ===");
        match serde_json::to_string_pretty(&state_transition) {
            Ok(json_str) => {
                println!("SUCCESS: serde serialization worked!");
                println!("JSON:\n{}", json_str);
            }
            Err(e) => {
                println!("FAILED: serde serialization error: {}", e);
                println!(
                    "This is expected - PlatformAddress as BTreeMap key doesn't work with serde"
                );
            }
        }

        // Test 2: Try our custom to_json() method on the enum
        println!("\n=== Testing StateTransition enum to_json() method ===");
        match state_transition.to_json(JsonStateTransitionSerializationOptions::default()) {
            Ok(json_value) => {
                println!("SUCCESS: enum to_json() worked!");
                println!(
                    "JSON:\n{}",
                    serde_json::to_string_pretty(&json_value).unwrap()
                );
            }
            Err(e) => {
                println!("FAILED: enum to_json() error: {:?}", e);
                panic!("Expected enum to_json to work but it failed: {:?}", e);
            }
        }
    }

    /// Given an AddressFundsTransfer variant with BTreeMap<PlatformAddress, _> fields,
    /// When calling to_json() on the inner variant directly,
    /// Then serialization succeeds without "key must be a string" errors.
    #[test]
    fn variant_to_json_succeeds() {
        let st = make_address_funds_transfer();
        if let StateTransition::AddressFundsTransfer(ref inner) = st {
            inner
                .to_json(JsonStateTransitionSerializationOptions::default())
                .expect("variant to_json should succeed");
        }
    }

    /// Given an AddressFundsTransfer wrapped in the top-level StateTransition enum,
    /// When calling to_json() on the enum and serde_json::to_string_pretty(),
    /// Then both serialization paths succeed and PlatformAddress map keys are
    ///   rendered as human-readable strings (e.g. "P2PKH(hex...)"), not complex objects.
    #[test]
    fn enum_to_json_succeeds_and_serde_roundtrips() {
        let st = make_address_funds_transfer();

        // StateTransition::to_json() must succeed (the whole point of the fix).
        let json = st
            .to_json(JsonStateTransitionSerializationOptions::default())
            .expect("StateTransition::to_json should succeed");

        // serde_json must also work now (custom map-key serializer).
        let serde_json_str = serde_json::to_string_pretty(&st)
            .expect("serde_json::to_string_pretty should succeed after custom serde fix");

        // The to_json output must contain string keys for PlatformAddress maps.
        let inputs = json.get("inputs").expect("JSON must have 'inputs' field");
        assert!(
            inputs.is_object(),
            "inputs must be a JSON object (string keys)"
        );
        for key in inputs.as_object().unwrap().keys() {
            assert!(
                key.starts_with("P2PKH(") || key.starts_with("P2SH("),
                "map key must be a PlatformAddress Display string, got: {key}"
            );
        }

        // The serde output must also have string keys for PlatformAddress maps.
        // The exact nesting depends on enum repr; find "inputs" anywhere in the tree.
        let serde_val: serde_json::Value =
            serde_json::from_str(&serde_json_str).expect("re-parse must succeed");
        let inputs_field = find_key_recursive(&serde_val, "inputs")
            .expect("serde JSON must contain an 'inputs' field somewhere");
        assert!(
            inputs_field.is_object(),
            "serde inputs must be a JSON object with string keys"
        );
        for key in inputs_field.as_object().unwrap().keys() {
            assert!(
                key.starts_with("P2PKH(") || key.starts_with("P2SH("),
                "serde map key must be a PlatformAddress Display string, got: {key}"
            );
        }
    }
}
