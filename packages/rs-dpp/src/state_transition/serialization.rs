use crate::state_transition::StateTransition;
use crate::ProtocolError;
use bincode::Options;
use platform_value::Value;

impl StateTransition {
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .with_big_endian()
            .serialize(self)
            .map_err(|_| {
                ProtocolError::EncodingError(String::from(
                    "unable to serialize identity public key",
                ))
            })
    }

    pub fn serialized_size(&self) -> usize {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .with_big_endian()
            .serialized_size(self)
            .unwrap() as usize // this should not be able to error
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProtocolError> {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .with_big_endian()
            .deserialize(bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("unable to deserialize key {}", e)))
    }

    pub fn deserialize_many(
        raw_state_transitions: &Vec<Vec<u8>>,
    ) -> Result<Vec<Self>, ProtocolError> {
        raw_state_transitions
            .iter()
            .map(|raw_state_transition| Self::deserialize(raw_state_transition))
            .collect()
    }

    // pub fn deserialize_cbor(cbor_state_transition: &[u8]) -> Result<Self, ProtocolError> {
    //     let value : Value = ciborium::de::from_reader(cbor_state_transition).map_err(|e| ProtocolError::DecodingError(format!("{}", e)))?;
    //     Self::from_value(value)
    // }
    //
    // pub fn deserialize_many_cbor(
    //     cbor_state_transitions: &Vec<Vec<u8>>,
    // ) -> Result<Vec<Self>, ProtocolError> {
    //     cbor_state_transitions
    //         .iter()
    //         .map(|raw_state_transition| Self::deserialize_cbor(raw_state_transition))
    //         .collect()
    // }
}

// #[cfg(test)]
// mod tests {
//     use crate::document::document_transition::Action;
//     use crate::document::DocumentsBatchTransition;
//     use crate::state_transition::{StateTransition, StateTransitionConvert};
//     use crate::tests::fixtures::{get_data_contract_fixture, get_document_transitions_fixture, get_documents_fixture_with_owner_id_from_contract};
//
//     #[test]
//     fn cbor_ser_de() {
//         let data_contract = get_data_contract_fixture(None);
//         let documents =
//             get_documents_fixture_with_owner_id_from_contract(data_contract.clone()).unwrap();
//         let transitions = get_document_transitions_fixture([(Action::Create, documents)]);
//         let documents_batch_transition = DocumentsBatchTransition {
//             owner_id: data_contract.owner_id,
//             transitions,
//             ..Default::default()
//         };
//         let state_transition : StateTransition = documents_batch_transition.into();
//         let buffer = state_transition.to_cbor_buffer(false).expect("expected to get cbor");
//         let recovered_state_transition = StateTransition::deserialize_cbor(buffer.as_slice()).expect("expect to recover state transition");
//     }
// }
