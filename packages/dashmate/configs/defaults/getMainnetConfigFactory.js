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
      core: {
        log: {
          file: {
            path: homeDir.joinPath('logs', 'mainnet', 'core.log'),
          },
        },
      },
      network: NETWORK_MAINNET,
      platform: {
        enable: false,
        drive: {
          tenderdash: {
            p2p: {
              seeds: [
                {
                  id: '069639dfceec5f7c86257e6e9c46407c16ad1eab',
                  host: '34.211.174.194',
                  port: 26656,
                },
                {
                  id: 'd46e2445642b2f94158ac3c2a6d90b88b83705b8',
                  host: '3.76.148.150',
                  port: 26656,
                },
                {
                  id: 'b08a650ecfac178939f21c0c12801eccaf18a5ea',
                  host: '3.0.60.103',
                  port: 26656,
                },
                {
                  id: '4cb4a8488eb1dbabda7fb79e47ac3c14eec73c4f',
                  host: '152.42.151.147',
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
              chain_id: 'dash-1',
              validator_quorum_type: 4,
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

    return new Config('mainnet', lodashMerge({}, getBaseConfig().getOptions(), options));
  }

  return getMainnetConfig;
}
