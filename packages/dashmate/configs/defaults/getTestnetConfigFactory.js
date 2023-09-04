const lodashMerge = require('lodash/merge');

const {
  NETWORK_TESTNET,
} = require('../../src/constants');

const Config = require('../../src/config/Config');

/**
 * @param {HomeDir} homeDir
 * @param {getBaseConfig} getBaseConfig
 * @returns {getTestnetConfig}
 */
function getTestnetConfigFactory(homeDir, getBaseConfig) {
  /**
   * @typedef {function} getTestnetConfig
   * @returns {Config}
   */
  function getTestnetConfig() {
    const options = {
      description: 'node with testnet configuration',
      docker: {
        network: {
          subnet: '172.25.24.0/24',
        },
      },
      core: {
        docker: {
          image: 'dashpay/dashd:20.0.0-alpha.6',
        },
        p2p: {
          port: 19999,
        },
        rpc: {
          port: 19998,
        },
        log: {
          file: {
            categories: [],
            path: homeDir.joinPath('logs', 'testnet', 'core.log'),
          },
        },
        spork: {
          address: 'yjPtiKh2uwk3bDutTEA2q9mCtXyiZRWn55',
        },
      },
      platform: {
        dapi: {
          envoy: {
            http: {
              port: 1443,
            },
          },
        },
        drive: {
          abci: {
            log: {
              prettyFile: {
                path: homeDir.joinPath('logs', 'testnet', 'drive-pretty.log'),
              },
              jsonFile: {
                path: homeDir.joinPath('logs', 'testnet', 'drive-json.log'),
              },
            },
            validatorSet: {
              llmqType: 6,
            },
          },
          tenderdash: {
            p2p: {
              seeds: [
                {
                  id: '74907790a03b51ac062c8a1453dafd72a08668a3',
                  host: '35.92.167.154',
                  port: 36656,
                },
                {
                  id: '2006632eb20e670923d13d4f53abc24468eaad4d',
                  host: '52.12.116.10',
                  port: 36656,
                },
              ],
              port: 36656,
            },
            rpc: {
              port: 36657,
            },
            genesis: {
              genesis_time: '2023-04-26T10:43:20.921Z',
              chain_id: 'dash-testnet-22',
              initial_core_chain_locked_height: 854281,
              consensus_params: {
                timeout: {
                  propose: '50000000000',
                  propose_delta: '10000000000',
                  vote: '500000000',
                  vote_delta: '100000000',
                  commit: '1000000000',
                  bypass_commit_timeout: false,
                },
                block: {
                  max_bytes: '22020096',
                  max_gas: '-1',
                  time_iota_ms: '5000',
                },
                evidence: {
                  max_age: '100000',
                  max_age_num_blocks: '100000',
                  max_age_duration: '172800000000000',
                },
                validator: {
                  pub_key_types: [
                    'bls12381',
                  ],
                },
                version: {
                  app_version: '1',
                },
              },
              validator_quorum_type: 6,
            },
          },
        },
        dpns: {
          masterPublicKey: '02c8b4747b528cac5fddf7a6cc63702ee04ed7d1332904e08510343ea00dce546a',
          secondPublicKey: '0201ee28f84f5485390567e939c2b586010b63a69ec92cab535dc96a8c71913602',
        },
        dashpay: {
          masterPublicKey: '02d4dcce3f0a8d2936ce26df4d255fd2835b629b73eea39d4b2778096b91e77946',
          secondPublicKey: '03699c8b4ebf1696c92e9ec605a02a38f6f9cec47d13fb584fdad779e936e20ccb',
        },
        featureFlags: {
          masterPublicKey: '029cf2232549de08c114c19763309cb067688e21e310ac07458b59c2c026be7234',
          secondPublicKey: '02a2abb50c03ae9f778f08a93849ba334a82e625153720dd5ef14e564b78b414e5',
        },
        masternodeRewardShares: {
          masterPublicKey: '0319d795c0795bc8678bd0e58cfc7a4ad75c8e1797537728e7e8de8b9acc2bae2b',
          secondPublicKey: '033756572938aaad752158b858ad38511c6edff4c79cf8462f70baa25fc6e8a616',
        },
        withdrawals: {
          masterPublicKey: '032f79d1d9d6e652599d3315d30306b1277fbf588e32e383aef0a59749547d47b7',
          secondPublicKey: '03eebbe3dc3721603a0b5a13441f214550ffa7d035b7dea9f1911de0f63ddac58d',
        },
      },
      network: NETWORK_TESTNET,
    };

    return new Config('testnet', lodashMerge({}, getBaseConfig().getOptions(), options));
  }

  return getTestnetConfig;
}

module.exports = getTestnetConfigFactory;
