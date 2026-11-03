//! A document type paying its contract's own non-transferable token (protocol version 14): it
//! may burn the token, but paying the contract owner would move the token to another identity.
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::codes::ErrorWithCode;
use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::document_type::accessors::DocumentTypeV1Getters;
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use platform_value::platform_value;

fn token(transferable: bool) -> BTreeMap<TokenContractPosition, TokenConfiguration> {
    let mut configuration =
        TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
    configuration.set_transferable(transferable);
    BTreeMap::from([(0, configuration)])
}

/// A `post` whose create costs one of the contract's token at position 0, with `effect` when
/// given (the default pays the contract owner).
fn schema(effect: Option<u64>) -> Value {
    let mut cost = platform_value!({ "tokenPosition": 0_u64, "amount": 10_u64 });
    if let Some(effect) = effect {
        cost.as_map_mut()
            .expect("cost map")
            .push((Value::Text("effect".to_string()), Value::U64(effect)));
    }
    platform_value!({
        "type": "object",
        "properties": {
            "text": {"type": "string", "position": 0, "maxLength": 40_u32},
        },
        "tokenCost": { "create": cost },
        "additionalProperties": false,
    })
}

fn parse(
    schema: Value,
    token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
) -> Result<DocumentType, ProtocolError> {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "post",
        schema,
        None,
        token_configurations,
        &config,
        true,
        &mut vec![],
        platform_version,
    )
}

#[test]
fn should_refuse_paying_the_contract_owner_its_own_non_transferable_token_with_error_10280() {
    for effect in [None, Some(0)] {
        let error = parse(schema(effect), &token(false))
            .expect_err("paying the owner a non-transferable token is refused");
        let ProtocolError::ConsensusError(error) = error else {
            panic!("expected a consensus error, got {error:?}");
        };
        let ConsensusError::BasicError(BasicError::NonTransferableTokenPaymentMustBurnError(inner)) =
            error.as_ref()
        else {
            panic!("expected NonTransferableTokenPaymentMustBurnError, got {error:?}");
        };
        assert_eq!(inner.token_contract_position(), 0);
        assert_eq!(inner.action(), "create");
        assert_eq!(error.code(), 10280);
    }
}

#[test]
fn should_accept_burning_its_own_non_transferable_token() {
    let document_type = parse(schema(Some(1)), &token(false)).expect("a burn cost is accepted");
    let cost = document_type
        .document_creation_token_cost()
        .expect("the create cost is kept");
    assert_eq!(cost.effect, DocumentActionTokenEffect::BurnToken);
}

#[test]
fn should_still_let_a_transferable_token_pay_the_contract_owner() {
    let document_type = parse(schema(None), &token(true)).expect("a transferable token pays");
    let cost = document_type
        .document_creation_token_cost()
        .expect("the create cost is kept");
    assert_eq!(
        cost.effect,
        DocumentActionTokenEffect::TransferTokenToContractOwner
    );
}
