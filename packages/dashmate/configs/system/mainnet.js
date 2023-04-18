const lodashMerge = require('lodash/merge');

const {
  NETWORK_MAINNET,
} = require('../../src/constants');

const baseConfig = require('./base');

const mainnetConfig = lodashMerge({}, baseConfig, {
  description: 'node with mainnet configuration',
  docker: {
    network: {
      subnet: '172.26.24.0/24',
    },
  },
  core: {
    docker: {
      image: 'dashpay/dashd:18.2.2',
    },
    indexes: false,
  },
  network: NETWORK_MAINNET,
  platform: {
    enable: false,
  },
});

module.exports = mainnetConfig;
