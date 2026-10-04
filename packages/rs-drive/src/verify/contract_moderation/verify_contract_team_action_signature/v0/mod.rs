use crate::drive::contract::paths::CONTRACT_TEAM_CLOSED_ACTIONS_KEY;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

impl Drive {
    pub(super) fn verify_contract_team_action_signature_v0(
        proof: &[u8],
        contract_id: Identifier,
        action_id: Identifier,
        signer_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroupActionStatus), Error> {
        let path_query = Self::contract_team_action_signer_query(
            contract_id.to_buffer(),
            action_id.to_buffer(),
            signer_id.to_buffer(),
        );
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut status = None;
        for (path, _key, element) in proved_key_values {
            let Some(element) = element else {
                continue;
            };
            if !matches!(element, Element::SumItem(..)) {
                return Err(Error::Proof(ProofError::CorruptedProof(format!(
                    "the approval of team action {} by {} is not a sum item",
                    action_id, signer_id
                ))));
            }
            if status.is_some() {
                return Err(Error::Proof(ProofError::CorruptedProof(format!(
                    "the approval of team action {} by {} is both active and closed",
                    action_id, signer_id
                ))));
            }
            // `[64, contract id, 2, 24, M or X, action id, S]`
            status = Some(match path.get(4).map(Vec::as_slice) {
                Some(key) if key == CONTRACT_TEAM_CLOSED_ACTIONS_KEY => {
                    GroupActionStatus::ActionClosed
                }
                _ => GroupActionStatus::ActionActive,
            });
        }
        let status = status.ok_or_else(|| {
            Error::Proof(ProofError::IncorrectProof(format!(
                "proof did not show the approval of team action {} by {}",
                action_id, signer_id
            )))
        })?;
        Ok((root_hash, status))
    }
}
