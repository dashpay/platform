import path from 'path';
import {PACKAGE_ROOT_DIR} from '../../constants';

export function getConfigFileDataV0250() {
  return {
    configFormatVersion: '0.25.0',
    defaultConfigName: null,
    defaultGroupName: null,
    configs: {
      base: {
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
            image: 'dashpay/dashd:20.0.0-beta.4',
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
              path: '/Users/dashmate/.dashmate/logs/base/core.log',
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
                image: 'dashpay/dapi:0.25',
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
                image: 'dashpay/drive:0.25',
                build: {
                  enabled: false,
                  context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
                  dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
                  target: 'drive-abci',
                },
              },
              log: {
                stdout: {
                  level: 'info',
                },
                prettyFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/base/drive-pretty.log',
                },
                jsonFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/base/drive-json.log',
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
                image: 'dashpay/tenderdash:0.13.2',
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
                level: 'debug',
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
              id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            },
            ownerId: '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
            masterPublicKey: null,
            secondPublicKey: null,
          },
          dashpay: {
            contract: {
              id: 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7',
            },
            masterPublicKey: null,
            secondPublicKey: null,
          },
          featureFlags: {
            contract: {
              id: 'HY1keaRK5bcDmujNCQq5pxNyvAiHHpoHQgLN5ppiu4kh',
            },
            ownerId: 'H9sjb2bHG8t7gq5SwNdqzMWG7KR6sf3CbziFzthCkDD6',
            masterPublicKey: null,
            secondPublicKey: null,
          },
          sourcePath: null,
          masternodeRewardShares: {
            contract: {
              id: 'rUnsWrFu3PKyRMGk2mxmZVBPbQuZx2qtHeFjURoQevX',
            },
            masterPublicKey: null,
            secondPublicKey: null,
          },
          withdrawals: {
            contract: {
              id: '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6',
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
        network: 'testnet',
        environment: 'production',
      },
      local: {
        description: 'template for local configs',
        group: null,
        docker: {
          network: {
            subnet: '172.24.24.0/24',
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
            image: 'dashpay/dashd:20.0.0-beta.4',
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
          devnet: {
            name: null,
            minimumDifficultyBlocks: 0,
            powTargetSpacing: 150,
          },
          log: {
            file: {
              categories: [],
              path: '/Users/dashmate/.dashmate/logs/base/core.log',
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
                port: 2443,
              },
              rateLimiter: {
                maxTokens: 300,
                tokensPerFill: 150,
                fillInterval: '60s',
                enabled: false,
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
                image: 'dashpay/dapi:0.25',
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
                image: 'dashpay/drive:0.25',
                build: {
                  enabled: false,
                  context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
                  dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
                  target: 'drive-abci',
                },
              },
              log: {
                stdout: {
                  level: 'info',
                },
                prettyFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/base/drive-pretty.log',
                },
                jsonFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/base/drive-json.log',
                },
              },
              validatorSet: {
                llmqType: 106,
              },
              epochTime: 788400,
            },
            tenderdash: {
              mode: 'full',
              docker: {
                image: 'dashpay/tenderdash:0.13.2',
              },
              p2p: {
                port: 46656,
                persistentPeers: [],
                seeds: [],
              },
              rpc: {
                port: 46657,
              },
              pprof: {
                enabled: false,
                port: 46060,
              },
              metrics: {
                enabled: false,
                port: 46660,
              },
              consensus: {
                createEmptyBlocks: true,
                createEmptyBlocksInterval: '3m',
              },
              log: {
                level: 'debug',
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
              id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            },
            ownerId: '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
            masterPublicKey: null,
            secondPublicKey: null,
          },
          dashpay: {
            contract: {
              id: 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7',
            },
            masterPublicKey: null,
            secondPublicKey: null,
          },
          featureFlags: {
            contract: {
              id: 'HY1keaRK5bcDmujNCQq5pxNyvAiHHpoHQgLN5ppiu4kh',
            },
            ownerId: 'H9sjb2bHG8t7gq5SwNdqzMWG7KR6sf3CbziFzthCkDD6',
            masterPublicKey: null,
            secondPublicKey: null,
          },
          sourcePath: null,
          masternodeRewardShares: {
            contract: {
              id: 'rUnsWrFu3PKyRMGk2mxmZVBPbQuZx2qtHeFjURoQevX',
            },
            masterPublicKey: null,
            secondPublicKey: null,
          },
          withdrawals: {
            contract: {
              id: '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6',
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
        network: 'local',
        environment: 'development',
      },
      testnet: {
        description: 'node with testnet configuration',
        group: null,
        docker: {
          network: {
            subnet: '172.25.24.0/24',
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
            image: 'dashpay/dashd:20.0.0-beta.2',
          },
          p2p: {
            port: 19999,
            seeds: [],
          },
          rpc: {
            port: 19998,
            user: 'dashrpc',
            password: 'rpcpassword',
            allowIps: [
              '127.0.0.1',
              '172.16.0.0/12',
              '192.168.0.0/16',
            ],
          },
          spork: {
            address: 'yjPtiKh2uwk3bDutTEA2q9mCtXyiZRWn55',
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
              path: '/Users/dashmate/.dashmate/logs/testnet/core.log',
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
                port: 1443,
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
                image: 'dashpay/dapi:0.25',
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
                image: 'dashpay/drive:0.25',
                build: {
                  enabled: false,
                  context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
                  dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
                  target: 'drive-abci',
                },
              },
              log: {
                stdout: {
                  level: 'info',
                },
                prettyFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/testnet/drive-pretty.log',
                },
                jsonFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/testnet/drive-json.log',
                },
              },
              validatorSet: {
                llmqType: 6,
              },
              epochTime: 788400,
            },
            tenderdash: {
              mode: 'full',
              docker: {
                image: 'dashpay/tenderdash:0.13.2',
              },
              p2p: {
                port: 36656,
                persistentPeers: [],
                seeds: [
                  {
                    id: '74907790a03b51ac062c8a1453dafd72a08668a3',
                    host: '35.166.35.250',
                    port: 36656,
                  },
                  {
                    id: '2006632eb20e670923d13d4f53abc24468eaad4d',
                    host: '35.92.64.72',
                    port: 36656,
                  },
                ],
              },
              rpc: {
                port: 36657,
              },
              pprof: {
                enabled: false,
                port: 36060,
              },
              metrics: {
                enabled: false,
                port: 36660,
              },
              consensus: {
                createEmptyBlocks: true,
                createEmptyBlocksInterval: '3m',
              },
              log: {
                level: 'debug',
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
                  timeout: {
                    propose: '50000000000',
                    propose_delta: '10000000000',
                    vote: '500000000',
                    vote_delta: '100000000',
                    commit: '1000000000',
                    bypass_commit_timeout: false,
                  },
                },
                genesis_time: '2023-10-10T10:43:20.921Z',
                chain_id: 'dash-testnet-26',
                initial_core_chain_locked_height: 921380,
                validator_quorum_type: 6,
              },
              moniker: null,
            },
          },
          dpns: {
            contract: {
              id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            },
            ownerId: '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
            masterPublicKey: '02c8b4747b528cac5fddf7a6cc63702ee04ed7d1332904e08510343ea00dce546a',
            secondPublicKey: '0201ee28f84f5485390567e939c2b586010b63a69ec92cab535dc96a8c71913602',
          },
          dashpay: {
            contract: {
              id: 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7',
            },
            masterPublicKey: '02d4dcce3f0a8d2936ce26df4d255fd2835b629b73eea39d4b2778096b91e77946',
            secondPublicKey: '03699c8b4ebf1696c92e9ec605a02a38f6f9cec47d13fb584fdad779e936e20ccb',
          },
          featureFlags: {
            contract: {
              id: 'HY1keaRK5bcDmujNCQq5pxNyvAiHHpoHQgLN5ppiu4kh',
            },
            ownerId: 'H9sjb2bHG8t7gq5SwNdqzMWG7KR6sf3CbziFzthCkDD6',
            masterPublicKey: '029cf2232549de08c114c19763309cb067688e21e310ac07458b59c2c026be7234',
            secondPublicKey: '02a2abb50c03ae9f778f08a93849ba334a82e625153720dd5ef14e564b78b414e5',
          },
          sourcePath: null,
          masternodeRewardShares: {
            contract: {
              id: 'rUnsWrFu3PKyRMGk2mxmZVBPbQuZx2qtHeFjURoQevX',
            },
            masterPublicKey: '0319d795c0795bc8678bd0e58cfc7a4ad75c8e1797537728e7e8de8b9acc2bae2b',
            secondPublicKey: '033756572938aaad752158b858ad38511c6edff4c79cf8462f70baa25fc6e8a616',
          },
          withdrawals: {
            contract: {
              id: '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6',
            },
            masterPublicKey: '032f79d1d9d6e652599d3315d30306b1277fbf588e32e383aef0a59749547d47b7',
            secondPublicKey: '03eebbe3dc3721603a0b5a13441f214550ffa7d035b7dea9f1911de0f63ddac58d',
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
        network: 'testnet',
        environment: 'production',
      },
      mainnet: {
        description: 'node with mainnet configuration',
        group: null,
        docker: {
          network: {
            subnet: '172.26.24.0/24',
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
            image: 'dashpay/dashd:19.3.0',
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
              path: '/Users/dashmate/.dashmate/logs/mainnet/core.log',
            },
          },
          logIps: 0,
          indexes: false,
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
                image: 'dashpay/dapi:0.25',
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
                image: 'dashpay/drive:0.25',
                build: {
                  enabled: false,
                  context: path.join(PACKAGE_ROOT_DIR, '..', '..'),
                  dockerFile: path.join(PACKAGE_ROOT_DIR, '..', '..', 'Dockerfile'),
                  target: 'drive-abci',
                },
              },
              log: {
                stdout: {
                  level: 'info',
                },
                prettyFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/base/drive-pretty.log',
                },
                jsonFile: {
                  level: 'silent',
                  path: '/Users/dashmate/.dashmate/logs/base/drive-json.log',
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
                image: 'dashpay/tenderdash:0.13.2',
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
                level: 'debug',
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
              id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            },
            ownerId: '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
            masterPublicKey: null,
            secondPublicKey: null,
          },
          dashpay: {
            contract: {
              id: 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7',
            },
            masterPublicKey: null,
            secondPublicKey: null,
          },
          featureFlags: {
            contract: {
              id: 'HY1keaRK5bcDmujNCQq5pxNyvAiHHpoHQgLN5ppiu4kh',
            },
            ownerId: 'H9sjb2bHG8t7gq5SwNdqzMWG7KR6sf3CbziFzthCkDD6',
            masterPublicKey: null,
            secondPublicKey: null,
          },
          sourcePath: null,
          masternodeRewardShares: {
            contract: {
              id: 'rUnsWrFu3PKyRMGk2mxmZVBPbQuZx2qtHeFjURoQevX',
            },
            masterPublicKey: null,
            secondPublicKey: null,
          },
          withdrawals: {
            contract: {
              id: '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6',
            },
            masterPublicKey: null,
            secondPublicKey: null,
          },
          enable: false,
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
        network: 'mainnet',
        environment: 'production',
      },
    },
    projectId: 'ad3bc757',
  };
}
