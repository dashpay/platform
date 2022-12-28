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
      image: 'dashpay/dashd:18.1.0',
    },
    p2p: {
      port: 20001,
      seeds: [],
    },
    rpc: {
      port: 20002,
      user: 'dashrpc',
      password: 'rpcpassword',
      allowIps: [
        '127.0.0.1',
        '172.16.0.0/12',
        '192.168.0.0/16',
      ],
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
        image: 'dashpay/sentinel:1.7.1',
      },
    },
    debug: 0,
    logIps: 0,
    indexes: true,
    minimumDifficultyBlocks: 0,
    powTargetSpacing: 150,
    reindex: {
      enable: false,
      containerId: null,
    },
    devnetName: null,
  },
  platform: {
    dapi: {
      envoy: {
        docker: {
          image: 'dashpay/envoy:0.24-dev',
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
        ssl: {
          enabled: false,
          provider: 'zerossl',
          providerConfigs: {
            zerossl: {
              apiKey: null,
              id: null,
            },
          },
        },
      },
      api: {
        docker: {
          image: 'dashpay/dapi:0.24.0-dev',
        },
      },
    },
    drive: {
      abci: {
        docker: {
          image: 'dashpay/drive:0.24.0-dev',
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
          image: 'dashpay/tenderdash:0.10.0-dev.6',
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
          level: 'debug',
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
  dashmate: {
    helper: {
      docker: {
        image: 'dashpay/dashmate-helper:0.24-dev',
      },
    },
  },
  externalIp: null,
  network: NETWORK_TESTNET,
  environment: 'production',
};
