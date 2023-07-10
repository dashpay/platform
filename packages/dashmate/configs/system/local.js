const lodashMerge = require('lodash/merge');

const {
  NETWORK_LOCAL,
} = require('../../src/constants');

const baseConfig = require('./base');

module.exports = lodashMerge({}, baseConfig, {
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
});
