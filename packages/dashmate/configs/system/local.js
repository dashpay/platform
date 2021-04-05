const lodashMerge = require('lodash.merge');

const {
  NETWORK_LOCAL,
} = require('../../src/constants');

const baseConfig = require('./base');

module.exports = lodashMerge({}, baseConfig, {
  description: 'template for local configs',
  platform: {
    dapi: {
      nginx: {
        rateLimiter: {
          enable: false,
        },
      },
    },
    drive: {
      skipAssetLockConfirmationValidation: true,
      passFakeAssetLockProofForTests: true,
      tenderdash: {
        consensus: {
          createEmptyBlocks: true,
          createEmptyBlocksInterval: '10s',
        },
      },
    },
  },
  externalIp: '127.0.0.1',
  environment: 'development',
  network: NETWORK_LOCAL,
});
