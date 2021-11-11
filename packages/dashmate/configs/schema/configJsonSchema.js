const { NETWORKS } = require('../../src/constants');

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
        image: {
          type: 'string',
          minLength: 1,
        },
        build: {
          type: 'object',
          properties: {
            path: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          additionalProperties: false,
          required: ['path'],
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
          enum: ['fatal', 'error', 'warn', 'info', 'debug', 'trace', 'silent'],
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
        sentinel: {
          type: 'object',
          properties: {
            docker: {
              $ref: '#/definitions/docker',
            },
          },
          required: ['docker'],
          additionalProperties: false,
        },
        devnetName: {
          type: ['string', 'null'],
          minLength: 1,
        },
        debug: {
          type: 'integer',
          enum: [0, 1],
        },
      },
      required: ['docker', 'p2p', 'rpc', 'spork', 'masternode', 'miner', 'devnetName', 'debug'],
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
                grpc: {
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
              },
              required: ['docker', 'http', 'grpc', 'rateLimiter'],
              additionalProperties: false,
            },
            api: {
              type: 'object',
              properties: {
                docker: {
                  $ref: '#/definitions/dockerBuild',
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
            mongodb: {
              type: 'object',
              properties: {
                docker: {
                  $ref: '#/definitions/docker',
                },
              },
              required: ['docker'],
              additionalProperties: false,
            },
            abci: {
              type: 'object',
              properties: {
                docker: {
                  $ref: '#/definitions/dockerBuild',
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
                      // https://github.com/dashevo/dashcore-lib/blob/286c33a9d29d33f05d874c47a9b33764a0be0cf1/lib/constants/index.js#L42-L57
                      enum: [1, 2, 3, 4, 100, 101, 102],
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
                      type: 'object',
                      properties: {
                        'abci-client': {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        blockchain: {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        consensus: {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        main: {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        mempool: {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        p2p: {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        'rpc-server': {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        state: {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        statesync: {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                        '*': {
                          $ref: '#/definitions/tenderdashLogModule',
                        },
                      },
                      minProperties: 1,
                      additionalProperties: false,
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
                nodeKey: {
                  type: 'object',
                },
                genesis: {
                  type: 'object',
                },
                nodeId: {
                  type: ['string', 'null'],
                },
              },
              required: ['docker', 'p2p', 'rpc', 'consensus', 'nodeKey', 'genesis', 'nodeId'],
              additionalProperties: false,
            },
          },
          required: ['mongodb', 'abci', 'tenderdash'],
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
                blockHeight: {
                  type: ['integer', 'null'],
                  minimum: 1,
                },
              },
              required: ['id', 'blockHeight'],
              additionalProperties: false,
            },
            ownerId: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          required: ['contract', 'ownerId'],
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
                blockHeight: {
                  type: ['integer', 'null'],
                  minimum: 1,
                },
              },
              required: ['id', 'blockHeight'],
              additionalProperties: false,
            },
          },
          required: ['contract'],
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
                blockHeight: {
                  type: ['integer', 'null'],
                  minimum: 1,
                },
              },
              required: ['id', 'blockHeight'],
              additionalProperties: false,
            },
            ownerId: {
              type: ['string', 'null'],
              minLength: 1,
            },
          },
          required: ['contract', 'ownerId'],
          additionalProperties: false,
        },
      },
      required: ['dapi', 'drive', 'dpns', 'dashpay', 'featureFlags'],
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
  required: ['description', 'group', 'core', 'externalIp', 'network', 'environment'],
  additionalProperties: false,
};
