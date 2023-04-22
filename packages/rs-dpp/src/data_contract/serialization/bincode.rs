use crate::data_contract::{DataContract, DataContractInner};
use crate::ProtocolError;
use bincode::config;
use std::convert::TryInto;

impl DataContract {
    pub fn serialize_consume(self) -> Result<Vec<u8>, ProtocolError> {
        let config = config::standard().with_big_endian().with_no_limit();
        let inner: DataContractInner = self.into();
        bincode::encode_to_vec(inner, config).map_err(|e| {
            ProtocolError::EncodingError(format!("unable to serialize data contract {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::DataContract;
    use crate::identity::Identity;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_value::Bytes32;

    #[test]
    fn data_contract_ser_de() {
        let identity = Identity::random_identity(5, Some(5));
        let mut contract = get_data_contract_fixture(Some(identity.id));
        contract.entropy = Bytes32::default();
        let bytes = contract.serialize().expect("expected to serialize");
        let recovered_contract =
            DataContract::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(contract, recovered_contract);
    }
}
