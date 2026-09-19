use crate::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::data_contract::config::v2::DataContractConfigGettersV2;
use crate::data_contract::config::DataContractConfig;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

impl DataContractConfig {
    /// Generation 2 (protocol version 14): generation 1 plus the contract moderation rules.
    /// Which lists a contract keeps is decided when it is created and never changes: an update
    /// can not make an unmoderated contract moderated, turn a second list on, or turn a list
    /// off. Whoever writes documents under a contract knows, from its first version, whether
    /// and how they can be barred from it, and a list that is on may hold entries. Only the
    /// moderators may change.
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

        let lists = |config: &DataContractConfig| {
            config
                .moderation()
                .map(|moderation| (moderation.banlist, moderation.suspensions))
                .unwrap_or((false, false))
        };
        let (old_banlist, old_suspensions) = lists(self);
        let (new_banlist, new_suspensions) = lists(new_config);

        let refusal = if old_banlist && !new_banlist {
            Some("contract can not turn off its banlist once it keeps one")
        } else if old_suspensions && !new_suspensions {
            Some("contract can not turn off its suspension list once it keeps one")
        } else if !old_banlist && new_banlist {
            Some("contract can not start keeping a banlist after it is created")
        } else if !old_suspensions && new_suspensions {
            Some("contract can not start keeping a suspension list after it is created")
        } else {
            None
        };

        match refusal {
            Some(reason) => SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(contract_id, reason).into(),
            ),
            None => SimpleConsensusValidationResult::new(),
        }
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
    fn should_reject_enabling_moderation_on_update() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let old = DataContractConfig::V1(DataContractConfigV1::default());
        for new in [
            moderated(true, false),
            moderated(false, true),
            moderated(true, true),
        ] {
            let result = old.validate_update_v2(&new, contract_id, platform_version);
            assert!(!result.is_valid(), "{new:?}");
        }
    }

    #[test]
    fn should_reject_turning_a_second_list_on() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        for (old, new) in [
            (moderated(true, false), moderated(true, true)),
            (moderated(false, true), moderated(true, true)),
        ] {
            let result = old.validate_update_v2(&new, contract_id, platform_version);
            assert!(!result.is_valid(), "{old:?} -> {new:?}");
        }
    }

    #[test]
    fn should_allow_an_update_that_keeps_the_lists() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let unmoderated = DataContractConfig::V1(DataContractConfigV1::default());
        assert!(unmoderated
            .validate_update_v2(&unmoderated, contract_id, platform_version)
            .is_valid());
        for config in [moderated(true, false), moderated(true, true)] {
            let result = config.validate_update_v2(&config, contract_id, platform_version);
            assert!(result.is_valid(), "{:?}", result.errors);
        }
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
