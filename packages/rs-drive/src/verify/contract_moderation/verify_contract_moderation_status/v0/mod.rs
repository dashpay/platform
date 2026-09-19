use crate::drive::contract::moderation::types::decode_until;
use crate::drive::contract::paths::{CONTRACT_BANLIST_KEY, CONTRACT_SUSPENSIONS_KEY};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::config::moderation::{ContractModerationList, ContractModerationStatus};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

impl Drive {
    pub(super) fn verify_contract_moderation_status_v0(
        proof: &[u8],
        contract_id: Identifier,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractModerationStatus), Error> {
        if lists.is_empty() {
            return Err(Error::Proof(ProofError::IncorrectProof(
                "a contract moderation status proof needs at least one list".to_string(),
            )));
        }
        let path_query = Self::contract_moderation_status_query(
            contract_id.to_buffer(),
            identity_id.to_buffer(),
            lists,
            &platform_version.drive.grove_version,
        )?;
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let mut status = ContractModerationStatus::default();
        for (path, key, element) in proved_key_values {
            let Some(element) = element else {
                continue;
            };
            if key.as_slice() != identity_id.as_slice() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "contract moderation status proof holds another identity's entry".to_string(),
                )));
            }
            let Element::Item(value, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "contract moderation entry is not an item".to_string(),
                )));
            };
            match path.last().map(Vec::as_slice) {
                Some([CONTRACT_BANLIST_KEY]) => status.banned = true,
                Some([CONTRACT_SUSPENSIONS_KEY]) => {
                    status.suspended_until = Some(decode_until(&value).map_err(|description| {
                        Error::Proof(ProofError::CorruptedProof(format!(
                            "contract suspension entry is malformed: {}",
                            description
                        )))
                    })?);
                }
                _ => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "contract moderation status proof holds an entry outside the lists"
                            .to_string(),
                    )));
                }
            }
        }

        Ok((root_hash, status))
    }
}
