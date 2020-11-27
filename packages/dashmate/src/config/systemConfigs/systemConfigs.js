const lodashMerge = require('lodash.merge');

const NETWORKS = require('../../networks');

const baseConfig = {
  description: 'base config for use as template',
  core: {
    docker: {
      image: 'dashpay/dashd-develop:latest',
    },
    p2p: {
      port: 20001,
    },
    masternode: {
      enable: true,
      operator: {
        privateKey: null,
      },
    },
    miner: {
      enable: false,
      interval: '2.5m',
      address: null,
    },
  },
  platform: {
    dapi: {
      envoy: {
        docker: {
          image: 'envoyproxy/envoy:v1.14-latest',
        },
      },
      nginx: {
        docker: {
          image: 'nginx:latest',
        },
      },
      api: {
        docker: {
          image: 'dashpay/dapi:0.17-dev',
        },
      },
      insight: {
        docker: {
          image: 'dashpay/insight-api:3.0.1',
        },
      },
    },
    drive: {
      mongodb: {
        docker: {
          image: 'mongo:4.2',
        },
      },
      abci: {
        docker: {
          image: 'dashpay/drive:0.17-dev',
        },
        log: {
          level: 'info',
        },
      },
      tendermint: {
        docker: {
          image: 'dashpay/tendermint:v0.32.12',
        },
      },
    },
    dpns: {
      contract: {
        id: null,
        blockHeight: null,
      },
      ownerId: null,
    },
  },
  externalIp: null,
  network: NETWORKS.TESTNET,
  compose: {
    file: 'docker-compose.yml:docker-compose.platform.yml',
  },
  environment: 'production',
};

module.exports = {
  base: baseConfig,
  local: lodashMerge({}, baseConfig, {
    description: 'standalone node for local development',
    externalIp: '127.0.0.1',
    network: NETWORKS.LOCAL,
    environment: 'development',
  }),
  evonet: lodashMerge({}, baseConfig, {
    description: 'node with Evonet configuration',
    network: NETWORKS.EVONET,
    platform: {
      dpns: {
        contract: {
          id: '3VvS19qomuGSbEYWbTsRzeuRgawU3yK4fPMzLrbV62u8',
          blockHeight: 35,
        },
        ownerId: 'Gxiu28Lzfj66aPBCxD7AgTbbauLf68jFLNibWGU39Fuh',
      },
    },
  }),
  testnet: lodashMerge({}, baseConfig, {
    description: 'node with testnet configuration',
    core: {
      p2p: {
        port: 19999,
      },
    },
    network: NETWORKS.TESTNET,
    compose: {
      file: 'docker-compose.yml',
    },
  }),
};
