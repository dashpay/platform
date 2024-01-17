import path from 'path';

import DPNSContract from '@dashevo/dpns-contract/lib/systemIds.js';

import DashPayContract from '@dashevo/dashpay-contract/lib/systemIds.js';

import FeatureFlagsContract from '@dashevo/feature-flags-contract/lib/systemIds.js';

import MasternodeRewardSharesContract from '@dashevo/masternode-reward-shares-contract/lib/systemIds.js';

import WithdrawalsContract from '@dashevo/withdrawals-contract/lib/systemIds.js';

import semver from 'semver';

import fs from 'fs';
import {
  NETWORK_TESTNET, PACKAGE_ROOT_DIR,
} from '../../src/constants.js';
import Config from '../../src/config/Config.js';

const { contractId: dpnsContractId, ownerId: dpnsOwnerId } = DPNSContract;

const { contractId: dashpayContractId } = DashPayContract;

const { contractId: featureFlagsContractId, ownerId: featureFlagsOwnerId } = FeatureFlagsContract;
const { contractId: masternodeRewardSharesContractId } = MasternodeRewardSharesContract;
const { contractId: withdrawalsContractId } = WithdrawalsContract;

const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

/**
 * @param {HomeDir} homeDir
 * @returns {getBaseConfig}
 */
export default function getBaseConfigFactory(homeDir) {
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
        insight: {
          enabled: false,
          ui: {
            enabled: false,
            docker: {
              image: 'dashpay/insight:latest',
            },
          },
          docker: {
            image: 'dashpay/insight-api:latest',
          },
          port: 3001,
        },
        docker: {
          image: 'dashpay/dashd:20', commandArgs: [],
        },
        p2p: {
          host: '0.0.0.0',
          port: 9999,
          seeds: [],
        },
        rpc: {
          host: '127.0.0.1',
          port: 9998,
          user: 'dashrpc',
          password: 'rpcpassword',
          allowIps: ['127.0.0.1', '172.16.0.0/12', '192.168.0.0/16'],
        },
        spork: {
          address: null, privateKey: null,
        },
        masternode: {
          enable: true,
          operator: {
            privateKey: null,
          },
        },
        miner: {
          enable: false, interval: '2.5m', mediantime: null, address: null,
        },
        devnet: {
          name: null, minimumDifficultyBlocks: 0, powTargetSpacing: 150,
        },
        log: {
          file: {
            categories: [], path: homeDir.joinPath('logs', 'base', 'core.log'),
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
              host: '0.0.0.0',
              port: 443,
              connectTimeout: '5s',
              responseTimeout: '15s',
            },
            rateLimiter: {
              maxTokens: 300, tokensPerFill: 150, fillInterval: '60s', enabled: true,
            },
            ssl: {
              enabled: false,
              provider: 'zerossl',
              providerConfigs: {
                zerossl: {
                  apiKey: null, id: null,
                },
              },
            },
          },
          api: {
            docker: {
              image: `dashpay/dapi:${dockerImageVersion}`,
              deploy: {
                replicas: 1,
              },
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
                destination: 'stdout', level: 'info', format: 'compact', color: true,
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
              image: 'dashpay/tenderdash:fix-ordered-map',
            },
            p2p: {
              host: '0.0.0.0',
              port: 26656,
              persistentPeers: [],
              seeds: [],
              flushThrottleTimeout: '100ms',
              maxPacketMsgPayloadSize: 10240,
              sendRate: 5120000,
              recvRate: 5120000,
            },
            rpc: {
              host: '127.0.0.1',
              port: 26657,
              maxOpenConnections: 900,
            },
            pprof: {
              enabled: false, port: 6060,
            },
            metrics: {
              enabled: false,
              host: '127.0.0.1',
              port: 26660,
            },
            mempool: {
              size: 5000,
              maxTxsBytes: 1073741824,
            },
            consensus: {
              createEmptyBlocks: true,
              createEmptyBlocksInterval: '3m',
              peer: {
                gossipSleepDuration: '100ms',
                queryMaj23SleepDuration: '2s',
              },
              unsafeOverride: {
                propose: {
                  timeout: null,
                  delta: null,
                },
                vote: {
                  timeout: null,
                  delta: null,
                },
                commit: {
                  timeout: null,
                  bypass: null,
                },
              },
            },
            log: {
              level: 'info', format: 'plain', path: null,
            },
            node: {
              id: null, key: null,
            },
            genesis: {
              consensus_params: {
                block: {
                  max_bytes: '2097152', max_gas: '57631392000', time_iota_ms: '5000',
                },
                evidence: {
                  max_age: '100000',
                  max_age_num_blocks: '100000',
                  max_age_duration: '172800000000000',
                },
                validator: {
                  pub_key_types: ['bls12381'],
                },
                version: {
                  app_version: '1',
                },
                timeout: {
                  propose: '30000000000',
                  propose_delta: '1000000000',
                  vote: '2000000000',
                  vote_delta: '500000000',
                  commit: '1000000000',
                  bypass_commit_timeout: false,
                },
                synchrony: {
                  message_delay: '32000000000',
                  precision: '500000000',
                },
                abci: {
                  recheck_tx: true,
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
            enable: false, port: 9100,
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
