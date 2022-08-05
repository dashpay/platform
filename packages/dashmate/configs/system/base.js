const path = require('path');

const {
  contractId: dpnsContractId,
  ownerId: dpnsOwnerId,
} = require('@dashevo/dpns-contract/lib/systemIds');

const {
  contractId: dashpayContractId,
} = require('@dashevo/dashpay-contract/lib/systemIds');

const {
  contractId: featureFlagsContractId,
  ownerId: featureFlagsOwnerId,
} = require('@dashevo/feature-flags-contract/lib/systemIds');

const {
  contractId: masternodeRewardSharesContractId,
} = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');

const {
  NETWORK_TESTNET,
  HOME_DIR_PATH,
} = require('../../src/constants');

module.exports = {
  description: 'base config for use as template',
  group: null,
  docker: {
    network: {
      subnet: '172.24.24.0/24',
    },
  },
  core: {
    docker: {
      image: 'dashpay/dashd:18.0.0-rc6',
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
      mediantime: null,
      address: null,
    },
    sentinel: {
      docker: {
        image: 'dashpay/sentinel:1.6.0',
      },
    },
    debug: 0,
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
          image: 'dashpay/dapi:0.23-dev',
        },
      },
    },
    drive: {
      abci: {
        docker: {
          image: 'dashpay/drive:0.23-dev',
        },
        log: {
          stdout: {
            level: 'info',
          },
          prettyFile: {
            level: 'silent',
            path: path.join(HOME_DIR_PATH, 'logs', 'base', 'drive-pretty.log'),
          },
          jsonFile: {
            level: 'silent',
            path: path.join(HOME_DIR_PATH, 'logs', 'base', 'drive-json.log'),
          },
        },
        validatorSet: {
          llmqType: 4,
        },
      },
      tenderdash: {
        docker: {
          image: 'dashpay/tenderdash:0.8.0-dev.6',
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
        log: {
          level: {
            main: 'info',
            state: 'info',
            statesync: 'info',
            '*': 'error',
          },
          format: 'plain',
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
        id: dpnsContractId,
      },
      ownerId: dpnsOwnerId,
      masterPublicKey: null,
      secondPublicKey: null,
    },
    dashpay: {
      contract: {
        id: dashpayContractId,
      },
      masterPublicKey: null,
      secondPublicKey: null,
    },
    featureFlags: {
      contract: {
        id: featureFlagsContractId,
      },
      ownerId: featureFlagsOwnerId,
      masterPublicKey: null,
      secondPublicKey: null,
    },
    sourcePath: null,
    masternodeRewardShares: {
      contract: {
        id: masternodeRewardSharesContractId,
      },
      masterPublicKey: null,
      secondPublicKey: null,
    },
  },
  externalIp: null,
  network: NETWORK_TESTNET,
  environment: 'production',
};
