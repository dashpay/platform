const { NETWORKS } = require('../constants');

module.exports = {
  $schema: 'http://json-schema.org/draft-07/schema#',
  type: 'object',
  definitions: {
    docker: {
      type: 'object',
      properties: {
        image: {
          type: 'string',
          minLength: 1,
        },
      },
      required: ['image'],
      additionalProperties: false,
    },
    dockerBuild: {
      type: 'object',
      properties: {
        enabled: {
          type: 'boolean',
        },
        context: {
          type: ['string', 'null'],
          minLength: 1,
        },
        dockerFile: {
          type: ['string', 'null'],
          minLength: 1,
        },
        target: {
          type: ['string', 'null'],
        },
      },
      required: ['enabled', 'context', 'dockerFile', 'target'],
      additionalProperties: false,
    },
    dockerWithBuild: {
      type: 'object',
      properties: {
        image: {
          type: 'string',
          minLength: 1,
        },
        build: {
          $ref: '#/definitions/dockerBuild',
        },
      },
      required: ['image', 'build'],
      additionalProperties: false,
    },
    port: {
      type: 'integer',
      minimum: 0,
    },
    tenderdashNodeAddress: {
      type: 'object',
      properties: {
        id: {
          type: 'string',
          minLength: 1,
        },
        host: {
          type: 'string',
          minLength: 1,
        },
        port: {
          $ref: '#/definitions/port',
        },
      },
      required: ['id', 'host', 'port'],
      additionalProperties: false,
    },
    abciLogFile: {
      type: 'object',
      properties: {
        level: {
          type: 'string',
          enum: ['error', 'warn', 'info', 'debug', 'trace', 'silent'],
        },
        path: {
          type: 'string',
          minLength: 1,
        },
      },
      additionalProperties: false,
      required: ['level', 'path'],
    },
    tenderdashLogModule: {
      type: 'string',
      enum: ['debug', 'info', 'error'],
    },
  },
  properties: {
    description: {
      type: ['string', 'null'],
    },
    group: {
      type: ['string', 'null'],
    },
    docker: {
      type: 'object',
      properties: {
        network: {
          type: 'object',
          properties: {
            subnet: {
              type: 'string',
            },
            bindIp: {
              type: 'string',
              format: 'ipv4',
            },
          },
          additionalProperties: false,
          required: ['subnet', 'bindIp'],
        },
        baseImage: {
          type: 'object',
          properties: {
            build: {
              $ref: '#/definitions/dockerBuild',
            },
          },
          additionalProperties: false,
          required: ['build'],
        },
      },
      additionalProperties: false,
      required: ['network', 'baseImage'],
    },
    core: {
      type: 'object',
      properties: {
        docker: {
          $ref: '#/definitions/docker',
        },
        p2p: {
          type: 'object',
          properties: {
            port: {
              $ref: '#/definitions/port',
            },
            seeds: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  host: {
                    type: 'string',
                    minLength: 1,
                  },
                  port: {
                    $ref: '#/definitions/port',
                  },
                },
                required: ['host', 'port'],
                additionalProperties: false,
              },
            },
          },
          required: ['port', 'seeds'],
          additionalProperties: false,
        },
        rpc: {
          type: 'object',
          properties: {
            port: {
              $ref: '#/definitions/port',
            },
            user: {
              type: 'string',
              minLength: 1,
            },
            password: {
              type: 'string',
              minLength: 1,
            },
            allowIps: {
              type: 'array',
              items: {
                type: 'string',
              },
            },
          },
          required: ['port', 'user', 'password'],
          additionalProperties: false,
        },
        spork: {
          type: 'object',
          properties: {
            address: {
              type: ['string', 'null'],
            },
            privateKey: {
              type: ['string', 'null'],
            },
          },
          required: ['address', 'privateKey'],
          additionalProperties: false,
        },
        masternode: {
          type: 'object',
          properties: {
            enable: {
              type: 'boolean',
            },
            operator: {
              type: 'object',
              properties: {
                privateKey: {
                  type: ['string', 'null'],
                },
              },
              required: ['privateKey'],
              additionalProperties: false,
            },
          },
          required: ['enable', 'operator'],
          additionalProperties: false,
        },
        miner: {
          type: 'object',
          properties: {
            enable: {
              type: 'boolean',
            },
            interval: {
              type: 'string',
              pattern: '^[0-9]+(.[0-9]+)?(m|s|h)$',
            },
            mediantime: {
              type: ['integer', 'null'],
              minimum: 0,
            },
            address: {
              type: ['string', 'null'],
            },
          },
          required: ['enable', 'interval', 'mediantime', 'address'],
          additionalProperties: false,
        },
        devnet: {
          type: 'object',
          properties: {
            name: {
              type: ['string', 'null'],
              minLength: 1,
            },
            minimumDifficultyBlocks: {
              type: 'integer',
              minimum: 0,
            },
            powTargetSpacing: {
              type: 'integer',
              minimum: 1,
            },
          },
          additionalProperties: false,
          required: ['name', 'minimumDifficultyBlocks', 'powTargetSpacing'],
        },
        log: {
          type: 'object',
          properties: {
            file: {
              type: 'object',
              properties: {
                categories: {
                  type: 'array',
                  uniqueItems: true,
                  items: {
                    type: 'string',
                    enum: ['all', 'net', 'tor', 'mempool', 'http', 'bench', 'zmq', 'walletdb', 'rpc', 'estimatefee',
                      'addrman', 'selectcoins', 'reindex', 'cmpctblock', 'rand', 'prune', 'proxy', 'mempoolrej',
                      'libevent', 'coindb', 'qt', 'leveldb', 'chainlocks', 'gobject', 'instantsend', 'llmq',
                      'llmq-dkg', 'llmq-sigs', 'mnpayments', 'mnsync', 'coinjoin', 'spork', 'netconn',
                    ],
                  },
                },
                path: {
                  type: 'string',
                  minLength: 1,
                },
              },
              additionalProperties: false,
              required: ['categories', 'path'],
            },
          },
          additionalProperties: false,
          required: ['file'],
        },
        logIps: {
          type: 'integer',
          enum: [0, 1],
        },
        indexes: {
          type: 'boolean',
        },
      },
      required: ['docker', 'p2p', 'rpc', 'spork', 'masternode', 'miner', 'devnet', 'log',
        'logIps', 'indexes'],
      additionalProperties: false,
    },
    platform: {
      type: 'object',
      properties: {
        dapi: {
          type: 'object',
          properties: {
            envoy: {
              type: 'object',
              properties: {
                docker: {
                  $ref: '#/definitions/docker',
                },
                http: {
                  type: 'object',
                  properties: {
                    port: {
                      $ref: '#/definitions/port',
                    },
                  },
                  required: ['port'],
                  additionalProperties: false,
                },
                rateLimiter: {
                  type: 'object',
                  properties: {
                    maxTokens: {
                      type: 'integer',
                      minimum: 0,
                    },
                    tokensPerFill: {
                      type: 'integer',
                      minimum: 0,
                    },
                    fillInterval: {
                      type: 'string',
                      pattern: '^[0-9]+(ms|s|m|h)$',
                    },
                    enabled: {
                      type: 'boolean',
                    },
                  },
                  required: ['enabled', 'fillInterval', 'tokensPerFill', 'maxTokens'],
                  additionalProperties: false,
                },
                ssl: {
                  type: 'object',
                  properties: {
                    enabled: {
                      type: 'boolean',
                    },
                    provider: {
                      type: 'string',
                      enum: ['zerossl', 'self-signed', 'file'],
                    },
                    providerConfigs: {
                      type: 'object',
                      properties: {
                        zerossl: {
                          type: ['object'],
                          properties: {
                            apiKey: {
                              type: ['string', 'null'],
                              minLength: 32,
                            },
                            id: {
                              type: ['string', 'null'],
                              minLength: 32,
                            },
                          },
                          required: ['apiKey', 'id'],
                          additionalProperties: false,
                        },
                      },
                    },
                  },
                  required: ['provider', 'providerConfigs', 'enabled'],
                  additionalProperties: false,
                },
              },
              required: ['docker', 'http', 'rateLimiter', 'ssl'],
              additionalProperties: false,
            },
            api: {
              type: 'object',
              properties: {
                docker: {
                  $ref: '#/definitions/dockerWithBuild',
                },
              },
              required: ['docker'],
              additionalProperties: false,
            },
          },
          required: ['envoy', 'api'],
          additionalProperties: false,
        },
        drive: {
          type: 'object',
          properties: {
            abci: {
              type: 'object',
              properties: {
                docker: {
                  $ref: '#/definitions/dockerWithBuild',
                },
                log: {
                  type: 'object',
                  properties: {
                    stdout: {
                      type: 'object',
                      properties: {
                        level: {
                          $ref: '#/definitions/abciLogFile/properties/level',
                        },
                      },
                      additionalProperties: false,
                      required: ['level'],
                    },
                    prettyFile: {
                      $ref: '#/definitions/abciLogFile',
                    },
                    jsonFile: {
                      $ref: '#/definitions/abciLogFile',
                    },
                  },
                  additionalProperties: false,
                  required: ['stdout', 'prettyFile', 'jsonFile'],
                },
                validatorSet: {
                  type: 'object',
                  properties: {
                    llmqType: {
                      type: 'number',
                      // https://github.com/dashpay/dashcore-lib/blob/843176fed9fc81feae43ccf319d99e2dd942fe1f/lib/constants/index.js#L50-L99
                      enum: [1, 2, 3, 4, 5, 6, 100, 101, 102, 103, 104, 105, 106, 107],
                    },
                  },
                  additionalProperties: false,
                  required: ['llmqType'],
                },
              },
              additionalProperties: false,
              required: ['docker', 'log', 'validatorSet'],
            },
            tenderdash: {
              type: 'object',
              properties: {
                mode: {
                  type: 'string',
                  enum: ['full', 'validator', 'seed'],
                },
                docker: {
                  $ref: '#/definitions/docker',
                },
                p2p: {
                  type: 'object',
                  properties: {
                    port: {
                      $ref: '#/definitions/port',
                    },
                    persistentPeers: {
                      type: 'array',
                      items: {
                        $ref: '#/definitions/tenderdashNodeAddress',
                      },
                    },
                    seeds: {
                      type: 'array',
                      items: {
                        $ref: '#/definitions/tenderdashNodeAddress',
                      },
                    },
                  },
                  required: ['port', 'persistentPeers', 'seeds'],
                  additionalProperties: false,
                },
                consensus: {
                  type: 'object',
                  properties: {
                    createEmptyBlocks: {
                      type: 'boolean',
                    },
                    createEmptyBlocksInterval: {
                      type: 'string',
                      pattern: '^[0-9]+(.[0-9]+)?(m|s|h)$',
                    },
                  },
                  additionalProperties: false,
                  required: ['createEmptyBlocks', 'createEmptyBlocksInterval'],
                },
                log: {
                  type: 'object',
                  properties: {
                    level: {
                      type: 'string',
                      enum: ['trace', 'debug', 'info', 'warn', 'error'],
                    },
                    format: {
                      type: 'string',
                      enum: ['plain', 'json'],
                    },
                  },
                  required: ['level', 'format'],
                  additionalProperties: false,
                },
                rpc: {
                  type: 'object',
                  properties: {
                    port: {
                      $ref: '#/definitions/port',
                    },
                  },
                  required: ['port'],
                  additionalProperties: false,
                },
                pprof: {
                  type: 'object',
                  properties: {
                    enabled: {
                      type: 'boolean',
                    },
                    port: {
                      $ref: '#/definitions/port',
                    },
                  },
                  required: ['enabled', 'port'],
                  additionalProperties: false,
                },
                metrics: {
                  description: 'Prometheus metrics',
                  type: 'object',
                  properties: {
                    enabled: {
                      type: 'boolean',
                    },
                    port: {
                      $ref: '#/definitions/port',
                    },
                  },
                  required: ['enabled', 'port'],
                  additionalProperties: false,
                },
                node: {
                  type: 'object',
                  properties: {
                    id: {
                      type: ['string', 'null'],
                    },
                    key: {
                      type: ['string', 'null'],
                    },
                  },
                  additionalProperties: false,
                },
                moniker: {
                  type: ['string', 'null'],
                },
                genesis: {
                  type: 'object',
                },
              },
              required: ['mode', 'docker', 'p2p', 'consensus', 'log', 'rpc', 'pprof', 'node', 'moniker', 'genesis', 'metrics'],
              additionalProperties: false,
            },
          },
          required: ['abci', 'tenderdash'],
          additionalProperties: false,
        },
        dpns: {
          type: 'object',
          properties: {
            contract: {
              type: 'object',
              properties: {
                id: {
                  type: ['string', 'null'],
                  minLength: 1,
                },
              },
              required: ['id'],
              additionalProperties: false,
            },
            ownerId: {
              type: ['string', 'null'],
              minLength: 1,
            },
            masterPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
            secondPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          required: ['contract', 'ownerId', 'masterPublicKey', 'secondPublicKey'],
          additionalProperties: false,
        },
        dashpay: {
          type: 'object',
          properties: {
            contract: {
              type: 'object',
              properties: {
                id: {
                  type: ['string', 'null'],
                  minLength: 1,
                },
              },
              required: ['id'],
              additionalProperties: false,
            },
            masterPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
            secondPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          required: ['contract', 'masterPublicKey', 'secondPublicKey'],
          additionalProperties: false,
        },
        featureFlags: {
          type: 'object',
          properties: {
            contract: {
              type: 'object',
              properties: {
                id: {
                  type: ['string', 'null'],
                  minLength: 1,
                },
              },
              required: ['id'],
              additionalProperties: false,
            },
            ownerId: {
              type: ['string', 'null'],
              minLength: 1,
            },
            masterPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
            secondPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          required: ['contract', 'ownerId', 'masterPublicKey', 'secondPublicKey'],
          additionalProperties: false,
        },
        sourcePath: {
          type: ['string', 'null'],
          minLength: 1,
        },
        masternodeRewardShares: {
          type: 'object',
          properties: {
            contract: {
              type: 'object',
              properties: {
                id: {
                  type: ['string', 'null'],
                  minLength: 1,
                },
              },
              required: ['id'],
              additionalProperties: false,
            },
            masterPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
            secondPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          required: ['contract', 'masterPublicKey', 'secondPublicKey'],
          additionalProperties: false,
        },
        withdrawals: {
          type: 'object',
          properties: {
            contract: {
              type: 'object',
              properties: {
                id: {
                  type: ['string', 'null'],
                  minLength: 1,
                },
              },
              required: ['id'],
              additionalProperties: false,
            },
            masterPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
            secondPublicKey: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          required: ['contract', 'masterPublicKey', 'secondPublicKey'],
          additionalProperties: false,
        },
        enable: {
          type: 'boolean',
        },
      },
      required: ['dapi', 'drive', 'dpns', 'dashpay', 'featureFlags', 'sourcePath', 'masternodeRewardShares', 'withdrawals', 'enable'],
      additionalProperties: false,
    },
    dashmate: {
      type: 'object',
      properties: {
        helper: {
          type: 'object',
          properties: {
            docker: {
              type: 'object',
              properties: {
                build: {
                  $ref: '#/definitions/dockerBuild',
                },
              },
              required: ['build'],
              additionalProperties: false,
            },
            api: {
              type: 'object',
              properties: {
                enable: {
                  type: 'boolean',
                },
                port: {
                  $ref: '#/definitions/port',
                },
              },
              required: ['enable', 'port'],
              additionalProperties: false,
            },
          },
          required: ['docker', 'api'],
          additionalProperties: false,
        },
      },
      required: ['helper'],
      additionalProperties: false,
    },
    externalIp: {
      type: ['string', 'null'],
      format: 'ipv4',
    },
    network: {
      type: 'string',
      enum: NETWORKS,
    },
    environment: {
      type: 'string',
      enum: ['development', 'production'],
    },
  },
  required: ['description', 'group', 'docker', 'core', 'externalIp', 'network', 'environment'],
  additionalProperties: false,
};
