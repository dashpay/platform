#[cfg(test)]
mod tests {
    use crate::data_contract::DataContract;
    use crate::identity::Identity;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::version::PlatformVersion;

    #[test]
    fn data_contract_ser_de() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let contract = get_data_contract_fixture(Some(identity.id)).data_contract;
        let bytes = contract.serialize().expect("expected to serialize");
        let recovered_contract =
            DataContract::deserialize(&bytes).expect("expected to deserialize state transition");
        assert_eq!(contract, recovered_contract);
    }
}
