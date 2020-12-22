const NETWORKS = require('../networks');

module.exports = {
  $schema: 'http://json-schema.org/draft-07/schema#',
  type: 'object',
  definitions: {
    docker: {
      type: 'object',
      properties: {
        docker: {
          type: 'object',
          properties: {
            image: {
              type: 'string',
            },
          },
          required: ['image'],
          additionalProperties: false,
        },
      },
      required: ['docker'],
    },
    port: {
      type: 'integer',
      minimum: 0,
    },
  },
  properties: {
    description: {
      type: ['string', 'null'],
    },
    core: {
      type: 'object',
      properties: {
        docker: {
          $ref: '#/definitions/docker/properties/docker',
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
            address: {
              type: ['string', 'null'],
            },
          },
          required: ['enable', 'interval', 'address'],
          additionalProperties: false,
        },
        devnetName: {
          type: ['string', 'null'],
          minLength: 1,
        },
      },
      required: ['docker', 'p2p', 'rpc', 'spork', 'masternode', 'miner', 'devnetName'],
      additionalProperties: false,
    },
    platform: {
      type: 'object',
      properties: {
        dapi: {
          type: 'object',
          properties: {
            envoy: {
              $ref: '#/definitions/docker',
            },
            nginx: {
              properties: {
                docker: {
                  $ref: '#/definitions/docker/properties/docker',
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
                    enable: {
                      type: 'boolean',
                    },
                    rate: {
                      type: 'integer',
                      minimum: 0,
                    },
                    burst: {
                      type: 'integer',
                      minimum: 0,
                    },
                  },
                  required: ['enable', 'rate', 'burst'],
                  additionalProperties: false,
                },
              },
              required: ['docker', 'http', 'grpc', 'rateLimiter'],
              additionalProperties: false,
            },
            api: {
              $ref: '#/definitions/docker',
            },
            insight: {
              $ref: '#/definitions/docker',
            },
          },
          required: ['envoy', 'nginx', 'api', 'insight'],
          additionalProperties: false,
        },
        drive: {
          type: 'object',
          properties: {
            mongodb: {
              $ref: '#/definitions/docker',
            },
            abci: {
              properties: {
                docker: {
                  $ref: '#/definitions/docker/properties/docker',
                },
                log: {
                  properties: {
                    level: {
                      type: 'string',
                      enum: ['fatal', 'error', 'warn', 'info', 'debug', 'trace', 'silent'],
                    },
                  },
                  additionalProperties: false,
                  required: ['level'],
                },
              },
              additionalProperties: false,
              required: ['docker', 'log'],
            },
            tenderdash: {
              properties: {
                docker: {
                  $ref: '#/definitions/docker/properties/docker',
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
                    },
                  },
                  required: ['port', 'persistentPeers'],
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
                validatorKey: {
                  type: 'object',
                },
                nodeKey: {
                  type: 'object',
                },
                genesis: {
                  type: 'object',
                },
              },
              required: ['docker', 'p2p', 'rpc', 'validatorKey', 'nodeKey', 'genesis'],
              additionalProperties: false,
            },
            skipAssetLockConfirmationValidation: {
              type: 'boolean',
            },
          },
          required: ['mongodb', 'abci', 'tenderdash', 'skipAssetLockConfirmationValidation'],
          additionalProperties: false,
        },
        dpns: {
          type: 'object',
          properties: {
            contract: {
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
      required: ['dapi', 'drive', 'dpns'],
      additionalProperties: false,
    },
    externalIp: {
      type: ['string', 'null'],
      format: 'ipv4',
    },
    network: {
      type: 'string',
      enum: Object.values(NETWORKS),
    },
    compose: {
      type: 'object',
      properties: {
        file: {
          type: 'string',
        },
      },
      required: ['file'],
      additionalProperties: false,
    },
    environment: {
      type: 'string',
      enum: ['development', 'production'],
    },
  },
  required: ['description', 'core', 'platform', 'externalIp', 'network', 'compose', 'environment'],
  additionalProperties: false,
};
