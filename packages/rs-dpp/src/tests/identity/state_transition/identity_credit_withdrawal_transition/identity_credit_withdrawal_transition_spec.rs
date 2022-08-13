#[cfg(test)]
use crate::{
    identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
    prelude::Identifier,
    tests::fixtures::{
        identity_credit_withdrawal_transition_fixture_json,
        identity_credit_withdrawal_transition_fixture_raw_object,
    },
};

mod deserialization {
    use super::*;

    #[test]
    fn from_raw_object() {
        let raw_object = identity_credit_withdrawal_transition_fixture_raw_object();
        let state_transition =
            IdentityCreditWithdrawalTransition::from_raw_object(raw_object).unwrap();

        assert_eq!(
            state_transition.identity_id,
            Identifier::from_bytes(&vec![1; 32]).unwrap()
        );

        assert_eq!(state_transition.output, vec![0; 20]);
        assert_eq!(state_transition.signature, vec![0; 65]);
    }

    #[test]
    fn from_json() {
        let json_value = identity_credit_withdrawal_transition_fixture_json();
        let state_transition = IdentityCreditWithdrawalTransition::from_json(json_value).unwrap();

        assert_eq!(
            state_transition.identity_id,
            Identifier::from_bytes(&vec![1; 32]).unwrap()
        );

        assert_eq!(state_transition.output, vec![0; 20]);
        assert_eq!(state_transition.signature, vec![0; 65]);
    }
}

#[cfg(test)]
mod serialization {
    use crate::state_transition::StateTransitionConvert;

    use super::*;

    #[test]
    fn to_raw_object() {
        let raw_object = identity_credit_withdrawal_transition_fixture_raw_object();
        let state_transition =
            IdentityCreditWithdrawalTransition::from_raw_object(raw_object).unwrap();

        assert_eq!(
            identity_credit_withdrawal_transition_fixture_raw_object(),
            state_transition.to_object(false).unwrap()
        );
    }

    #[test]
    fn to_json() {
        let json_value = identity_credit_withdrawal_transition_fixture_json();
        let state_transition = IdentityCreditWithdrawalTransition::from_json(json_value).unwrap();

        assert_eq!(
            identity_credit_withdrawal_transition_fixture_json(),
            state_transition.to_json().unwrap()
        );
    }
}
