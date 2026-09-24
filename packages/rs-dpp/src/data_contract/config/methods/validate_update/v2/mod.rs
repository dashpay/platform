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
    /// moderators may change, and only between the merged kinds: an elected declaration is
    /// fixed at creation in every field, its interim included, and a contract neither enters
    /// nor leaves elected moderation by an update.
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
                .map(|moderation| {
                    (
                        moderation.banlist,
                        moderation.suspensions,
                        moderation.warnings,
                    )
                })
                .unwrap_or((false, false, false))
        };
        let (old_banlist, old_suspensions, old_warnings) = lists(self);
        let (new_banlist, new_suspensions, new_warnings) = lists(new_config);

        let refusal = if old_banlist && !new_banlist {
            Some("contract can not turn off its banlist once it keeps one")
        } else if old_suspensions && !new_suspensions {
            Some("contract can not turn off its suspension list once it keeps one")
        } else if old_warnings && !new_warnings {
            Some("contract can not turn off its warning list once it keeps one")
        } else if !old_banlist && new_banlist {
            Some("contract can not start keeping a banlist after it is created")
        } else if !old_suspensions && new_suspensions {
            Some("contract can not start keeping a suspension list after it is created")
        } else if !old_warnings && new_warnings {
            Some("contract can not start keeping a warning list after it is created")
        } else {
            None
        };

        if let Some(reason) = refusal {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(contract_id, reason).into(),
            );
        }

        let elected = |config: &DataContractConfig| {
            config
                .moderation()
                .and_then(|moderation| moderation.moderators.elected())
                .cloned()
        };
        let refusal = match (elected(self), elected(new_config)) {
            (Some(old), Some(new)) if old != new => {
                Some("contract can not change its elected moderation declaration")
            }
            (Some(_), None) => {
                Some("contract can not leave elected moderation once it declares it")
            }
            (None, Some(_)) => {
                Some("contract can not declare elected moderation after it is created")
            }
            _ => None,
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
    use crate::data_contract::config::moderation::{
        ContractModerationConfig, ContractModerators, ElectedModerators, InterimModerators,
        ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
    };
    use crate::data_contract::config::v1::DataContractConfigV1;
    use crate::data_contract::config::v2::DataContractConfigV2;
    use std::collections::{BTreeMap, BTreeSet};

    fn moderated(banlist: bool, suspensions: bool) -> DataContractConfig {
        DataContractConfig::V2(DataContractConfigV2 {
            moderation: Some(ContractModerationConfig {
                banlist,
                suspensions,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
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
    fn should_fix_the_warning_list_at_creation_like_the_others() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let with_warnings = |banlist: bool| {
            DataContractConfig::V2(DataContractConfigV2 {
                moderation: Some(ContractModerationConfig {
                    banlist,
                    suspensions: false,
                    warnings: true,
                    moderators: ContractModerators::ContractOwner,
                }),
                ..DataContractConfigV2::default()
            })
        };
        // On: refused. Off: refused. Kept: fine.
        assert!(!moderated(true, false)
            .validate_update_v2(&with_warnings(true), contract_id, platform_version)
            .is_valid());
        assert!(!with_warnings(true)
            .validate_update_v2(&moderated(true, false), contract_id, platform_version)
            .is_valid());
        let kept = with_warnings(false).validate_update_v2(
            &with_warnings(false),
            contract_id,
            platform_version,
        );
        assert!(kept.is_valid(), "{:?}", kept.errors);
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

    /// An elected declaration with both lists, `post` moderated, the owner in the interim
    fn elected(modify: impl FnOnce(&mut ElectedModerators)) -> DataContractConfig {
        let mut declaration = ElectedModerators {
            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            challenge_cool_down: Some(1_209_600),
            moderated_document_types: BTreeMap::from([(
                "post".to_string(),
                BTreeSet::from([ModerationAbility::Ban]),
            )]),
            interim: InterimModerators::ContractOwner,
            election_delay: None,
            max_added_moderators: 0,
            owner_protected: false,
        };
        modify(&mut declaration);
        DataContractConfig::V2(DataContractConfigV2 {
            moderation: Some(ContractModerationConfig {
                banlist: true,
                suspensions: true,
                warnings: false,
                moderators: ContractModerators::Elected(Box::new(declaration)),
            }),
            ..DataContractConfigV2::default()
        })
    }

    #[test]
    fn should_freeze_every_field_of_an_elected_declaration() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let unchanged = elected(|_| {});
        let kept = unchanged.validate_update_v2(&unchanged, contract_id, platform_version);
        assert!(kept.is_valid(), "{:?}", kept.errors);

        type Change = (&'static str, fn(&mut ElectedModerators));
        let changes: [Change; 10] = [
            ("join window", |d| d.join_window += 1),
            ("vote window", |d| d.vote_window += 1),
            ("seat made permanent", |d| d.challenge_cool_down = None),
            ("challenge cool-down", |d| {
                d.challenge_cool_down = d.challenge_cool_down.map(|cool_down| cool_down + 1)
            }),
            ("election delay", |d| d.election_delay = Some(1)),
            ("added moderators", |d| d.max_added_moderators = 1),
            ("moderated set", |d| {
                d.moderated_document_types
                    .insert("like".to_string(), BTreeSet::from([ModerationAbility::Ban]));
            }),
            ("abilities", |d| {
                d.moderated_document_types
                    .get_mut("post")
                    .expect("post is moderated")
                    .insert(ModerationAbility::Suspend);
            }),
            ("interim", |d| {
                d.interim =
                    InterimModerators::AppointedModerators([Identifier::new([5u8; 32])].into())
            }),
            ("owner flag", |d| d.owner_protected = true),
        ];
        for (what, change) in changes {
            let result =
                unchanged.validate_update_v2(&elected(change), contract_id, platform_version);
            assert!(
                !result.is_valid(),
                "expected a changed {what} to be refused"
            );
            assert!(
                format!("{:?}", result.errors).contains("can not change its elected"),
                "{what}: {:?}",
                result.errors
            );
        }

        // Nor can a seat that can not be contested be opened to challenges
        let permanent = elected(|d| d.challenge_cool_down = None);
        let kept = permanent.validate_update_v2(&permanent, contract_id, platform_version);
        assert!(kept.is_valid(), "{:?}", kept.errors);
        let opened = permanent.validate_update_v2(&unchanged, contract_id, platform_version);
        assert!(
            format!("{:?}", opened.errors).contains("can not change its elected"),
            "{:?}",
            opened.errors
        );
    }

    #[test]
    fn should_refuse_entering_or_leaving_elected_moderation() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let entering = moderated(true, true).validate_update_v2(
            &elected(|_| {}),
            contract_id,
            platform_version,
        );
        assert!(format!("{:?}", entering.errors).contains("after it is created"));
        let leaving = elected(|_| {}).validate_update_v2(
            &moderated(true, true),
            contract_id,
            platform_version,
        );
        assert!(format!("{:?}", leaving.errors).contains("once it declares it"));
        // Leaving moderation altogether is caught by the lists first; the declaration would be too.
        let dropped = elected(|_| {}).validate_update_v2(
            &DataContractConfig::V1(DataContractConfigV1::default()),
            contract_id,
            platform_version,
        );
        assert!(!dropped.is_valid());
    }

    #[test]
    fn should_allow_changing_the_moderators() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let new = DataContractConfig::V2(DataContractConfigV2 {
            moderation: Some(ContractModerationConfig {
                banlist: true,
                suspensions: false,
                moderators: ContractModerators::AppointedModerators(
                    [Identifier::new([5u8; 32])].into_iter().collect(),
                ),
                warnings: false,
            }),
            ..DataContractConfigV2::default()
        });
        let result = moderated(true, false).validate_update_v2(&new, contract_id, platform_version);
        assert!(result.is_valid(), "{:?}", result.errors);
    }
}
