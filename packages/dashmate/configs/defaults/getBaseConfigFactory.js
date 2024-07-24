import path from 'path';
import semver from 'semver';

import fs from 'fs';
import Config from '../../src/config/Config.js';
import {
  NETWORK_MAINNET,
  PACKAGE_ROOT_DIR,
} from '../../src/constants.js';

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
          image: 'dashpay/dashd:20',
          commandArgs: [],
        },
        p2p: {
          host: '0.0.0.0',
          port: 9999,
          seeds: [],
        },
        rpc: {
          host: '127.0.0.1',
          port: 9998,
          users: {
            dashmate: {
              password: 'rpcpassword',
              whitelist: null,
              lowPriority: false,
            },
            dapi: {
              password: 'rpcpassword',
              whitelist: [
                'getbestblockhash', 'getblockhash', 'sendrawtransaction', 'getrawtransaction',
                'getblockstats', 'getmerkleblocks', 'getrawtransactionmulti', 'getrawmempool',
                'getblockcount', 'getbestchainlock', 'getblock', 'getblockheader', 'getblockheaders',
                'protxdiff', 'getnetworkinfo', 'getblockchaininfo', 'mnsyncstatus', 'masternodestatus',
              ],
              lowPriority: true,
            },
            drive_consensus: {
              password: 'rpcpassword',
              whitelist: [
                'getbestchainlock', 'getblockchaininfo', 'getrawtransaction', 'submitchainlock',
                'verifychainlock', 'protxlistdiff', 'quorumlistextended', 'quoruminfo',
                'getassetunlockstatuses', 'sendrawtransaction', 'mnsyncstatus',
              ],
              lowPriority: false,
            },
            drive_check_tx: {
              password: 'rpcpassword',
              whitelist: ['getrawtransaction'],
              lowPriority: true,
            },
            tenderdash: {
              password: 'rpcpassword',
              whitelist: [
                'quoruminfo', 'quorumverify', 'quorumplatformsign', 'masternodestatus', 'masternodelist',
                'ping', 'getnetworkinfo',
              ],
              lowPriority: false,
            },
          },
          allowIps: ['127.0.0.1', '172.16.0.0/12', '192.168.0.0/16'],
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
          llmq: {
            chainLocks: 'llmq_devnet',
            instantSend: 'llmq_devnet_dip0024',
            platform: 'llmq_devnet_platform',
            mnhf: 'llmq_devnet',
          },
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
        gateway: {
          docker: {
            image: 'dashpay/envoy:1.30.2-impr.1',
          },
          maxConnections: 1000,
          maxHeapSizeInBytes: 125000000, // 1 Gb
          upstreams: {
            driveGrpc: {
              maxRequests: 100,
            },
            dapiApi: {
              maxRequests: 100,
            },
            dapiCoreStreams: {
              maxRequests: 100,
            },
            dapiJsonRpc: {
              maxRequests: 100,
            },
          },
          metrics: {
            enabled: false,
            host: '127.0.0.1',
            port: 9090,
          },
          admin: {
            enabled: false,
            host: '127.0.0.1',
            port: 9901,
          },
          listeners: {
            dapiAndDrive: {
              http2: {
                maxConcurrentStreams: 10,
              },
              host: '0.0.0.0',
              port: 443,
            },
          },
          log: {
            level: 'info',
            accessLogs: [
              {
                type: 'stdout',
                format: 'text',
                template: null,
              },
            ],
          },
          rateLimiter: {
            docker: {
              image: 'envoyproxy/ratelimit:3fcc3609',
            },
            metrics: {
              enabled: false,
              docker: {
                image: 'prom/statsd-exporter:v0.26.1',
              },
              host: '127.0.0.1',
              port: 9102,
            },
            unit: 'minute',
            requestsPerUnit: 150,
            blacklist: [],
            whitelist: [],
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
        dapi: {
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
                destination: 'stdout',
                level: 'info',
                format: 'compact',
                color: true,
              },
            },
            tokioConsole: {
              enabled: false,
              host: '127.0.0.1',
              port: 6669,
              retention: 60 * 3,
            },
            validatorSet: {
              quorum: {
                llmqType: 4,
                dkgInterval: 24,
                activeSigners: 24,
                rotation: false,
              },
            },
            chainLock: {
              quorum: {
                llmqType: 2,
                dkgInterval: 288,
                activeSigners: 4,
                rotation: false,
              },
            },
            instantLock: {
              quorum: {
                llmqType: 5,
                dkgInterval: 288,
                activeSigners: 32,
                rotation: true,
              },
            },
            metrics: {
              enabled: false,
              host: '127.0.0.1',
              port: 29090,
            },
            grovedbVisualizer: {
              enabled: false,
              host: '127.0.0.1',
              port: 8083,
            },
            epochTime: 788400,
          },
          tenderdash: {
            mode: 'full',
            docker: {
              image: 'dashpay/tenderdash:1.1.0-dev.2',
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
              timeoutBroadcastTx: 0,
            },
            pprof: {
              enabled: false,
              port: 6060,
            },
            metrics: {
              enabled: false,
              host: '127.0.0.1',
              port: 26660,
            },
            mempool: {
              cacheSize: 15000,
              size: 5000,
              maxTxsBytes: 1073741824,
              timeoutCheckTx: '0',
              txEnqueueTimeout: '0',
              txSendRateLimit: 0,
              txRecvRateLimit: 0,
              maxConcurrentCheckTx: 250,
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
                  max_bytes: '2097152',
                  max_gas: '57631392000',
                  time_iota_ms: '5000',
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
        sourcePath: null,
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
      network: NETWORK_MAINNET,
      environment: 'production',
    };

    return new Config('base', options);
  }

  return getBaseConfig;
}
