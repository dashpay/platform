import lodash from 'lodash';
import {
  NETWORK_TESTNET,
} from '../../src/constants.js';
import Config from '../../src/config/Config.js';

const { merge: lodashMerge } = lodash;
/**
 * @param {HomeDir} homeDir
 * @param {getBaseConfig} getBaseConfig
 * @returns {getTestnetConfig}
 */
export default function getTestnetConfigFactory(homeDir, getBaseConfig) {
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
        p2p: {
          port: 19999,
        },
        rpc: {
          port: 19998,
        },
        log: {
          file: {
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
            epochTime: 3600,
            validatorSet: {
              llmqType: 6,
            },
            chainLock: {
              llmqType: 1,
              dkgInterval: 24,
              llmqSize: 50,
            },
          },
          tenderdash: {
            p2p: {
              seeds: [
                {
                  id: '74907790a03b51ac062c8a1453dafd72a08668a3',
                  host: '35.166.35.250',
                  port: 36656,
                },
                {
                  id: '2006632eb20e670923d13d4f53abc24468eaad4d',
                  host: '35.92.64.72',
                  port: 36656,
                },
              ],
              port: 36656,
            },
            rpc: {
              port: 36657,
            },
            pprof: {
              port: 36060,
            },
            metrics: {
              port: 36660,
            },
            genesis: {
              genesis_time: '2023-11-02T10:18:00.000Z',
              chain_id: 'dash-testnet-37',
              validator_quorum_type: 6,
              initial_core_chain_locked_height: 918609,
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
