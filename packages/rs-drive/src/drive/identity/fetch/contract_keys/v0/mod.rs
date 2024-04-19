use std::collections::BTreeMap;
use grovedb::query_result_type::QueryResultType;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::identity::{Purpose};
use grovedb::TransactionArg;
use dpp::prelude::Identifier;
use platform_version::version::PlatformVersion;
use crate::error::query::QuerySyntaxError;

impl Drive {
    /// Proves identities with all its information from an identity ids.
    pub(super) fn get_identities_contract_keys_v0(
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

        let query = Self::identities_contract_keys_query(identity_ids, contract_id, &document_type_name, &purposes);

        let result = self.grove_get_path_query(
            &query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut drive_operations,
            &platform_version.drive,
        )?.0.to_path_key_elements();

        let mut partial_identities = BTreeMap::new();

        for (path, _, element) in result {
            if let Some(identity_id_bytes) = path.get(1) {
                let identity_id = Identifier::from_vec(identity_id_bytes.to_owned())?;
                let purpose= *path.last().expect("last path element is the purpose")
                    .first().ok_or(Error::Query(QuerySyntaxError::InvalidKeyParameter("invalid purpose".to_string())))?;
                let purpose = Purpose::try_from(purpose)
                    .map_err(|_| Error::Query(QuerySyntaxError::InvalidKeyParameter("invalid purpose".to_string())))?;

                let entry = partial_identities.entry(identity_id)
                    .or_insert(BTreeMap::new());

                entry.insert(purpose, element.into_item_bytes()?);
            }
        }


        Ok(partial_identities)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;

    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::GroveDb;
    use grovedb::QueryItem;
    use std::borrow::Borrow;
    use std::collections::BTreeMap;
    use std::ops::RangeFull;

    use crate::drive::Drive;

    use dpp::version::PlatformVersion;
}

