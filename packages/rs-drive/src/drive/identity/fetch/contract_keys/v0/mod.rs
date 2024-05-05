use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::Purpose;
use dpp::prelude::Identifier;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches identities keys bound to specified contract
    pub(super) fn fetch_identities_contract_keys_v0(
        &self,
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, BTreeMap<Purpose, Vec<u8>>>, Error> {
        // let contract = &self.get_contract_with_fetch_info(
        //     *contract_id,
        //     false,
        //     transaction,
        //     platform_version,
        // )?.ok_or(Error::Query(QuerySyntaxError::DataContractNotFound("Contract not found for get_identities_contract_keys")))?.contract;

        // let (requires_encryption, requires_decryption) =
        // if let Some(document_type_name) = document_type_name {
        //     let document_type = contract.document_type_for_name(&document_type_name)?;
        //     (
        //         document_type.requires_identity_encryption_bounded_key(),
        //         document_type.requires_identity_decryption_bounded_key()
        //     )
        // } else {
        //     (
        //         contract.config().requires_identity_encryption_bounded_key(),
        //         contract.config().requires_identity_decryption_bounded_key(),
        //     )
        // };

        // let purpose_to_storage_requirements = purposes
        //     .into_iter()
        //     .map(|purpose| {
        //         let requirements = if purpose == Purpose::ENCRYPTION {
        //             requires_encryption.ok_or(Error::DataContract(DataContractError::KeyBoundsExpectedButNotPresent(
        //                 "expected an encryption key"
        //             )))
        //         } else if purpose == Purpose::DECRYPTION {
        //             requires_decryption.ok_or(Error::DataContract(DataContractError::KeyBoundsExpectedButNotPresent(
        //                 "expected a decryption key"
        //             )))
        //         } else {
        //             Err(
        //               Error::Query(
        //                   QuerySyntaxError::InvalidKeyParameter(
        //                       "expected an encryption or decryption key".to_string()
        //                   )
        //               )
        //             )
        //         }?;
        //
        //         Ok((purpose, requirements))
        //     })
        //     .collect::<Result<BTreeMap<Purpose, StorageKeyRequirements>, Error>>()?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let query = Self::identities_contract_keys_query(
            identity_ids,
            contract_id,
            &document_type_name,
            &purposes,
            Some((identity_ids.len() * purposes.len()) as u16),
        );

        let result = self
            .grove_get_path_query(
                &query,
                transaction,
                QueryResultType::QueryPathKeyElementTrioResultType,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .0
            .to_path_key_elements();

        let mut partial_identities = BTreeMap::new();

        for (path, _, element) in result {
            if let Some(identity_id_bytes) = path.get(1) {
                let identity_id = Identifier::from_vec(identity_id_bytes.to_owned())?;
                // We can use expect here because we have already shown that the path must have
                //  at least 2 sub parts as we get index 1
                let purpose_bytes = path.last().expect("last path element is the purpose");
                if purpose_bytes.len() != 1 {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "purpose for identifier {} at path {} is {}, should be 1 byte",
                        identity_id,
                        path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                        hex::encode(purpose_bytes)
                    ))));
                }

                let purpose_first_byte = purpose_bytes
                    .first()
                    .expect("we have already shown there is 1 byte");

                let purpose = Purpose::try_from(*purpose_first_byte).map_err(|e| {
                    Error::Drive(DriveError::CorruptedDriveState(format!(
                        "purpose for identifier {} at path {} has error : {}",
                        identity_id,
                        path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                        e
                    )))
                })?;

                let entry = partial_identities
                    .entry(identity_id)
                    .or_insert(BTreeMap::new());

                entry.insert(purpose, element.into_item_bytes()?);
            }
        }

        Ok(partial_identities)
    }
}
