const contract = {
  $format_version: '1',
  id: 'Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i',
  config: {
    $format_version: '1',
    canBeDeleted: false,
    readonly: false,
    keepsHistory: false,
    documentsKeepHistoryContractDefault: false,
    documentsMutableContractDefault: true,
    documentsCanBeDeletedContractDefault: true,
    requiresIdentityEncryptionBoundedKey: null,
    requiresIdentityDecryptionBoundedKey: null,
    sizedIntegerTypes: true
  },
  version: 1,
  ownerId: '7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC',
  schemaDefs: null,
  documentSchemas: {
    card: {
      type: 'object',
      documentsMutable: false,
      canBeDeleted: true,
      transferable: 1,
      tradeMode: 1,
      creationRestrictionMode: 1,
      properties: {
        name: {
          type: 'string',
          description: 'Name of the card',
          minLength: 0,
          maxLength: 63,
          position: 0
        },
        description: {
          type: 'string',
          description: 'Description of the card',
          minLength: 0,
          maxLength: 256,
          position: 1
        },
        attack: {
          type: 'integer',
          description: 'Attack power of the card',
          position: 2
        },
        defense: {
          type: 'integer',
          description: 'Defense level of the card',
          position: 3
        }
      },
      indices: [
        {
          name: 'owner',
          properties: [
            {
              $ownerId: 'asc'
            }
          ]
        },
        {
          name: 'attack',
          properties: [
            {
              attack: 'asc'
            }
          ]
        },
        {
          name: 'defense',
          properties: [
            {
              defense: 'asc'
            }
          ]
        }
      ],
      required: [
        'name',
        'attack',
        'defense'
      ],
      additionalProperties: false
    }
  },
  createdAt: 1756237255149,
  updatedAt: null,
  createdAtBlockHeight: 174305,
  updatedAtBlockHeight: null,
  createdAtEpoch: 9690,
  updatedAtEpoch: null,
  groups: {
    '0': {
      $format_version: '0',
      members: {
        '7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC': 1,
        'HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG': 1
      },
      required_power: 2
    }
  },
  tokens: {
    '0': {
      $format_version: '0',
      conventions: {
        $format_version: '0',
        localizations: {
          en: {
            $format_version: '0',
            shouldCapitalize: true,
            singularForm: 'stt-99',
            pluralForm: 'stt-99s'
          }
        },
        decimals: 0
      },
      conventionsChangeRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      baseSupply: 100,
      maxSupply: null,
      keepsHistory: {
        $format_version: '0',
        keepsTransferHistory: true,
        keepsFreezingHistory: true,
        keepsMintingHistory: true,
        keepsBurningHistory: true,
        keepsDirectPricingHistory: true,
        keepsDirectPurchaseHistory: true
      },
      startAsPaused: false,
      allowTransferToFrozenBalance: true,
      maxSupplyChangeRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      distributionRules: {
        $format_version: '0',
        perpetualDistribution: {
          $format_version: '0',
          distributionType: {
            BlockBasedDistribution: {
              interval: 100,
              function: {
                FixedAmount: {
                  amount: 1
                }
              }
            }
          },
          distributionRecipient: 'ContractOwner'
        },
        perpetualDistributionRules: {
          V0: {
            authorized_to_make_change: 'ContractOwner',
            admin_action_takers: 'ContractOwner',
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true
          }
        },
        preProgrammedDistribution: null,
        newTokensDestinationIdentity: '7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC',
        newTokensDestinationIdentityRules: {
          V0: {
            authorized_to_make_change: 'ContractOwner',
            admin_action_takers: 'ContractOwner',
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true
          }
        },
        mintingAllowChoosingDestination: false,
        mintingAllowChoosingDestinationRules: {
          V0: {
            authorized_to_make_change: 'ContractOwner',
            admin_action_takers: 'ContractOwner',
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true
          }
        },
        changeDirectPurchasePricingRules: {
          V0: {
            authorized_to_make_change: 'ContractOwner',
            admin_action_takers: 'ContractOwner',
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true
          }
        }
      },
      marketplaceRules: {
        $format_version: '0',
        tradeMode: 'NotTradeable',
        tradeModeChangeRules: {
          V0: {
            authorized_to_make_change: 'ContractOwner',
            admin_action_takers: 'ContractOwner',
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true
          }
        }
      },
      manualMintingRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      manualBurningRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      freezeRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      unfreezeRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      destroyFrozenFundsRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      emergencyActionRules: {
        V0: {
          authorized_to_make_change: 'ContractOwner',
          admin_action_takers: 'ContractOwner',
          changing_authorized_action_takers_to_no_one_allowed: true,
          changing_admin_action_takers_to_no_one_allowed: true,
          self_changing_admin_action_takers_allowed: true
        }
      },
      mainControlGroup: 0,
      mainControlGroupCanBeModified: 'ContractOwner',
      description: null
    }
  },
  keywords: [
    'stt-99'
  ],
  description: null
}

export default contract;
