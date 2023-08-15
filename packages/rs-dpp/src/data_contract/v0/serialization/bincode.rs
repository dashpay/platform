#[cfg(test)]
mod tests {
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use crate::data_contract::DataContract;
    use crate::identity::Identity;
    use crate::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion, PlatformDeserializableWithPotentialValidationFromVersionedStructure};
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::PlatformVersion;
    use crate::identity::accessors::IdentityGettersV0;

    #[test]
    #[cfg(feature = "random-identities")]
    fn data_contract_ser_de() {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(5, Some(5), platform_version)
            .expect("expected a random identity");
        let contract =
            get_data_contract_fixture(Some(identity.id()), platform_version.protocol_version)
                .data_contract_owned();
        let bytes = contract.serialize_with_platform_version(LATEST_PLATFORM_VERSION).expect("expected to serialize");
        let recovered_contract =
            DataContract::versioned_deserialize(&bytes, false, &platform_version)
                .expect("expected to deserialize state transition");
        assert_eq!(contract, recovered_contract);
    }
}
