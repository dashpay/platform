const path = require('path');

const {
  NETWORK_TESTNET,
  HOME_DIR_PATH,
} = require('../../src/constants');

module.exports = {
  description: 'base config for use as template',
  group: null,
  core: {
    docker: {
      image: 'dashpay/dashd:0.17',
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
          image: 'dashpay/dapi:0.21',
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
          image: 'dashpay/drive:0.21',
        },
        log: {
          stdout: {
            level: 'info',
          },
          prettyFile: {
            level: 'silent',
            path: path.join(HOME_DIR_PATH, 'base', 'logs', 'drive-pretty.log'),
          },
          jsonFile: {
            level: 'silent',
            path: path.join(HOME_DIR_PATH, 'base', 'logs', 'drive-json.log'),
          },
        },
        validatorSet: {
          llmqType: 4,
        },
      },
      tenderdash: {
        docker: {
          image: 'dashpay/tenderdash:0.7.0-dev.4',
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
        id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
      },
      ownerId: '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
      masterPublicKey: null,
    },
    dashpay: {
      contract: {
        id: 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7',
      },
      masterPublicKey: null,
    },
    featureFlags: {
      contract: {
        id: 'HY1keaRK5bcDmujNCQq5pxNyvAiHHpoHQgLN5ppiu4kh',
      },
      ownerId: 'H9sjb2bHG8t7gq5SwNdqzMWG7KR6sf3CbziFzthCkDD6',
      masterPublicKey: null,
    },
    sourcePath: null,
    masternodeRewardShares: {
      contract: {
        id: 'rUnsWrFu3PKyRMGk2mxmZVBPbQuZx2qtHeFjURoQevX',
      },
      masterPublicKey: null,
    },
  },
  externalIp: null,
  network: NETWORK_TESTNET,
  environment: 'production',
};
