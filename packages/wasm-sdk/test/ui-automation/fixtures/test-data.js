/**
 * Test data extracted from existing WASM SDK test parameters
 * Based on update_inputs.py and existing test files
 */

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
            documentTypeName: "domain",
            keyRequestType: "all"
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
            limit: 10
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
          { name: "alice" },
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
            resultType: "documents",
            allowIncludeLockedAndAbstainingVoteTally: false,
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
            // indexValues: "['dash', 'alice']", // This field is missing from the actual site currently
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
            startTimeMs: 1735689600000,
            endTimeMs: 1754006400000,
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

module.exports = {
  testData,
  getTestParameters,
  getAllTestParameters
};
