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
      image: 'dashpay/dashd:18.1.0-rc.1',
    },
    p2p: {
      port: 9999,
    },
    rpc: {
      port: 9998,
    },
  },
  network: NETWORK_MAINNET,
});

delete mainnetConfig.platform;

module.exports = mainnetConfig;
