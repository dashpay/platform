/**
 * Test data extracted from existing WASM SDK test parameters
 * Based on update_inputs.py and existing test files
 */

const testData = {
  // Known testnet identity IDs for testing
  identityIds: {
    testnet: [
      "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
      "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
    ],
    mainnet: [
      // Add mainnet IDs when available
    ]
  },

  // Data contract IDs
  dataContracts: {
    testnet: {
      dpns: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
      dashpay: "ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A",
      sample: "HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM"
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
      "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
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
          { id: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec" },
          { id: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk" }
        ]
      },
      getIdentityKeys: {
        testnet: [
          { 
            identityId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            keyRequestType: "all"
          },
          {
            identityId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            keyRequestType: "specific",
            specificKeyIds: ["1", "2"]
          }
        ]
      },
      getIdentityBalance: {
        testnet: [
          { id: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec" }
        ]
      },
      getIdentityByPublicKeyHash: {
        testnet: [
          { publicKeyHash: "b7e904ce25ed97594e72f7af0e66f298031c1754" }
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
      getCurrentEpoch: {
        testnet: [{}]
      },
      getTotalCreditsInPlatform: {
        testnet: [{}]
      }
    },

    protocol: {
      getProtocolVersionUpgradeState: {
        testnet: [{}]
      }
    },

    epoch: {
      getEpochsInfo: {
        testnet: [
          {
            epoch: 1000,
            count: 5,
            ascending: true
          }
        ]
      },
      getEvonodesProposedEpochBlocksByIds: {
        testnet: [
          {
            ids: ["143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113"]
          }
        ]
      }
    },

    token: {
      getTokenStatuses: {
        testnet: [
          {
            tokenIds: ["Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"]
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