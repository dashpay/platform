use std::collections::BTreeMap;

use grovedb_commitment_tree::ProvingKey;

use crate::address_funds::AddressFundsFeeStrategy;
use crate::address_funds::{OrchardAddress, PlatformAddress};
use crate::fee::Credits;
use crate::identity::signer::Signer;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{build_output_only_bundle, serialize_authorized_bundle};

/// Builds a Shield state transition (transparent platform addresses -> shielded pool).
///
/// Constructs an output-only Orchard bundle (no spends), proves it, signs the
/// transparent input witnesses, and returns a ready-to-broadcast `StateTransition`.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield
/// - `inputs` - Platform address inputs with their nonces and balances
/// - `fee_strategy` - How to deduct fees from the transparent inputs
/// - `signer` - Signs each input address witness (ECDSA)
/// - `user_fee_increase` - Fee multiplier (0 = 100% base fee)
/// - `proving_key` - Halo 2 proving key (cache with `OnceLock` — ~30s to build)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `platform_version` - Protocol version
#[allow(clippy::too_many_arguments)]
pub fn build_shield_transition<S: Signer<PlatformAddress>>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    fee_strategy: AddressFundsFeeStrategy,
    signer: &S,
    user_fee_increase: UserFeeIncrease,
    proving_key: &ProvingKey,
    memo: [u8; 36],
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    if fee_strategy.is_empty() {
        return Err(ProtocolError::Generic(
            "fee_strategy must have at least one step".to_string(),
        ));
    }

    let bundle = build_output_only_bundle(recipient, shield_amount, memo, proving_key)?;
    let sb = serialize_authorized_bundle(&bundle);

    ShieldTransition::try_from_bundle_with_signer(
        inputs,
        sb.actions,
        sb.value_balance,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        fee_strategy,
        signer,
        user_fee_increase,
        platform_version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressWitness;
    use crate::address_funds::AddressFundsFeeStrategyStep;
    use crate::shielded::builder::test_helpers::{proving_key, test_orchard_address};
    use platform_value::BinaryData;

    /// A dummy signer that produces a fake 65-byte signature.
    /// Only used to test the builder pipeline — the signature is not validated here.
    #[derive(Debug)]
    struct DummySigner;

    impl Signer<PlatformAddress> for DummySigner {
        fn sign(&self, _key: &PlatformAddress, _data: &[u8]) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![0u8; 65]))
        }

        fn sign_create_witness(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Ok(AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0u8; 65]),
            })
        }

        fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
            true
        }
    }

    #[test]
    fn test_build_shield_empty_fee_strategy() {
        let recipient = test_orchard_address();
        let platform_version = PlatformVersion::latest();
        let pk = proving_key();

        let result = build_shield_transition(
            &recipient,
            1000,
            BTreeMap::new(),
            vec![], // empty fee strategy
            &DummySigner,
            0,
            pk,
            [0u8; 36],
            platform_version,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("fee_strategy must have at least one step"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_build_shield_transition_valid() {
        let recipient = test_orchard_address();
        let platform_version = PlatformVersion::latest();
        let pk = proving_key();

        // Create a P2PKH address as input
        let input_address = PlatformAddress::P2pkh([1u8; 20]);
        let mut inputs = BTreeMap::new();
        inputs.insert(input_address, (0u32, 100_000u64));

        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let result = build_shield_transition(
            &recipient,
            50_000,
            inputs,
            fee_strategy,
            &DummySigner,
            0,
            pk,
            [0u8; 36],
            platform_version,
        );

        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
        match result.unwrap() {
            StateTransition::Shield(_) => {} // correct variant
            other => panic!("expected Shield variant, got {:?}", other),
        }
    }
}
