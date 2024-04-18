use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
use dapi_grpc::platform::v0::get_identities_contract_keys_response::{
    get_identities_contract_keys_response_v0, GetIdentitiesContractKeysResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identity::Purpose;
use dpp::platform_value::Bytes32;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;

impl<C> Platform<C> {
    pub(super) fn query_identities_contract_keys_v0(
        &self,
        GetIdentitiesContractKeysRequestV0 {
            identities_ids,
            contract_id,
            document_type_name,
            purposes,
            prove,
        }: GetIdentitiesContractKeysRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesContractKeysResponseV0>, Error> {
        let identities_ids = check_validation_result_with_data!(identities_ids
            .into_iter()
            .map(|identity_id| {
                let identity_id = Bytes32::from_vec(identity_id)
                    .map(|bytes| bytes.0)
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?;

                Ok(identity_id)
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());

        let contract_id = check_validation_result_with_data!(Bytes32::from_vec(contract_id)
            .map(|bytes| bytes.0)
            .map_err(|_| {
                QueryError::InvalidArgument(
                    "contract_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let purposes = check_validation_result_with_data!(purposes
            .into_iter()
            .map(
                |purpose| Purpose::try_from(purpose as u8).map_err(|_| QueryError::Query(
                    QuerySyntaxError::InvalidKeyParameter(format!(
                        "purpose {} not recognized",
                        purpose
                    ))
                ))
            )
            .collect::<Result<Vec<Purpose>, QueryError>>());

        let response = if prove {
            let proof = self.drive.prove_identities_contract_keys(
                identities_ids.as_slice(),
                &contract_id,
                document_type_name,
                purposes,
                None,
                &platform_version.drive,
            )?;

            GetIdentitiesContractKeysResponseV0 {
                result: Some(get_identities_contract_keys_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            use get_identities_contract_keys_response_v0::IdentitiesKeys;
            use get_identities_contract_keys_response_v0::IdentityKeys;
            use get_identities_contract_keys_response_v0::PurposeKeys;
            use get_identities_contract_keys_response_v0::Result;

            let keys = self.drive.get_identities_contract_keys(
                identities_ids.as_slice(),
                &contract_id,
                document_type_name,
                purposes,
                None,
                &platform_version,
            )?;

            let identities_keys = keys
                .iter()
                .map(|(identity_id, keys)| {
                    let keys = keys
                        .iter()
                        .map(|(purpose, key)| PurposeKeys {
                            purpose: *purpose as i32,
                            keys_bytes: vec![key.to_owned()],
                        })
                        .collect::<Vec<PurposeKeys>>();

                    IdentityKeys {
                        identity_id: identity_id.to_vec(),
                        keys,
                    }
                })
                .collect::<Vec<IdentityKeys>>();

            GetIdentitiesContractKeysResponseV0 {
                result: Some(Result::IdentitiesKeys(IdentitiesKeys {
                    entries: identities_keys,
                })),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
    use dapi_grpc::platform::v0::get_identities_contract_keys_response::{
        get_identities_contract_keys_response_v0, GetIdentitiesContractKeysResponseV0,
    };
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::{Identity, IdentityV0, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::{DataContract, IdentityPublicKey};
    use dpp::serialization::{PlatformDeserializable, PlatformSerializableWithPlatformVersion};
    use drive::common::test_utils::identities::create_test_identity_with_rng;
    use drive::drive::batch::{DataContractOperationType, DriveOperation};
    use rand::prelude::StdRng;
    use rand::{Rng, SeedableRng};
    use std::borrow::Cow;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use arc_swap::Guard;
    use itertools::Itertools;
    use dapi_grpc::platform::v0::get_identities_contract_keys_response::get_identities_contract_keys_response_v0::IdentityKeys;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use platform_version::version::PlatformVersion;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;

    // TODO: test not found keys

    fn init_dashpay(platform: &TempPlatform<MockCoreRPCLike>, platform_version: &PlatformVersion) -> Guard<Arc<DataContract>> {
        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();

        let serialization = dashpay
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("expected to serialize data contract");

        platform
            .drive
            .apply_drive_operations(
                vec![DriveOperation::DataContractOperation(
                    DataContractOperationType::ApplyContractWithSerialization {
                        contract: Cow::Borrowed(dashpay.as_ref()),
                        serialized_contract: serialization,
                        storage_flags: None,
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to register dashpay contract");

        dashpay
    }

    #[test]
    fn test_identities_contract_keys() {
        let (platform, state, platform_version) = setup_platform();

        let dashpay = init_dashpay(&platform, platform_version);

        let (alice, bob) = {
            let mut rng = StdRng::seed_from_u64(10);

            let alice_id = rng.gen::<[u8; 32]>();
            let bob_id = rng.gen::<[u8; 32]>();

            // Create alice and bob identities
            let (mut alice, mut bob) = {
                let mut alice = create_test_identity_with_rng(
                    &platform.drive,
                    alice_id,
                    &mut rng,
                    None,
                    platform_version,
                ).expect("expected to create a test identity");

                let mut bob = create_test_identity_with_rng(
                    &platform.drive,
                    bob_id,
                    &mut rng,
                    None,
                    platform_version,
                ).expect("expected to create a test identity");

                (alice, bob)
            };

            // Add keys to alice and bob
            {
                let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());
                let db_transaction = platform.drive.grove.start_transaction();

                let (alice_enc_key, _) = IdentityPublicKey::random_key_with_known_attributes(
                    2,
                    &mut rng,
                    Purpose::ENCRYPTION,
                    SecurityLevel::MEDIUM,
                    KeyType::ECDSA_SECP256K1,
                    Some(ContractBounds::SingleContractDocumentType {
                        id: dashpay.id(),
                        document_type_name: "contactRequest".to_string(),
                    }),
                    platform_version,
                ).unwrap();

                alice.public_keys_mut().insert(alice_enc_key.id(), alice_enc_key.clone());

                let (bob_enc_key, _) = IdentityPublicKey::random_key_with_known_attributes(
                    2,
                    &mut rng,
                    Purpose::ENCRYPTION,
                    SecurityLevel::MEDIUM,
                    KeyType::ECDSA_SECP256K1,
                    Some(ContractBounds::SingleContractDocumentType {
                        id: dashpay.id(),
                        document_type_name: "contactRequest".to_string(),
                    }),
                    platform_version,
                ).unwrap();
                bob.public_keys_mut().insert(bob_enc_key.id(), bob_enc_key.clone());

                platform
                    .drive
                    .add_new_unique_keys_to_identity(
                        alice.id().to_buffer(),
                        vec![alice_enc_key],
                        &block,
                        true,
                        Some(&db_transaction),
                        platform_version,
                    )
                    .expect("expected to add a new key");

                platform
                    .drive
                    .add_new_unique_keys_to_identity(
                        bob.id().to_buffer(),
                        vec![bob_enc_key],
                        &block,
                        true,
                        Some(&db_transaction),
                        platform_version,
                    )
                    .expect("expected to add a new key");

                platform
                    .drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");
            }

            (alice, bob)
        };

        let request = GetIdentitiesContractKeysRequestV0 {
            identities_ids: vec![alice.id().to_vec(), bob.id().to_vec()],
            contract_id: dashpay.id().to_vec(),
            document_type_name: Some("contactRequest".to_string()),
            purposes: vec![Purpose::ENCRYPTION as i32, Purpose::DECRYPTION as i32],
            prove: false,
        };

        let result = platform
            .query_identities_contract_keys_v0(request, &state, platform_version)
            .expect("query failed");

        use get_identities_contract_keys_response_v0::Result;
        use get_identities_contract_keys_response_v0::IdentitiesKeys;

        let GetIdentitiesContractKeysResponseV0 {
           result,
            ..
        } = result.data.expect("expected data");

        let Result::IdentitiesKeys(IdentitiesKeys { entries: keys } ) = result.expect("expected result") else {
            panic!("expected IdentitiesKeys");
        };
        fn assert_keys(identity: &Identity, result_keys: &Vec<IdentityKeys>) {
            let identity_keys_result = result_keys.iter().find(|key| key.identity_id == identity.id().to_vec());
            assert_eq!(identity_keys_result.is_some(), true);
            let identity_keys_result = identity_keys_result.unwrap();

            let enc_keys_expected = identity.public_keys()
                .iter()
                .filter(|(_, key)| key.purpose() == Purpose::ENCRYPTION)
                .map(|(key_id, _)| *key_id)
                .sorted()
                .collect::<Vec<KeyID>>();

            let mut enc_keys_result = identity_keys_result
                .keys.iter().filter(|key| key.purpose == Purpose::ENCRYPTION as i32)
                .fold(vec![], |mut acc, keys| {
                    let keys = keys.keys_bytes
                        .iter()
                        .map(|key_bytes| IdentityPublicKey::deserialize_from_bytes(key_bytes.as_slice()).unwrap().id());
                    acc.extend(keys);
                    acc
                });
            enc_keys_result.sort();

            assert_eq!(enc_keys_result, enc_keys_expected);
        }

        assert_keys(&alice, &keys);
        assert_keys(&bob, &keys);
    }

    #[test]
    fn test_identities_contract_keys_proof() {
        let (platform, state, platform_version) = setup_platform();

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();

        let serialization = dashpay
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("expected to serialize data contract");

        platform
            .drive
            .apply_drive_operations(
                vec![DriveOperation::DataContractOperation(
                    DataContractOperationType::ApplyContractWithSerialization {
                        contract: Cow::Borrowed(dashpay.as_ref()),
                        serialized_contract: serialization,
                        storage_flags: None,
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected to register dashpay contract");

        let (alice, bob) = {
            let mut rng = StdRng::seed_from_u64(10);

            let alice_id = rng.gen::<[u8; 32]>();
            let bob_id = rng.gen::<[u8; 32]>();
            let alice = create_test_identity_with_rng(
                &platform.drive,
                alice_id,
                &mut rng,
                None,
                platform_version,
            )
            .expect("expected to create a test identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let (alice_enc_key, _) = IdentityPublicKey::random_key_with_known_attributes(
                2,
                &mut rng,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
                KeyType::ECDSA_SECP256K1,
                Some(ContractBounds::SingleContractDocumentType {
                    id: dashpay.id(),
                    document_type_name: "contactRequest".to_string(),
                }),
                platform_version,
            )
            .unwrap();

            let db_transaction = platform.drive.grove.start_transaction();

            platform
                .drive
                .add_new_unique_keys_to_identity(
                    alice.id().to_buffer(),
                    vec![alice_enc_key],
                    &block,
                    true,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to add a new key");
            platform
                .drive
                .grove
                .commit_transaction(db_transaction)
                .unwrap()
                .expect("expected to be able to commit a transaction");

            let bob = create_test_identity_with_rng(
                &platform.drive,
                bob_id,
                &mut rng,
                None,
                platform_version,
            )
            .expect("expected to create a test identity");
            (alice, bob)
        };

        let request = GetIdentitiesContractKeysRequestV0 {
            identities_ids: vec![alice.id().to_vec()],
            contract_id: dashpay.id().to_vec(),
            document_type_name: Some("contactRequest".to_string()),
            purposes: vec![Purpose::ENCRYPTION as i32, Purpose::DECRYPTION as i32],
            prove: true,
        };

        let result = platform
            .query_identities_contract_keys_v0(request, &state, platform_version)
            .expect("query failed");

        assert!(result.is_valid());
        assert!(matches!(
            result.data,
            Some(GetIdentitiesContractKeysResponseV0 {
                result: Some(get_identities_contract_keys_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_identities_contract_keys_absence_proof() {
        let (platform, state, version) = setup_platform();
        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();

        let request = GetIdentitiesContractKeysRequestV0 {
            identities_ids: vec![vec![0; 32]],
            contract_id: dashpay.id().to_vec(),
            document_type_name: Some("contactRequest".to_string()),
            purposes: vec![Purpose::ENCRYPTION as i32, Purpose::DECRYPTION as i32],
            prove: true,
        };

        let result = platform
            .query_identities_contract_keys_v0(request, &state, &version)
            .expect("query failed");

        assert!(result.is_valid());
        assert!(matches!(
            result.data,
            Some(GetIdentitiesContractKeysResponseV0 {
                result: Some(get_identities_contract_keys_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
