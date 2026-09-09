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
}

#[cfg(test)]
mod tests {
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
            "$formatVersion": "1",
            "id": id,
            "ownerId": owner,
            "version": 1,
            "config": {
                "$formatVersion": "0",
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
                    "$formatVersion": "0",
                    "conventions": { "$formatVersion": "0", "decimals": 2, "localizations": {} },
                    "distributionRules": {
                        "$formatVersion": "0",
                        "perpetualDistribution": {
                            "$formatVersion": "0",
                            "distributionType": {
                                "$type": "blockBasedDistribution",
                                "interval": 10,
                                "function": { "$type": "stepwise", "0": 100, "10": 50 }
                            },
                            "distributionRecipient": {"$type": "contractOwner"}
                        },
                        "perpetualDistributionRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "contractOwner"},
                            "adminActionTakers": {"$type": "contractOwner"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "preProgrammedDistribution": null,
                        "preProgrammedDistributionRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "noOne"},
                            "adminActionTakers": {"$type": "noOne"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "newTokensDestinationIdentity": null,
                        "newTokensDestinationIdentityRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "contractOwner"},
                            "adminActionTakers": {"$type": "contractOwner"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "mintingAllowChoosingDestination": false,
                        "mintingAllowChoosingDestinationRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "contractOwner"},
                            "adminActionTakers": {"$type": "contractOwner"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "changeDirectPurchasePricingRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "noOne"},
                            "adminActionTakers": {"$type": "noOne"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        }
                    },
                    "marketplaceRules": {"$formatVersion": "0", "tradeMode": "NotTradeable"},
                    "manualMintingRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "manualBurningRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "freezeRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "unfreezeRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "destroyFrozenFundsRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "emergencyActionRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "directPurchaseRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "noOne"},
                        "adminActionTakers": {"$type": "noOne"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "mainControlGroup": null,
                    "mainControlGroupCanBeModified": {"$type": "noOne"}
                }
            }
        });

        let result = DataContract::from_json(contract, true, platform_version);
        assert!(
            result.is_ok(),
            "Stepwise with string keys should be accepted by from_json"
        );
    }

    #[test]
    fn from_json_accepts_preprogrammed_with_string_timestamp_keys() {
        let platform_version = PlatformVersion::latest();

        let owner = "HtQNfXBZJu3WnvjvCFJKgbvfgWYJxWxaFWy23TKoFjg9";
        let id = "BmKTJeLL3GfH8FxEx7SUbTog4eAKj8vJRDi97gYkxB9p";

        let contract = json!({
            "$formatVersion": "1",
            "id": id,
            "ownerId": owner,
            "version": 1,
            "config": {
                "$formatVersion": "0",
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
                    "$formatVersion": "0",
                    "conventions": { "$formatVersion": "0", "decimals": 2, "localizations": {} },
                    "distributionRules": {
                        "$formatVersion": "0",
                        "perpetualDistribution": null,
                        "perpetualDistributionRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "noOne"},
                            "adminActionTakers": {"$type": "noOne"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "preProgrammedDistribution": {
                            "$formatVersion": "0",
                            "distributions": {
                                "1735689600000": {
                                    "HtQNfXBZJu3WnvjvCFJKgbvfgWYJxWxaFWy23TKoFjg9": 1000
                                }
                            }
                        },
                        "preProgrammedDistributionRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "contractOwner"},
                            "adminActionTakers": {"$type": "contractOwner"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "newTokensDestinationIdentity": null,
                        "newTokensDestinationIdentityRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "contractOwner"},
                            "adminActionTakers": {"$type": "contractOwner"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "mintingAllowChoosingDestination": false,
                        "mintingAllowChoosingDestinationRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "contractOwner"},
                            "adminActionTakers": {"$type": "contractOwner"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        },
                        "changeDirectPurchasePricingRules": {
                            "$formatVersion": "0",
                            "authorizedToMakeChange": {"$type": "noOne"},
                            "adminActionTakers": {"$type": "noOne"},
                            "changingAuthorizedActionTakersToNoOneAllowed": false,
                            "changingAdminActionTakersToNoOneAllowed": false,
                            "selfChangingAdminActionTakersAllowed": false
                        }
                    },
                    "marketplaceRules": {"$formatVersion": "0", "tradeMode": "NotTradeable"},
                    "manualMintingRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "manualBurningRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "freezeRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "unfreezeRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "destroyFrozenFundsRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "emergencyActionRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "contractOwner"},
                        "adminActionTakers": {"$type": "contractOwner"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "directPurchaseRules": {
                        "$formatVersion": "0",
                        "authorizedToMakeChange": {"$type": "noOne"},
                        "adminActionTakers": {"$type": "noOne"},
                        "changingAuthorizedActionTakersToNoOneAllowed": false,
                        "changingAdminActionTakersToNoOneAllowed": false,
                        "selfChangingAdminActionTakersAllowed": false
                    },
                    "mainControlGroup": null,
                    "mainControlGroupCanBeModified": {"$type": "noOne"}
                }
            }
        });

        let result = DataContract::from_json(contract, true, platform_version);
        assert!(
            result.is_ok(),
            "PreProgrammed with string timestamp keys should be accepted by from_json"
        );
    }
}
