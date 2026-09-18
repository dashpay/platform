use crate::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::data_contract::config::v2::DataContractConfigGettersV2;
use crate::data_contract::config::DataContractConfig;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

impl DataContractConfig {
    /// Generation 2 (protocol version 14): generation 1 plus the contract moderation rules. A
    /// moderation list may be turned on by an update, and the moderators may change, but a
    /// list that is on can never be turned off: its tree may hold entries.
    #[inline(always)]
    pub(super) fn validate_update_v2(
        &self,
        new_config: &DataContractConfig,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let v1_result = self.validate_update_v1(new_config, contract_id, platform_version);
        if !v1_result.is_valid() {
            return v1_result;
        }

        let Some(old_moderation) = self.moderation() else {
            return SimpleConsensusValidationResult::new();
        };

        let (new_banlist, new_suspensions) = new_config
            .moderation()
            .map(|moderation| (moderation.banlist, moderation.suspensions))
            .unwrap_or((false, false));

        if old_moderation.banlist && !new_banlist {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not turn off its banlist once it keeps one",
                )
                .into(),
            );
        }

        if old_moderation.suspensions && !new_suspensions {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not turn off its suspension list once it keeps one",
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use crate::data_contract::config::v1::DataContractConfigV1;
    use crate::data_contract::config::v2::DataContractConfigV2;

    fn moderated(banlist: bool, suspensions: bool) -> DataContractConfig {
        DataContractConfig::V2(DataContractConfigV2 {
            moderation: Some(ContractModerationConfig {
                banlist,
                suspensions,
                moderators: ContractModerators::ContractOwner,
            }),
            ..DataContractConfigV2::default()
        })
    }

    #[test]
    fn should_allow_enabling_moderation_on_update() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let old = DataContractConfig::V1(DataContractConfigV1::default());
        let result = old.validate_update_v2(&moderated(true, false), contract_id, platform_version);
        assert!(result.is_valid(), "{:?}", result.errors);
        let result = moderated(true, false).validate_update_v2(
            &moderated(true, true),
            contract_id,
            platform_version,
        );
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_turning_a_list_off() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let result = moderated(true, true).validate_update_v2(
            &moderated(true, false),
            contract_id,
            platform_version,
        );
        assert!(!result.is_valid());
        let result = moderated(true, false).validate_update_v2(
            &DataContractConfig::V1(DataContractConfigV1::default()),
            contract_id,
            platform_version,
        );
        assert!(!result.is_valid());
    }

    #[test]
    fn should_allow_changing_the_moderators() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let new = DataContractConfig::V2(DataContractConfigV2 {
            moderation: Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::OwnerAndIdentities(
                    [Identifier::new([5u8; 32])].into_iter().collect(),
                ),
            }),
            ..DataContractConfigV2::default()
        });
        let result = moderated(true, false).validate_update_v2(&new, contract_id, platform_version);
        assert!(result.is_valid(), "{:?}", result.errors);
    }
}
