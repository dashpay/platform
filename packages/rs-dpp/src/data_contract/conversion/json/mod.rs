mod v0;
pub use v0::*;

use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DataContract, DataContractV1};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

impl DataContractJsonConversionMethodsV0 for DataContract {
    fn from_json(
        json_value: JsonValue,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(
                DataContractV0::from_json(json_value, full_validation, platform_version)?.into(),
            ),
            1 => Ok(
                DataContractV1::from_json(json_value, full_validation, platform_version)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_json".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    fn to_json(&self, platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_json(platform_version),
            DataContract::V1(v1) => v1.to_json(platform_version),
        }
    }

    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_validating_json(platform_version),
            DataContract::V1(v1) => v1.to_validating_json(platform_version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use crate::prelude::DataContract;
    use crate::version::PlatformVersion;
    use serde_json::json;

    #[test]
    fn from_json_accepts_stepwise_with_string_keys() {
        let platform_version = PlatformVersion::latest();

        let owner = "HtQNfXBZJu3WnvjvCFJKgbvfgWYJxWxaFWy23TKoFjg9";
        let id = "BmKTJeLL3GfH8FxEx7SUbTog4eAKj8vJRDi97gYkxB9p";

        let contract = json!({
            "$format_version": "1",
            "id": id,
            "ownerId": owner,
            "version": 1,
            "config": {
                "$format_version": "0",
                "canBeDeleted": false,
                "readonly": false,
                "keepsHistory": false,
                "documentsKeepHistoryContractDefault": false,
                "documentsMutableContractDefault": true,
                "documentsCanBeDeletedContractDefault": false,
                "requiresIdentityEncryptionBoundedKey": null,
                "requiresIdentityDecryptionBoundedKey": null
            },
            "documentSchemas": {},
            "tokens": {
                "0": {
                    "$format_version": "0",
                    "conventions": { "$format_version": "0", "decimals": 2, "localizations": {} },
                    "distributionRules": {
                        "$format_version": "0",
                        "perpetualDistribution": {
                            "$format_version": "0",
                            "distributionType": {
                                "BlockBasedDistribution": {
                                    "interval": 10,
                                    "function": {
                                        "Stepwise": { "0": 100, "10": 50 }
                                    }
                                }
                            },
                            "distributionRecipient": "ContractOwner"
                        },
                        "perpetualDistributionRules": {"V0": {
                            "authorized_to_make_change": "ContractOwner",
                            "admin_action_takers": "ContractOwner",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "preProgrammedDistribution": null,
                        "preProgrammedDistributionRules": {"V0": {
                            "authorized_to_make_change": "NoOne",
                            "admin_action_takers": "NoOne",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "newTokensDestinationIdentity": null,
                        "newTokensDestinationIdentityRules": {"V0": {
                            "authorized_to_make_change": "ContractOwner",
                            "admin_action_takers": "ContractOwner",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "mintingAllowChoosingDestination": false,
                        "mintingAllowChoosingDestinationRules": {"V0": {
                            "authorized_to_make_change": "ContractOwner",
                            "admin_action_takers": "ContractOwner",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "changeDirectPurchasePricingRules": {"V0": {
                            "authorized_to_make_change": "NoOne",
                            "admin_action_takers": "NoOne",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }}
                    },
                    "marketplaceRules": {"$format_version": "0", "tradeMode": "NotTradeable"},
                    "manualMintingRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "manualBurningRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "freezeRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "unfreezeRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "destroyFrozenFundsRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "emergencyActionRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "directPurchaseRules": {"V0": {
                        "authorized_to_make_change": "NoOne",
                        "admin_action_takers": "NoOne",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "mainControlGroup": null,
                    "mainControlGroupCanBeModified": "NoOne"
                }
            }
        });

        let result = DataContract::from_json(contract, true, &platform_version);
        assert!(result.is_ok(), "Stepwise with string keys should be accepted by from_json");
    }

    #[test]
    fn from_json_accepts_preprogrammed_with_string_timestamp_keys() {
        let platform_version = PlatformVersion::latest();

        let owner = "HtQNfXBZJu3WnvjvCFJKgbvfgWYJxWxaFWy23TKoFjg9";
        let id = "BmKTJeLL3GfH8FxEx7SUbTog4eAKj8vJRDi97gYkxB9p";

        let contract = json!({
            "$format_version": "1",
            "id": id,
            "ownerId": owner,
            "version": 1,
            "config": {
                "$format_version": "0",
                "canBeDeleted": false,
                "readonly": false,
                "keepsHistory": false,
                "documentsKeepHistoryContractDefault": false,
                "documentsMutableContractDefault": true,
                "documentsCanBeDeletedContractDefault": false,
                "requiresIdentityEncryptionBoundedKey": null,
                "requiresIdentityDecryptionBoundedKey": null
            },
            "documentSchemas": {},
            "tokens": {
                "0": {
                    "$format_version": "0",
                    "conventions": { "$format_version": "0", "decimals": 2, "localizations": {} },
                    "distributionRules": {
                        "$format_version": "0",
                        "perpetualDistribution": null,
                        "perpetualDistributionRules": {"V0": {
                            "authorized_to_make_change": "NoOne",
                            "admin_action_takers": "NoOne",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "preProgrammedDistribution": {
                            "$format_version": "0",
                            "distributions": {
                                "1735689600000": {
                                    "HtQNfXBZJu3WnvjvCFJKgbvfgWYJxWxaFWy23TKoFjg9": 1000
                                }
                            }
                        },
                        "preProgrammedDistributionRules": {"V0": {
                            "authorized_to_make_change": "ContractOwner",
                            "admin_action_takers": "ContractOwner",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "newTokensDestinationIdentity": null,
                        "newTokensDestinationIdentityRules": {"V0": {
                            "authorized_to_make_change": "ContractOwner",
                            "admin_action_takers": "ContractOwner",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "mintingAllowChoosingDestination": false,
                        "mintingAllowChoosingDestinationRules": {"V0": {
                            "authorized_to_make_change": "ContractOwner",
                            "admin_action_takers": "ContractOwner",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }},
                        "changeDirectPurchasePricingRules": {"V0": {
                            "authorized_to_make_change": "NoOne",
                            "admin_action_takers": "NoOne",
                            "changing_authorized_action_takers_to_no_one_allowed": false,
                            "changing_admin_action_takers_to_no_one_allowed": false,
                            "self_changing_admin_action_takers_allowed": false
                        }}
                    },
                    "marketplaceRules": {"$format_version": "0", "tradeMode": "NotTradeable"},
                    "manualMintingRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "manualBurningRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "freezeRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "unfreezeRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "destroyFrozenFundsRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "emergencyActionRules": {"V0": {
                        "authorized_to_make_change": "ContractOwner",
                        "admin_action_takers": "ContractOwner",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "directPurchaseRules": {"V0": {
                        "authorized_to_make_change": "NoOne",
                        "admin_action_takers": "NoOne",
                        "changing_authorized_action_takers_to_no_one_allowed": false,
                        "changing_admin_action_takers_to_no_one_allowed": false,
                        "self_changing_admin_action_takers_allowed": false
                    }},
                    "mainControlGroup": null,
                    "mainControlGroupCanBeModified": "NoOne"
                }
            }
        });

        let result = DataContract::from_json(contract, true, &platform_version);
        assert!(result.is_ok(), "PreProgrammed with string timestamp keys should be accepted by from_json");
    }
}
