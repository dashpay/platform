const lodashMerge = require('lodash.merge');

const NETWORKS = require('../../networks');

const baseConfig = {
  description: 'base config for use as template',
  core: {
    docker: {
      image: 'dashpay/dashd:0.15',
    },
    p2p: {
      port: 20001,
    },
    masternode: {
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
          image: 'dashpay/dapi:0.15-dev',
        },
      },
      insight: {
        docker: {
          image: 'dashpay/insight-api:latest',
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
          image: 'dashpay/drive:0.15-dev',
        },
      },
      tendermint: {
        docker: {
          image: 'dashpay/tendermint:v0.32.12',
        },
      },
    },
    dpns: {
      contractId: null,
      ownerId: null,
    },
  },
  externalIp: null,
  network: NETWORKS.TESTNET,
  compose: {
    file: 'docker-compose.yml:docker-compose.platform.yml',
  },
};

module.exports = {
  base: baseConfig,
  local: lodashMerge({}, baseConfig, {
    description: 'standalone node for local development',
    externalIp: '127.0.0.1',
    network: NETWORKS.LOCAL,
  }),
  evonet: lodashMerge({}, baseConfig, {
    description: 'node with Evonet configuration',
    platform: {
      dpns: {
        contractId: 'FiBkhut4LFPMJqDWbZrxVeT6Mr6LsH3mTNTSSHJY2ape',
        ownerId: '6UZ9jAodWiFxRg82HuA1Lf3mTh4fTGSiughxqkZX5kUA',
      },
    },
    network: NETWORKS.EVONET,
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
