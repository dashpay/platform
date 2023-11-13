import * as lodashMerge from 'lodash/merge';

import {
  NETWORK_MAINNET,
} from '../../src/constants';
import {Config} from "../../src/config/Config.js";

/**
 * @param {HomeDir} homeDir
 * @param {getBaseConfig} getBaseConfig
 * @returns {getMainnetConfig}
 */
export function getMainnetConfigFactory(homeDir, getBaseConfig) {
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
        docker: {
          image: 'dashpay/dashd:19.3.0',
        },
        indexes: false,
        log: {
          file: {
            path: homeDir.joinPath('logs', 'mainnet', 'core.log'),
          },
        },
      },
      network: NETWORK_MAINNET,
      platform: {
        enable: false,
      },
    };

    return new Config('mainnet', lodashMerge({}, getBaseConfig().getOptions(), options));
  }

  return getMainnetConfig;
}
