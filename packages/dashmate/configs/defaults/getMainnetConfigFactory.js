const lodashMerge = require('lodash/merge');

const {
  NETWORK_MAINNET,
} = require('../../src/constants');
const Config = require('../../src/config/Config');

/**
 * @param {HomeDir} homeDir
 * @param {getBaseConfig} getBaseConfig
 * @returns {getMainnetConfig}
 */
function getMainnetConfigFactory(homeDir, getBaseConfig) {
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
            categories: [],
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

module.exports = getMainnetConfigFactory;
