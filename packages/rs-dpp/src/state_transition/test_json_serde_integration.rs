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
}
