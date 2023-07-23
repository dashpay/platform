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
  contractId: withdrawalsContractId,
} = require('@dashevo/withdrawals-contract/lib/systemIds');

const semver = require('semver');

const {
  NETWORK_TESTNET,
  PACKAGE_ROOT_DIR,
} = require('../../../constants');

const { version } = require('../../../../package.json');

const Config = require('../../Config');

/**
 * @param {HomeDir} homeDir
 * @returns {getBaseConfig}
 */
function getBaseConfigFactory(homeDir) {
  const prereleaseTag = semver.prerelease(version) === null ? '' : `-${semver.prerelease(version)[0]}`;
  const dockerImageVersion = `${semver.major(version)}.${semver.minor(version)}${prereleaseTag}`;

  /**
   * @typedef {function} getBaseConfig
   * @returns {Config}
   */
  function getBaseConfig() {
    const options = {
      description: 'base config for use as template',
      group: null,
      docker: {
        network: {
          subnet: '172.24.24.0/24',
          bindIp: '0.0.0.0',
        },
      },
      core: {
        docker: {
          image: 'dashpay/dashd:19.2.0',
        },
        p2p: {
          port: 9999,
          seeds: [],
        },
        rpc: {
          port: 9998,
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
            image: 'dashpay/sentinel:1.7.3',
          },
        },
        devnet: {
          name: null,
          minimumDifficultyBlocks: 0,
          powTargetSpacing: 150,
        },
        log: {
          file: {
            categories: [],
            path: homeDir.joinPath('logs', 'base', 'core.log'),
          },
        },
        logIps: 0,
        indexes: true,
      },
      platform: {
        dapi: {
          envoy: {
            docker: {
              image: 'dashpay/envoy:1.22.11',
            },
            http: {
              port: 443,
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
              image: `dashpay/dapi:${dockerImageVersion}`,
              build: {
                enabled: false,
                context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
                dockerFile: path.join(PACKAGE_ROOT_DIR, '..', 'dapi', 'Dockerfile'),
                target: '',
              },
            },
          },
        },
        drive: {
          abci: {
            docker: {
              image: `dashpay/drive:${dockerImageVersion}`,
              build: {
                enabled: false,
                context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
                dockerFile: path.join(PACKAGE_ROOT_DIR, '..', 'js-drive', 'Dockerfile'),
                target: '',
              },
            },
            log: {
              stdout: {
                level: 'info',
              },
              prettyFile: {
                level: 'silent',
                path: homeDir.joinPath('logs', 'base', 'drive-pretty.log'),
              },
              jsonFile: {
                level: 'silent',
                path: homeDir.joinPath('logs', 'base', 'drive-json.log'),
              },
            },
            validatorSet: {
              llmqType: 4,
            },
          },
          tenderdash: {
            docker: {
              image: 'dashpay/tenderdash:0.11.2',
            },
            p2p: {
              port: 26656,
              persistentPeers: [],
              seeds: [],
            },
            rpc: {
              port: 26657,
            },
            pprof: {
              enabled: false,
              port: 6060,
            },
            consensus: {
              createEmptyBlocks: true,
              createEmptyBlocksInterval: '3m',
            },
            log: {
              level: 'debug',
              format: 'plain',
            },
            node: {
              id: null,
              key: null,
            },
            genesis: {
              consensus_params: {
                block: {
                  max_bytes: '22020096',
                  max_gas: '-1',
                  time_iota_ms: '5000',
                },
                evidence: {
                  max_age: '100000',
                  max_age_num_blocks: '100000',
                  max_age_duration: '172800000000000',
                },
                validator: {
                  pub_key_types: [
                    'bls12381',
                  ],
                },
                version: {
                  app_version: '1',
                },
              },
            },
            moniker: null,
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
        withdrawals: {
          contract: {
            id: withdrawalsContractId,
          },
          masterPublicKey: null,
          secondPublicKey: null,
        },
        enable: true,
      },
      dashmate: {
        helper: {
          docker: {
            build: {
              enabled: false,
              context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
              dockerFile: path.join(PACKAGE_ROOT_DIR, 'Dockerfile'),
              target: '',
            },
          },
          api: {
            enable: false,
            port: 9100,
          },
        },
      },
      externalIp: null,
      network: NETWORK_TESTNET,
      environment: 'production',
    };

    return new Config('base', options);
  }

  return getBaseConfig;
}

module.exports = getBaseConfigFactory;
