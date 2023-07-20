const lodashMerge = require('lodash/merge');

const {
  NETWORK_LOCAL,
} = require('../../../constants');

const Config = require('../../Config');

/**
 * @param {getBaseConfig} getBaseConfig
 * @returns {getLocalConfig}
 */
function getLocalConfigFactory(getBaseConfig) {
  /**
   * @typedef {function} getLocalConfig
   * @returns {Config}
   */
  function getLocalConfig() {
    const options = {
      description: 'template for local configs',
      docker: {
        network: {
          subnet: '172.24.24.0/24',
        },
      },
      core: {
        p2p: {
          port: 20001,
        },
        rpc: {
          port: 20002,
        },
      },
      platform: {
        dapi: {
          envoy: {
            http: {
              port: 2443,
            },
            rateLimiter: {
              enabled: false,
            },
          },
        },
        drive: {
          tenderdash: {
            p2p: {
              port: 46656,
            },
            rpc: {
              port: 46657,
            },
          },
          abci: {
            validatorSet: {
              llmqType: 106,
            },
          },
        },
      },
      environment: 'development',
      network: NETWORK_LOCAL,
    };

    return new Config('local', lodashMerge({}, getBaseConfig().getOptions(), options));
  }

  return getLocalConfig;
}

module.exports = getLocalConfigFactory;
