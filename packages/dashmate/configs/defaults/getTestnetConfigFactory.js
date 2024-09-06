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
        gateway: {
          listeners: {
            dapiAndDrive: {
              port: 1443,
            },
          },
        },
        drive: {
          abci: {
            epochTime: 3600,
            validatorSet: {
              quorum: {
                llmqType: 6,
                dkgInterval: 24,
                activeSigners: 24,
                rotation: false,
              },
            },
            chainLock: {
              quorum: {
                llmqType: 1,
                dkgInterval: 24,
                activeSigners: 24,
                rotation: false,
              },
            },
            instantLock: {
              quorum: {
                llmqType: 5,
                dkgInterval: 288,
                activeSigners: 32,
                rotation: true,
              },
            },
            proposer: {
              txProcessingTimeLimit: 5000,
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
            mempool: {
              timeoutCheckTx: '3s',
              txEnqueueTimeout: '30ms',
              txSendRateLimit: 100,
              txRecvRateLimit: 120,
              ttlDuration: '24h',
              ttlNumBlocks: 0,
            },
            rpc: {
              port: 36657,
              timeoutBroadcastTx: '1s',
            },
            pprof: {
              port: 36060,
            },
            metrics: {
              port: 36660,
            },
            genesis: {
              chain_id: 'dash-testnet-51',
              validator_quorum_type: 6,
            },
          },
        },
      },
      network: NETWORK_TESTNET,
    };

    return new Config('testnet', lodashMerge({}, getBaseConfig()
      .getOptions(), options));
  }

  return getTestnetConfig;
}
