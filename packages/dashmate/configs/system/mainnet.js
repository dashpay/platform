const lodashMerge = require('lodash/merge');

const path = require('path');
const {
  NETWORK_MAINNET, HOME_DIR_PATH,
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
      image: 'dashpay/dashd:19.2.0',
    },
    indexes: false,
    log: {
      file: {
        categories: [],
        path: path.join(HOME_DIR_PATH, 'logs', 'mainnet', 'core.log'),
      },
    },
  },
  network: NETWORK_MAINNET,
  platform: {
    enable: false,
  },
});

module.exports = mainnetConfig;
