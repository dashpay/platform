import lodash from 'lodash';
import tenderdashSeeds from './tenderdashSeeds.js';

import {
  NETWORK_MAINNET,
} from '../../src/constants.js';

import Config from '../../src/config/Config.js';

const { merge: lodashMerge } = lodash;

/**
 * @param {HomeDir} homeDir
 * @param {getBaseConfig} getBaseConfig
 * @returns {getMainnetConfig}
 */
export default function getMainnetConfigFactory(homeDir, getBaseConfig) {
  /**
   * @typedef {function} getMainnetConfig
   * @returns {Config}
   */
  function getMainnetConfig() {
    const options = {
      description: 'node with mainnet configuration',
      docker: {
        network: {
          subnet: '172.26.24.0/24',
        },
      },
      network: NETWORK_MAINNET,
      platform: {
        enable: false,
        drive: {
          tenderdash: {
            p2p: {
              seeds: tenderdashSeeds.mainnet.seeds,
            },
            mempool: {
              timeoutCheckTx: '3s',
              txEnqueueTimeout: '30ms',
              txSendRateLimit: 100,
              txRecvRateLimit: 120,
              ttlDuration: '24h',
              ttlNumBlocks: 0,
            },
            genesis: {
              chain_id: 'evo1',
              validator_quorum_type: 4,
              consensus_params: {
                version: {
                  app_version: '1',
                },
              },
            },
          },
          abci: {
            proposer: {
              txProcessingTimeLimit: 5000,
            },
          },
        },
      },
    };

    return new Config('mainnet', lodashMerge({}, getBaseConfig().getStoredOptions(), options));
  }

  return getMainnetConfig;
}
