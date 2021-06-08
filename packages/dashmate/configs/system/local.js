const lodashMerge = require('lodash.merge');

const {
  NETWORK_LOCAL,
} = require('../../src/constants');

const baseConfig = require('./base');

module.exports = lodashMerge({}, baseConfig, {
  description: 'template for local configs',
  platform: {
    dapi: {
      envoy: {
        rateLimiter: {
          enabled: false,
        },
      },
    },
    drive: {
      abci: {
        validatorSet: {
          llmqType: 100,
        },
      },
    },
  },
  externalIp: null,
  environment: 'development',
  network: NETWORK_LOCAL,
});
