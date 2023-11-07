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
} = require('../../src/constants');

const { version } = require('../../package.json');

const Config = require('../../src/config/Config');

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
          subnet: '0.0.0.0/0',
          bindIp: '0.0.0.0',
        },
        baseImage: {
          build: {
            enabled: false,
            context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
            dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
            target: '',
          },
        },
      },
      core: {
        docker: {
          image: 'dashpay/dashd:20.0.0-rc.2',
          commandArgs: [],
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
                dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
                target: 'dapi',
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
                dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
                target: 'drive-abci',
              },
            },
            logs: {
              stdout: {
                destination: 'stdout',
                level: 'info',
                format: 'compact',
                color: true,
              },
            },
            validatorSet: {
              llmqType: 4,
            },
            epochTime: 788400,
          },
          tenderdash: {
            mode: 'full',
            docker: {
              image: 'dashpay/tenderdash:0.13.3',
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
            metrics: {
              enabled: false,
              port: 26660,
            },
            consensus: {
              createEmptyBlocks: true,
              createEmptyBlocksInterval: '3m',
            },
            log: {
              level: 'info',
              format: 'plain',
              path: null,
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
              dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
              target: 'dashmate-helper',
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
