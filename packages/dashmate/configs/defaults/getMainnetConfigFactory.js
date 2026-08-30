import lodash from 'lodash';

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
              // Registered evonodes with open platform p2p, verified working
              // against the live evonode registry on 2026-08-30. Seeds rotate
              // with the registry, so this list rots; the long-term fix is to
              // generate it from the registry instead of hardcoding it.
              seeds: [
                {
                  id: 'ee9ab93559e6e931d7dbcf269e1ea8446e7068e5',
                  host: '149.28.241.190',
                  port: 26656,
                },
                {
                  id: '30918550e1f57eaff1b97f85adc8f4967065a16b',
                  host: '216.238.75.46',
                  port: 26656,
                },
                {
                  id: '6d9fe2b4f18b999521cf706e8c7b8559d4477e4c',
                  host: '89.125.209.110',
                  port: 26656,
                },
                {
                  id: 'dc812dc0e2e35a8a59491c5d20cba0390d045171',
                  host: '84.247.180.201',
                  port: 26656,
                },
                {
                  id: '3ed7bb4f1ed2f19cacd33f44a68b95d3f24cf85d',
                  host: '134.255.182.186',
                  port: 26656,
                },
              ],
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
