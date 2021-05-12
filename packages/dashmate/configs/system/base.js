const {
  NETWORK_TESTNET,
} = require('../../src/constants');

module.exports = {
  description: 'base config for use as template',
  group: null,
  core: {
    docker: {
      image: 'dashpay/dashd:0.17.0.0-rc4-hotfix2',
    },
    p2p: {
      port: 20001,
      seeds: [],
    },
    rpc: {
      port: 20002,
      user: 'dashrpc',
      password: 'rpcpassword',
    },
    spork: {
      address: null,
      privateKey: null,
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
    sentinel: {
      docker: {
        image: 'dashpay/sentinel:1.5.0',
      },
    },
    devnetName: null,
  },
  platform: {
    dapi: {
      envoy: {
        docker: {
          image: 'envoyproxy/envoy:v1.16-latest',
        },
        http: {
          port: 3000,
        },
        grpc: {
          port: 3010,
        },
        rateLimiter: {
          maxTokens: 300,
          tokensPerFill: 150,
          fillInterval: '60s',
          enabled: true,
        },
      },
      api: {
        docker: {
          image: 'dashpay/dapi:0.19',
          build: {
            path: null,
          },
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
          image: 'dashpay/drive:0.19',
          build: {
            path: null,
          },
        },
        log: {
          stdout: {
            level: 'info',
          },
          prettyFile: {
            level: 'silent',
            path: '/tmp/base-drive-pretty.log',
          },
          jsonFile: {
            level: 'silent',
            path: '/tmp/base-drive-json.json',
          },
        },
      },
      tenderdash: {
        docker: {
          image: 'dashpay/tenderdash:0.34.3',
        },
        p2p: {
          port: 26656,
          persistentPeers: [],
          seeds: [],
        },
        rpc: {
          port: 26657,
        },
        consensus: {
          createEmptyBlocks: true,
          createEmptyBlocksInterval: '3m',
        },
        validatorKey: {

        },
        nodeKey: {

        },
        genesis: {

        },
        nodeId: null,
      },
    },
    dpns: {
      contract: {
        id: null,
        blockHeight: null,
      },
      ownerId: null,
    },
    dashpay: {
      contract: {
        id: null,
        blockHeight: null,
      },
    },
    featureFlags: {
      contract: {
        id: null,
        blockHeight: null,
      },
      ownerId: null,
    },
  },
  externalIp: null,
  network: NETWORK_TESTNET,
  environment: 'production',
};
