/**
 * Test data extracted from existing WASM SDK test parameters
 * Based on update_inputs.py and existing test files
 */

// Load environment variables for sensitive test data
require('dotenv').config({ path: require('path').join(__dirname, '../.env') });

const testData = {
  // Known testnet identity IDs for testing (from WASM SDK docs and tests)
  identityIds: {
    testnet: [
      "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",  // Used in docs.html and multiple test files
      "5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX"   // Used in docs.html
    ],
    mainnet: [
      // Add mainnet IDs when available
    ]
  },

  // Data contract IDs (from WASM SDK files and update_inputs.py)
  dataContracts: {
    testnet: {
      dpns: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",  // Used in index.html as DPNS_CONTRACT_ID
      dashpay: "ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A",
      sample: "HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM",
      tokenPricing: "H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy",  // Used in test-token-pricing-complete.html
      tokenContract: "EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta", // Used in update_inputs.py
      postCreate: "9nzpvjVSStUrhkEs3eNHw2JYpcNoLh1MjmqW45QiyjSa"      // Used in test_post_create.html
    },
    mainnet: {
      // Add mainnet contract IDs when available
    }
  },

  // Public key hashes for testing
  publicKeyHashes: {
    testnet: [
      "b7e904ce25ed97594e72f7af0e66f298031c1754",
      "518038dc858461bcee90478fd994bba8057b7531"
    ]
  },

  // Token IDs for testing
  tokenIds: {
    testnet: [
      "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv",
      "HEv1AYWQfwCffXQgmuzmzyzUo9untRTmVr67n4e4PSWa", // Used in docs.html (last claim)
      "4tyvbA2ZGFLvjXLnJRCacSoMbFfpmBwGRrAZsVwnfYri", // Identity 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk frozen
    ]
  },

  // ProTx hashes for epoch testing
  proTxHashes: {
    testnet: [
      "143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113"
    ]
  },

  // Document IDs
  documentIds: {
    testnet: {
      dpnsDomain: "7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r"
    }
  },

  // Specialized balance IDs
  specializedBalanceIds: {
    testnet: [
      "AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT"
    ]
  },

  // Query test parameters organized by category
  queryParameters: {
    identity: {
      getIdentity: {
        testnet: [
          { id: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk" },
          { id: "5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX" }
        ]
      },
      getIdentityKeys: {
        testnet: [
          { 
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            keyRequestType: "all"
          },
          {
            identityId: "5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX",
            keyRequestType: "specific",
            specificKeyIds: ["1", "2"]
          }
        ]
      },
      getIdentityBalance: {
        testnet: [
          { id: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk" }
        ]
      },
      getIdentityByPublicKeyHash: {
        testnet: [
          { publicKeyHash: "b7e904ce25ed97594e72f7af0e66f298031c1754" }
        ]
      },
      getIdentitiesContractKeys: {
        testnet: [
          {
            identitiesIds: [
              "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
              "5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX"
            ],
            contractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            purposes: ["0", "3"] // Authentication and Transfer
          }
        ]
      },
      getIdentityNonce: {
        testnet: [
          { identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk" }
        ]
      },
      getIdentityContractNonce: {
        testnet: [
          { 
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            contractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
          }
        ]
      },
      getIdentitiesBalances: {
        testnet: [
          {
            identityIds: [
              "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
              "5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX"
            ]
          }
        ]
      },
      getIdentityBalanceAndRevision: {
        testnet: [
          { id: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk" }
        ]
      },
      getIdentityByNonUniquePublicKeyHash: {
        testnet: [
          { publicKeyHash: "518038dc858461bcee90478fd994bba8057b7531" }
        ]
      },
      getIdentityTokenBalances: {
        testnet: [
          {
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            tokenIds: [
              "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv",
              "HEv1AYWQfwCffXQgmuzmzyzUo9untRTmVr67n4e4PSWa"
            ]
          }
        ]
      },
      getIdentitiesTokenBalances: {
        testnet: [
          {
            identityIds: [
              "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
              "5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX"
            ],
            tokenId: "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
          }
        ]
      },
      getIdentityTokenInfos: {
        testnet: [
          {
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            tokenIds: [
              "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv",
              "4tyvbA2ZGFLvjXLnJRCacSoMbFfpmBwGRrAZsVwnfYri"
            ]
          }
        ]
      },
      getIdentitiesTokenInfos: {
        testnet: [
          {
            identityIds: [
              "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
              "5RG84o6KsTaZudDqS8ytbaRB8QP4YYQ2uwzb6Hj8cfjX"
            ],
            tokenId: "4tyvbA2ZGFLvjXLnJRCacSoMbFfpmBwGRrAZsVwnfYri"
          }
        ]
      }
    },

    dataContract: {
      getDataContract: {
        testnet: [
          { id: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec" },
          { id: "ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A" }
        ]
      },
      getDataContracts: {
        testnet: [
          { 
            ids: [
              "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
              "ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A"
            ]
          }
        ]
      },
      getDataContractHistory: {
        testnet: [
          { 
            id: "HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM",
            limit: 10,
            offset: 0
          }
        ]
      }
    },

    document: {
      getDocuments: {
        testnet: [
          {
            dataContractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            documentType: "domain",
            limit: 10,
            where: '[["normalizedParentDomainName", "==", "dash"], ["normalizedLabel", "startsWith", "test"]]',
            orderBy: '[["normalizedLabel", "asc"]]'
          }
        ]
      },
      getDocument: {
        testnet: [
          {
            dataContractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            documentType: "domain",
            documentId: "7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r"
          }
        ]
      }
    },

    system: {
      getStatus: {
        testnet: [{}] // No parameters needed
      },
      getTotalCreditsInPlatform: {
        testnet: [{}]
      },
      getCurrentQuorumsInfo: {
        testnet: [{}] // No parameters needed
      },
      getPrefundedSpecializedBalance: {
        testnet: [
          { identityId: "AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT" }
        ]
      }
    },

    protocol: {
      getProtocolVersionUpgradeState: {
        testnet: [{}] // No parameters needed
      },
      getProtocolVersionUpgradeVoteStatus: {
        testnet: [
          { 
            startProTxHash: "143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113",
            count: 100
          }
        ]
      }
    },

    epoch: {
      getCurrentEpoch: {
        testnet: [{}]
      },
      getEpochsInfo: {
        testnet: [
          {
            startEpoch: 1000,
            count: 100,
            ascending: true
          }
        ]
      },
      getFinalizedEpochInfos: {
        testnet: [
          {
            startEpoch: 8635,
            count: 100,
            ascending: true
          }
        ]
      },
      getEvonodesProposedEpochBlocksByIds: {
        testnet: [
          {
            epoch: 8635,
            ids: ["143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113"]
          }
        ]
      },
      getEvonodesProposedEpochBlocksByRange: {
        testnet: [
          {
            epoch: 8635,
            limit: 10,
            startAfter: "143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113",
            orderAscending: true
          }
        ]
      }
    },

    dpns: {
      getDpnsUsername: {
        testnet: [
          { 
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            limit: 10
          }
        ]
      },
      dpnsCheckAvailability: {
        testnet: [
          { label: "alice" },
          { label: "test-username" },
          { label: "available-name" }
        ]
      },
      dpnsResolve: {
        testnet: [
          { name: "therea1s11mshaddy5" },
          { name: "alice.dash" },
          { name: "test-name" }
        ]
      },
      dpnsSearch: {
        testnet: [
          { 
            prefix: "the",
            limit: 10
          },
          {
            prefix: "test",
            limit: 5
          }
        ]
      }
    },

    token: {
      getTokenStatuses: {
        testnet: [
          {
            tokenIds: ["Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv", "H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy"]
          }
        ]
      },
      getTokenDirectPurchasePrices: {
        testnet: [
          {
            tokenIds: ["H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy"]
          }
        ]
      },
      getTokenContractInfo: {
        testnet: [
          {
            dataContractId: "H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy"
          }
        ]
      },
      getTokenPerpetualDistributionLastClaim: {
        testnet: [
          {
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            tokenId: "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
          }
        ]
      },
      getTokenTotalSupply: {
        testnet: [
          {
            tokenId: "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
          }
        ]
      }
    },

    voting: {
      getContestedResources: {
        testnet: [
          {
            documentTypeName: "domain",
            dataContractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            indexName: "parentNameAndLabel",
            limit: 10,
            offset: 0,
            orderAscending: true
          }
        ]
      },
      getContestedResourceVoteState: {
        testnet: [
          {
            dataContractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            documentTypeName: "domain",
            indexName: "parentNameAndLabel",
            indexValues: ["dash", "alice"],
            resultType: "contenders",
            allowIncludeLockedAndAbstainingVoteTally: false,
            count: 10,
            orderAscending: true
          }
        ]
      },
      getContestedResourceVotersForIdentity: {
        testnet: [
          {
            dataContractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            documentTypeName: "domain",
            indexName: "parentNameAndLabel",
            indexValues: ["dash", "alice"],
            contestantId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            count: 10,
            orderAscending: true
          }
        ]
      },
      getContestedResourceIdentityVotes: {
        testnet: [
          {
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            limit: 10,
            offset: 0,
            orderAscending: true
          }
        ]
      },
      getVotePollsByEndDate: {
        testnet: [
          {
            limit: 10,
            offset: 0,
            orderAscending: true
          }
        ]
      }
    },

    group: {
      getGroupInfo: {
        testnet: [
          {
            contractId: "49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N",
            groupContractPosition: 0
          }
        ]
      },
      getGroupInfos: {
        testnet: [
          {
            contractId: "49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N",
            count: 100
          }
        ]
      },
      getGroupActions: {
        testnet: [
          {
            contractId: "49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N",
            groupContractPosition: 0,
            status: "ACTIVE",
            count: 10
          }
        ]
      },
      getGroupActionSigners: {
        testnet: [
          {
            contractId: "49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N",
            groupContractPosition: 0,
            status: "ACTIVE",
            actionId: "6XJzL6Qb8Zhwxt4HFwh8NAn7q1u4dwdoUf8EmgzDudFZ"
          }
        ]
      }
    }
  },

  // State transition test parameters organized by category
  stateTransitionParameters: {
    identity: {
      identityCreate: {
        testnet: [
          {
            seedPhrase: process.env.TEST_SEED_PHRASE_1 || "placeholder seed phrase",
            identityIndex: 0,
            keySelectionMode: "simple",
            assetLockProof: "a914b7e904ce25ed97594e72f7af0e66f298031c175487",
            privateKey: process.env.TEST_PRIVATE_KEY_IDENTITY_1 || "PLACEHOLDER_IDENTITY_KEY_1",
            expectedKeys: 2,
            description: "Test identity creation with standard seed phrase"
          }
        ]
      },
      identityTopUp: {
        testnet: [
          {
            identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            assetLockProof: "a914b7e904ce25ed97594e72f7af0e66f298031c175487",
            privateKey: process.env.TEST_PRIVATE_KEY_IDENTITY_1 || "PLACEHOLDER_IDENTITY_KEY_1",
            description: "Top up existing identity with credits"
          }
        ]
      },
      identityCreditTransfer: {
        testnet: [
          {
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            recipientId: "HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG",
            amount: 100000, // 0.000001 DASH in credits
            privateKey: process.env.TEST_PRIVATE_KEY_TRANSFER || "PLACEHOLDER_TRANSFER_KEY", // Transfer key
            description: "Transfer credits between identities"
          }
        ]
      },
      identityCreditWithdrawal: {
        testnet: [
          {
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            toAddress: "yQW6TmUFef5CDyhEYwjoN8aUTMmKLYYNDm",
            amount: 190000, // 0.0000019 DASH in credits (minimum withdrawal amount)
            privateKey: process.env.TEST_PRIVATE_KEY_TRANSFER || "PLACEHOLDER_TRANSFER_KEY",
            description: "Withdraw credits to Dash address"
          }
        ]
      }
    },
    dataContract: {
      dataContractCreate: {
        testnet: [
          {
            canBeDeleted: false,
            readonly: false,
            keepsHistory: false,
            documentSchemas: '{"note": {"type": "object", "properties": {"message": {"type": "string", "position": 0}}, "additionalProperties": false}}',
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            description: "Create simple test data contract with document schema"
          }
        ]
      },
      dataContractUpdate: {
        testnet: [
          {
            dataContractId: "5kMgvQ9foEQ9TzDhz5jvbJ9Lhv5qqBpUeYEezHNEa6Ti", // Sample contract ID
            newDocumentSchemas: '{"note": {"type": "object", "properties": {"message": {"type": "string", "position": 0}, "author": {"type": "string", "position": 1}}, "additionalProperties": false}}',
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            description: "Update existing note document schema to add author field"
          }
        ]
      }
    },
    document: {
      documentCreate: {
        testnet: [
          {
            contractId: "5kMgvQ9foEQ9TzDhz5jvbJ9Lhv5qqBpUeYEezHNEa6Ti", // Use simple note contract (will be created by dataContractCreate test)
            documentType: "note",
            documentFields: {
              message: "Document created for WASM-SDK UI testing"
            },
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            description: "Create test note document with simple schema"
          }
        ]
      },
      documentReplace: {
        testnet: [
          {
            contractId: "5kMgvQ9foEQ9TzDhz5jvbJ9Lhv5qqBpUeYEezHNEa6Ti", // Use simple note contract
            documentType: "note",
            documentId: "Dy19ZeYPpqbEDcpsPcLwkviY5GZqT7yJL2EY4YfxTYjn", // Persistent testnet document
            documentFields: {
              message: "Updated document message for automation testing"
            },
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            description: "Replace existing note document"
          }
        ]
      },
      documentDelete: {
        testnet: [
          {
            contractId: "5kMgvQ9foEQ9TzDhz5jvbJ9Lhv5qqBpUeYEezHNEa6Ti", // Use simple note contract
            documentType: "note",
            documentId: "PLACEHOLDER_DOCUMENT_ID", // Will be set dynamically
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            description: "Delete existing note document"
          }
        ]
      },
      documentTransfer: {
        testnet: [
          {
            identityId: "HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG", // Current owner
            privateKey: process.env.TEST_PRIVATE_KEY_SECONDARY || "PLACEHOLDER_CONTRACT_KEY",
            contractId: "HdRFTcxgwPSVgzdy6MTYutDLJdbpfLMXwuBaYLYKMVHv", // Use NFT contract
            documentType: "card",
            documentId: "EypPkQLgT6Jijht7NYs4jmK5TGzkNd1Z4WrQdH1hND59", // Existing trading card document
            recipientId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC", // Transfer recipient
            description: "Transfer trading card ownership to secondary identity"
          }
        ]
      },
      documentPurchase: {
        testnet: [
          {
            identityId: "HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG", // Buyer identity
            privateKey: process.env.TEST_PRIVATE_KEY_SECONDARY || "PLACEHOLDER_SECONDARY_KEY",
            contractId: "HdRFTcxgwPSVgzdy6MTYutDLJdbpfLMXwuBaYLYKMVHv", // Use NFT contract
            documentType: "card",
            documentId: "EypPkQLgT6Jijht7NYs4jmK5TGzkNd1Z4WrQdH1hND59", // Existing trading card document
            price: 1000, // Price in credits
            description: "Purchase a priced trading card with secondary identity"
          }
        ]
      },
      documentSetPrice: {
        testnet: [
          {
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC", // Primary identity owns card after creation
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            contractId: "HdRFTcxgwPSVgzdy6MTYutDLJdbpfLMXwuBaYLYKMVHv", // Use NFT contract
            documentType: "card",
            documentId: "EypPkQLgT6Jijht7NYs4jmK5TGzkNd1Z4WrQdH1hND59", // Existing trading card document
            price: 1000, // Price in credits
            description: "Set price for a trading card"
          }
        ]
      }
    },
    token: {
      tokenMint: {
        testnet: [
          {
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            amount: "2",
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            // issuedToIdentityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            publicNote: "Token mint test",
            description: "Mint new tokens (may fail without permissions)"
          }
        ]
      },      
      tokenTransfer: {
        testnet: [
          {
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            amount: "1",
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            recipientId: "HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG",            
            publicNote: "Token transfer test",
            description: "Transfer tokens between identities"
          }
        ]
      },
      tokenBurn: {
        testnet: [
          {
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            amount: "1",
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            publicNote: "Token burn test",
            description: "Burn tokens from identity balance"
          }
        ]
      },
      tokenFreeze: {
        testnet: [
          {
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            identityToFreeze: "HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            publicNote: "Token freeze test",
            description: "Freeze tokens for an identity"
          }
        ]
      },
      tokenDestroyFrozen: {
        testnet: [
          {
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            frozenIdentityId: "HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            publicNote: "Destroy frozen tokens test",
            description: "Destroy frozen tokens from an identity"
          }
        ]
      },
      tokenUnfreeze: {
        testnet: [
          {
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            identityToUnfreeze: "HJDxtN6FJF3U3T9TMLWCqudfJ5VRkaUrxTsRp36djXAG",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            publicNote: "Token unfreeze test",
            description: "Unfreeze tokens for an identity"
          }
        ]
      },
      tokenClaim: {
        testnet: [
          {
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            distributionType: "perpetual",
            publicNote: "Token claim test",
            description: "Claim tokens from distribution"
          }
        ]
      },
      tokenSetPriceForDirectPurchase: {
        testnet: [
          {
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            priceType: "single",
            priceData: "10",
            publicNote: "Set token price test",
            description: "Set price for direct token purchases"
          }
        ]
      },
      tokenDirectPurchase: {
        testnet: [
          {
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            amount: "1",
            totalAgreedPrice: "10",
            description: "Direct purchase of tokens at configured price"
          }
        ]
      },
      tokenConfigUpdate: {
        testnet: [
          {
            identityId: "7XcruVSsGQVSgTcmPewaE4tXLutnW1F6PXxwMbo8GYQC",
            privateKey: process.env.TEST_PRIVATE_KEY_CONTRACT || "PLACEHOLDER_CONTRACT_KEY",
            contractId: "Afk9QSj9UDE14K1y9y3iSx6kUSm5LLmhbdAvPvWL4P2i",
            tokenPosition: 0,
            configItemType: "max_supply",
            configValue: "1000000",
            publicNote: "Update max supply test",
            description: "Update token configuration max supply"
          }
        ]
      },
    }
  },

  // Common where clauses for document queries
  whereClausesExamples: {
    dpnsDomain: [
      [["normalizedParentDomainName", "==", "dash"]],
      [["normalizedParentDomainName", "==", "dash"], ["normalizedLabel", "startsWith", "test"]]
    ]
  },

  // Order by examples
  orderByExamples: {
    createdAtDesc: [["$createdAt", "desc"]],
    createdAtAsc: [["$createdAt", "asc"]]
  }
};

/**
 * Get test parameters for a specific query
 */
function getTestParameters(category, queryType, network = 'testnet') {
  const categoryData = testData.queryParameters[category];
  if (!categoryData) {
    throw new Error(`No test data found for category: ${category}`);
  }

  const queryData = categoryData[queryType];
  if (!queryData) {
    throw new Error(`No test data found for query: ${category}.${queryType}`);
  }

  const networkData = queryData[network];
  if (!networkData || networkData.length === 0) {
    throw new Error(`No test data found for ${category}.${queryType} on ${network}`);
  }

  return networkData[0]; // Return first test case
}

/**
 * Get all test parameters for a query (for parameterized testing)
 */
function getAllTestParameters(category, queryType, network = 'testnet') {
  const categoryData = testData.queryParameters[category];
  if (!categoryData) return [];

  const queryData = categoryData[queryType];
  if (!queryData) return [];

  return queryData[network] || [];
}

/**
 * Get test parameters for a specific state transition
 */
function getStateTransitionParameters(category, transitionType, network = 'testnet') {
  const categoryData = testData.stateTransitionParameters[category];
  if (!categoryData) {
    throw new Error(`No state transition test data found for category: ${category}`);
  }

  const transitionData = categoryData[transitionType];
  if (!transitionData) {
    throw new Error(`No state transition test data found for transition: ${category}.${transitionType}`);
  }

  const networkData = transitionData[network];
  if (!networkData || networkData.length === 0) {
    throw new Error(`No state transition test data found for ${category}.${transitionType} on ${network}`);
  }

  return networkData[0]; // Return first test case
}

/**
 * Get all test parameters for a state transition (for parameterized testing)
 */
function getAllStateTransitionParameters(category, transitionType, network = 'testnet') {
  const categoryData = testData.stateTransitionParameters[category];
  if (!categoryData) return [];

  const transitionData = categoryData[transitionType];
  if (!transitionData) return [];

  return transitionData[network] || [];
}

module.exports = {
  testData,
  getTestParameters,
  getAllTestParameters,
  getStateTransitionParameters,
  getAllStateTransitionParameters
};
