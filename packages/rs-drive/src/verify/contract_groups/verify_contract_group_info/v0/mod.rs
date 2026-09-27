use crate::drive::contract_groups::paths::CONTRACT_GROUP_INFO_KEY;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::contract_group::ContractGroupInfo;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableUntrusted;
use grovedb::{Element, GroveDb};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_contract_group_info_v0(
        proof: &[u8],
        contract_group_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<ContractGroupInfo>), Error> {
        let path_query = Self::contract_group_info_query(contract_group_id.to_buffer());
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let mut present = proved_key_values
            .into_iter()
            .filter_map(|(_path, key, element)| element.map(|element| (key, element)));
        let info = match present.next() {
            None => None,
            Some((key, Element::Item(bytes, _))) if key.as_slice() == CONTRACT_GROUP_INFO_KEY => {
                Some(
                    ContractGroupInfo::deserialize_from_bytes_untrusted(&bytes).map_err(|e| {
                        Error::Proof(ProofError::CorruptedProof(format!(
                            "contract group info does not decode: {}",
                            e
                        )))
                    })?,
                )
            }
            Some(_) => {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "contract group info proof holds an unexpected element".to_string(),
                )));
            }
        };
        if present.next().is_some() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "contract group info proof holds more than one element".to_string(),
            )));
        }

        Ok((root_hash, info))
    }
}
