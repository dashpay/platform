use crate::drive::contract::fee_pots::types::{decode_last_claim, ContractFeePots};
use crate::drive::contract::paths::{
    contract_fee_pots_key, contract_last_fee_claim_key, CONTRACT_OTHER_KEY,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

impl Drive {
    pub(super) fn verify_contract_fee_pots_v0(
        proof: &[u8],
        contract_id: Identifier,
        pots: &[ContractFeePot],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractFeePots), Error> {
        if pots.is_empty() {
            return Err(Error::Proof(ProofError::IncorrectProof(
                "a contract fee pots proof needs at least one pot".to_string(),
            )));
        }
        let path_query = Self::contract_fee_pots_query(
            contract_id.to_buffer(),
            pots,
            &platform_version.drive.grove_version,
        )?;
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut fee_pots = ContractFeePots::default();
        for (path, key, element) in proved_key_values {
            let Some(element) = element else {
                continue;
            };
            let in_other_tree = path.last().map(Vec::as_slice) == Some(&[CONTRACT_OTHER_KEY]);
            if in_other_tree {
                // A last claim: `[64, contract id, 2] -> 32 | 96`
                let Some(pot) = pots
                    .iter()
                    .find(|pot| key.as_slice() == contract_last_fee_claim_key(**pot))
                else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "contract fee pots proof holds an entry outside the pots asked for"
                            .to_string(),
                    )));
                };
                let Element::Item(value, _) = element else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "contract fee pot last claim is not an item".to_string(),
                    )));
                };
                fee_pots.pot_mut(*pot).last_claim =
                    Some(decode_last_claim(&value).map_err(|description| {
                        Error::Proof(ProofError::CorruptedProof(format!(
                            "contract fee pot last claim is malformed: {}",
                            description
                        )))
                    })?);
            } else {
                // A pot: `[40, 64 | 192] -> contract id`
                if key.as_slice() != contract_id.as_slice() {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "contract fee pots proof holds another contract's pot".to_string(),
                    )));
                }
                let Some(pot) = pots.iter().find(|pot| {
                    path.last().map(Vec::as_slice) == Some(contract_fee_pots_key(**pot))
                }) else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "contract fee pots proof holds a pot outside the pots asked for"
                            .to_string(),
                    )));
                };
                let Element::SumItem(credits, _) = element else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "contract fee pot is not a sum item".to_string(),
                    )));
                };
                fee_pots.pot_mut(*pot).credits = u64::try_from(credits).map_err(|_| {
                    Error::Proof(ProofError::CorruptedProof(
                        "contract fee pot holds a negative amount".to_string(),
                    ))
                })?;
            }
        }

        Ok((root_hash, fee_pots))
    }
}
