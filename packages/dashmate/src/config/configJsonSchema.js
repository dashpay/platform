import { NETWORKS } from '../constants.js';

export default {
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
    host: {
      type: 'string',
      minLength: 1,
      format: 'ipv4',
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
    duration: {
      type: 'string',
      pattern: '^0|([0-9]+(\\.[0-9]+)?(ms|m|s|h))$',
    },
    optionalDuration: {
      type: ['null', 'string'],
      pattern: '^[0-9]+(\\.[0-9]+)?(ms|m|s|h)$',
    },
    durationInSeconds: {
      type: 'string',
      pattern: '^[0-9]+(\\.[0-9]+)?s$',
    },
    enabledHostPort: {
      type: 'object',
      properties: {
        enabled: {
          type: 'boolean',
        },
        host: {
          $ref: '#/definitions/host',
        },
        port: {
          $ref: '#/definitions/port',
        },
      },
      additionalProperties: false,
      required: ['enabled', 'host', 'port'],
    },
    quorum: {
      type: 'object',
      properties: {
        llmqType: {
          type: 'integer',
          enum: [1, 2, 3, 4, 5, 6, 100, 101, 102, 103, 104, 105, 106, 107],
        },
        dkgInterval: {
          type: 'integer',
          minimum: 1,
        },
        activeSigners: {
          type: 'integer',
          minimum: 1,
        },
        rotation: {
          type: 'boolean',
        },
      },
      required: ['llmqType', 'dkgInterval', 'activeSigners', 'rotation'],
      additionalProperties: false,
    },
    quorumName: {
      type: 'string',
      enum: [
        'llmq_devnet',
        'llmq_devnet_dip0024',
        'llmq_devnet_platform',
        'llmq_50_60',
        'llmq_60_75',
        'llmq_400_60',
        'llmq_400_85',
        'llmq_100_67',
        'llmq_25_67',
      ],
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
          },
          additionalProperties: false,
          required: ['subnet'],
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
        insight: {
          type: 'object',
          properties: {
            enabled: {
              type: 'boolean',
            },
            ui: {
              type: 'object',
              properties: {
                enabled: {
                  type: 'boolean',
                },
                docker: {
                  $ref: '#/definitions/docker',
                },
              },
              required: ['enabled', 'docker'],
              additionalProperties: false,
            },
            docker: {
              $ref: '#/definitions/docker',
            },
            port: {
              $ref: '#/definitions/port',
            },
          },
          required: ['enabled', 'docker', 'port', 'ui'],
          additionalProperties: false,
        },
        docker: {
          type: 'object',
          properties: {
            image: {
              type: 'string',
              minLength: 1,
            },
            commandArgs: {
              type: 'array',
              items: {
                type: 'string',
                minLength: 1,
              },
            },
          },
          required: ['image', 'commandArgs'],
          additionalProperties: false,
        },
        p2p: {
          type: 'object',
          properties: {
            host: {
              $ref: '#/definitions/host',
            },
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
          required: ['host', 'port', 'seeds'],
          additionalProperties: false,
        },
        rpc: {
          type: 'object',
          properties: {
            host: {
              type: 'string',
              minLength: 1,
              format: 'ipv4',
            },
            port: {
              $ref: '#/definitions/port',
            },
            users: {
              type: 'object',
              minProperties: 1,
              propertyNames: {
                type: 'string',
                minLength: 1,
              },
              additionalProperties: {
                type: 'object',
                properties: {
                  password: {
                    type: 'string',
                    minLength: 1,
                  },
                  whitelist: {
                    type: ['null', 'array'],
                    items: {
                      type: 'string',
                      minLength: 1,
                    },
                    minItems: 1,
                  },
                  lowPriority: {
                    type: 'boolean',
                  },
                },
                required: ['password', 'whitelist', 'lowPriority'],
                additionalProperties: false,
              },
            },
            allowIps: {
              type: 'array',
              items: {
                type: 'string',
              },
            },
          },
          required: ['host', 'port', 'users', 'allowIps'],
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
              $ref: '#/definitions/duration',
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
            llmq: {
              type: 'object',
              properties: {
                chainLocks: {
                  $ref: '#/definitions/quorumName',
                },
                instantSend: {
                  $ref: '#/definitions/quorumName',
                },
                platform: {
                  $ref: '#/definitions/quorumName',
                },
                mnhf: {
                  $ref: '#/definitions/quorumName',
                },
              },
              required: ['chainLocks', 'instantSend', 'platform', 'mnhf'],
              additionalProperties: false,
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
        'logIps', 'indexes', 'insight'],
      additionalProperties: false,
    },
    platform: {
      type: 'object',
      properties: {
        gateway: {
          type: 'object',
          properties: {
            docker: {
              $ref: '#/definitions/docker',
            },
            maxConnections: {
              type: 'integer',
              minimum: 1,
              description: 'Maximum number of connections that Gateway accepts from downstream clients',
            },
            maxHeapSizeInBytes: {
              type: 'integer',
              minimum: 1,
              description: 'Maximum heap size in bytes. If the heap size exceeds this value, Gateway will take actions to reduce memory usage',
            },
            upstreams: {
              type: 'object',
              properties: {
                driveGrpc: {
                  $id: 'gatewayUpstream',
                  type: 'object',
                  properties: {
                    maxRequests: {
                      type: 'integer',
                      minimum: 1,
                      description: 'The maximum number of parallel requests',
                    },
                  },
                  required: ['maxRequests'],
                  additionalProperties: false,
                },
                dapiApi: {
                  $ref: 'gatewayUpstream',
                },
                dapiCoreStreams: {
                  $ref: 'gatewayUpstream',
                },
                dapiJsonRpc: {
                  $ref: 'gatewayUpstream',
                },
              },
              additionalProperties: false,
              required: ['driveGrpc', 'dapiApi', 'dapiCoreStreams', 'dapiJsonRpc'],
            },
            metrics: {
              $ref: '#/definitions/enabledHostPort',
            },
            admin: {
              $ref: '#/definitions/enabledHostPort',
            },
            listeners: {
              type: 'object',
              properties: {
                dapiAndDrive: {
                  type: 'object',
                  properties: {
                    http2: {
                      type: 'object',
                      properties: {
                        maxConcurrentStreams: {
                          type: 'integer',
                          minimum: 1,
                          description: 'Maximum number of concurrent streams allowed for each connection',
                        },
                      },
                      additionalProperties: false,
                      required: ['maxConcurrentStreams'],
                    },
                    host: {
                      type: 'string',
                      minLength: 1,
                      format: 'ipv4',
                    },
                    port: {
                      $ref: '#/definitions/port',
                    },
                  },
                  required: ['http2', 'host', 'port'],
                  additionalProperties: false,
                },
              },
              required: ['dapiAndDrive'],
              additionalProperties: false,
            },
            rateLimiter: {
              type: 'object',
              properties: {
                docker: {
                  $ref: '#/definitions/docker',
                },
                unit: {
                  type: 'string',
                  enum: ['second', 'minute', 'hour', 'day'],
                },
                requestsPerUnit: {
                  type: 'integer',
                  minimum: 1,
                },
                blacklist: {
                  type: 'array',
                  items: {
                    $ref: '#/definitions/host',
                  },
                  description: 'List of IP addresses that are blacklisted from making requests',
                },
                whitelist: {
                  type: 'array',
                  items: {
                    $ref: '#/definitions/host',
                  },
                  description: 'List of IP addresses that are whitelisted to make requests without limits',
                },
                metrics: {
                  type: 'object',
                  properties: {
                    docker: {
                      $ref: '#/definitions/docker',
                    },
                    enabled: {
                      type: 'boolean',
                    },
                    host: {
                      $ref: '#/definitions/host',
                    },
                    port: {
                      $ref: '#/definitions/port',
                    },
                  },
                  additionalProperties: false,
                  required: ['docker', 'enabled', 'host', 'port'],
                },
                enabled: {
                  type: 'boolean',
                },
              },
              required: ['docker', 'enabled', 'unit', 'requestsPerUnit', 'blacklist', 'whitelist', 'metrics'],
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
            log: {
              type: 'object',
              properties: {
                level: {
                  type: 'string',
                  enum: ['trace', 'debug', 'info', 'warn', 'error', 'critical', 'off'],
                },
                accessLogs: {
                  type: 'array',
                  items: {
                    oneOf: [
                      {
                        type: 'object',
                        properties: {
                          type: {
                            type: 'string',
                            minLength: 1,
                            enum: ['stdout', 'stderr'],
                            description: 'Access log type: stdout, stderr or file',
                          },
                          format: {
                            type: 'string',
                            enum: ['text', 'json'],
                          },
                          template: true,
                        },
                        required: ['type', 'format'],
                        additionalProperties: false,
                        if: {
                          type: 'object',
                          properties: {
                            format: {
                              const: 'json',
                            },
                          },
                        },
                        then: {
                          type: 'object',
                          properties: {
                            template: {
                              type: ['null', 'object'],
                              additionalProperties: {
                                type: 'string',
                              },
                              description: 'JSON fields and values. If null, default template is used.',
                            },
                          },
                          required: ['template'],
                        },
                        else: {
                          type: 'object',
                          properties: {
                            template: {
                              type: ['null', 'string'],
                              description: 'Template string. If null, default template is used.',
                            },
                          },
                          required: ['template'],
                        },
                      },
                      {
                        type: 'object',
                        properties: {
                          type: {
                            type: 'string',
                            const: 'file',
                            description: 'Access log type: stdout, stderr or file',
                          },
                          format: {
                            type: 'string',
                            enum: ['text', 'json'],
                          },
                          path: {
                            type: 'string',
                            minLength: 1,
                          },
                          template: true,
                        },
                        required: ['type', 'format', 'path'],
                        additionalProperties: false,
                        if: {
                          type: 'object',
                          properties: {
                            format: {
                              const: 'json',
                            },
                          },
                        },
                        then: {
                          type: 'object',
                          properties: {
                            template: {
                              type: ['null', 'object'],
                              additionalProperties: {
                                type: 'string',
                              },
                              description: 'JSON fields and values. If null, default template is used.',
                            },
                          },
                          required: ['template'],
                        },
                        else: {
                          type: 'object',
                          properties: {
                            template: {
                              type: ['null', 'string'],
                              description: 'Template string. If null, default template is used.',
                            },
                          },
                          required: ['template'],
                        },
                      },
                    ],
                  },
                },
              },
              additionalProperties: false,
              required: ['level', 'accessLogs'],
            },
          },
          required: ['docker', 'listeners', 'rateLimiter', 'ssl', 'maxHeapSizeInBytes', 'maxConnections', 'upstreams', 'metrics', 'admin', 'log'],
          additionalProperties: false,
        },
        dapi: {
          type: 'object',
          properties: {
            api: {
              type: 'object',
              properties: {
                docker: {
                  type: 'object',
                  properties: {
                    image: {
                      type: 'string',
                      minLength: 1,
                    },
                    deploy: {
                      type: 'object',
                      properties: {
                        replicas: {
                          type: 'integer',
                          minimum: 0,
                        },
                      },
                      additionalProperties: false,
                      required: ['replicas'],
                    },
                    build: {
                      $ref: '#/definitions/dockerBuild',
                    },
                  },
                  required: ['image', 'build', 'deploy'],
                  additionalProperties: false,
                },
              },
              required: ['docker'],
              additionalProperties: false,
            },
          },
          required: ['api'],
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
                logs: {
                  type: 'object',
                  propertyNames: {
                    type: 'string',
                    minLength: 1,
                    pattern: '[a-z0-9]',
                  },
                  additionalProperties: {
                    type: 'object',
                    properties: {
                      destination: {
                        type: 'string',
                        minLength: 1,
                        description: 'stdout, stderr or absolute path to log file',
                      },
                      level: {
                        type: 'string',
                        minLength: 1,
                        description: 'error, warn, info, debug, trace, silent or logging specification string in RUST_LOG format',
                      },
                      format: {
                        type: 'string',
                        enum: ['full', 'compact', 'pretty', 'json'],
                      },
                      color: {
                        type: ['boolean', 'null'],
                      },
                    },
                    required: ['destination', 'level', 'format', 'color'],
                    additionalProperties: false,
                  },
                },
                tokioConsole: {
                  type: 'object',
                  properties: {
                    enabled: {
                      type: 'boolean',
                    },
                    host: {
                      $ref: '#/definitions/host',
                    },
                    port: {
                      $ref: '#/definitions/port',
                    },
                    retention: {
                      type: 'integer',
                      minimum: 0,
                      description: 'How many seconds keep data if console is not connected',
                    },
                  },
                  required: ['enabled', 'host', 'port', 'retention'],
                  additionalProperties: false,
                },
                validatorSet: {
                  type: 'object',
                  properties: {
                    quorum: {
                      $ref: '#/definitions/quorum',
                    },
                  },
                  additionalProperties: false,
                  required: ['quorum'],
                },
                chainLock: {
                  type: 'object',
                  properties: {
                    quorum: {
                      $ref: '#/definitions/quorum',
                    },
                  },
                  additionalProperties: false,
                  required: ['quorum'],
                },
                instantLock: {
                  type: 'object',
                  properties: {
                    quorum: {
                      $ref: '#/definitions/quorum',
                    },
                  },
                  additionalProperties: false,
                  required: ['quorum'],
                },
                epochTime: {
                  type: 'integer',
                  minimum: 180,
                },
                metrics: {
                  $ref: '#/definitions/enabledHostPort',
                },
                grovedbVisualizer: {
                  $ref: '#/definitions/enabledHostPort',
                },
              },
              additionalProperties: false,
              required: ['docker', 'logs', 'tokioConsole', 'validatorSet', 'chainLock', 'epochTime', 'metrics', 'grovedbVisualizer'],
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
                    host: {
                      type: 'string',
                      minLength: 1,
                      format: 'ipv4',
                    },
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
                    flushThrottleTimeout: {
                      $ref: '#/definitions/duration',
                    },
                    maxPacketMsgPayloadSize: {
                      type: 'integer',
                      minimum: 0,
                    },
                    sendRate: {
                      type: 'integer',
                      minimum: 0,
                    },
                    recvRate: {
                      type: 'integer',
                      minimum: 0,
                    },
                  },
                  required: ['host', 'port', 'persistentPeers', 'seeds', 'flushThrottleTimeout', 'maxPacketMsgPayloadSize', 'sendRate', 'recvRate'],
                  additionalProperties: false,
                },
                mempool: {
                  type: 'object',
                  properties: {
                    cacheSize: {
                      type: 'integer',
                      minimum: 0,
                    },
                    size: {
                      type: 'integer',
                      minimum: 0,
                    },
                    maxTxsBytes: {
                      type: 'integer',
                      minimum: 0,
                    },
                    timeoutCheckTx: {
                      $ref: '#/definitions/duration',
                    },
                    txEnqueueTimeout: {
                      $ref: '#/definitions/duration',
                    },
                    txSendRateLimit: {
                      type: 'integer',
                      minimum: 0,
                    },
                    txRecvRateLimit: {
                      type: 'integer',
                      minimum: 0,
                    },
                    maxConcurrentCheckTx: {
                      type: 'integer',
                      minimum: 0,
                    },
                  },
                  additionalProperties: false,
                  required: ['size', 'maxTxsBytes', 'cacheSize', 'timeoutCheckTx', 'txEnqueueTimeout', 'txSendRateLimit', 'txRecvRateLimit', 'maxConcurrentCheckTx'],
                },
                consensus: {
                  type: 'object',
                  properties: {
                    createEmptyBlocks: {
                      type: 'boolean',
                    },
                    createEmptyBlocksInterval: {
                      $ref: '#/definitions/duration',
                    },
                    peer: {
                      type: 'object',
                      properties: {
                        gossipSleepDuration: {
                          $ref: '#/definitions/duration',
                        },
                        queryMaj23SleepDuration: {
                          $ref: '#/definitions/duration',
                        },
                      },
                      additionalProperties: false,
                      required: ['gossipSleepDuration', 'queryMaj23SleepDuration'],
                    },
                    unsafeOverride: {
                      type: 'object',
                      properties: {
                        propose: {
                          type: 'object',
                          properties: {
                            timeout: {
                              $ref: '#/definitions/optionalDuration',
                            },
                            delta: {
                              $ref: '#/definitions/optionalDuration',
                            },
                          },
                          additionalProperties: false,
                          required: ['timeout', 'delta'],
                        },
                        vote: {
                          type: 'object',
                          properties: {
                            timeout: {
                              $ref: '#/definitions/optionalDuration',
                            },
                            delta: {
                              $ref: '#/definitions/optionalDuration',
                            },
                          },
                          additionalProperties: false,
                          required: ['timeout', 'delta'],
                        },
                        commit: {
                          type: 'object',
                          properties: {
                            timeout: {
                              $ref: '#/definitions/optionalDuration',
                            },
                            bypass: {
                              type: ['boolean', 'null'],
                            },
                          },
                          additionalProperties: false,
                          required: ['timeout', 'bypass'],
                        },
                      },
                      additionalProperties: false,
                      required: ['propose', 'vote', 'commit'],
                    },
                  },
                  additionalProperties: false,
                  required: ['createEmptyBlocks', 'createEmptyBlocksInterval', 'peer', 'unsafeOverride'],
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
                    path: {
                      type: ['string', 'null'],
                      minLength: 1,
                    },
                  },
                  required: ['level', 'format'],
                  additionalProperties: false,
                },
                rpc: {
                  type: 'object',
                  properties: {
                    host: {
                      type: 'string',
                      minLength: 1,
                      format: 'ipv4',
                    },
                    port: {
                      $ref: '#/definitions/port',
                    },
                    maxOpenConnections: {
                      type: 'integer',
                      minimum: 0,
                    },
                    timeoutBroadcastTx: {
                      $ref: '#/definitions/duration',
                    },
                  },
                  required: ['host', 'port', 'maxOpenConnections', 'timeoutBroadcastTx'],
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
                  $ref: '#/definitions/enabledHostPort',
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
              required: ['mode', 'docker', 'p2p', 'mempool', 'consensus', 'log', 'rpc', 'pprof', 'node', 'moniker', 'genesis', 'metrics'],
              additionalProperties: false,
            },
          },
          required: ['abci', 'tenderdash'],
          additionalProperties: false,
        },
        sourcePath: {
          type: ['string', 'null'],
          minLength: 1,
        },
        enable: {
          type: 'boolean',
        },
      },
      required: ['gateway', 'dapi', 'drive', 'sourcePath', 'enable'],
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
      enum: Object.values(NETWORKS),
    },
    environment: {
      type: 'string',
      enum: ['development', 'production'],
    },
  },
  required: ['description', 'group', 'docker', 'core', 'externalIp', 'network', 'environment'],
  additionalProperties: false,
};
