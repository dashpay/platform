const NETWORKS = require('../networks');

module.exports = {
  $schema: 'http://json-schema.org/draft-07/schema#',
  type: 'object',
  definitions: {
    dockerAndVersion: {
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
        version: {
          type: 'string',
        },
      },
      required: ['docker', 'version'],
      additionalProperties: false,
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
          type: 'object',
          properties: {
            image: {
              type: 'string',
            },
          },
          required: ['image'],
          additionalProperties: false,
        },
        version: {
          type: 'string',
        },
        p2p: {
          type: 'object',
          properties: {
            port: {
              type: 'integer',
              minimum: 0,
            },
          },
          required: ['port'],
          additionalProperties: false,
        },
        masternode: {
          type: 'object',
          properties: {
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
          required: ['operator'],
          additionalProperties: false,
        },
      },
      required: ['docker', 'version', 'p2p', 'masternode'],
      additionalProperties: false,
    },
    platform: {
      type: 'object',
      properties: {
        dapi: {
          type: 'object',
          properties: {
            envoy: {
              $ref: '#/definitions/dockerAndVersion',
            },
            nginx: {
              $ref: '#/definitions/dockerAndVersion',
            },
            api: {
              $ref: '#/definitions/dockerAndVersion',
            },
            insight: {
              $ref: '#/definitions/dockerAndVersion',
            },
          },
          required: ['envoy', 'nginx', 'api', 'insight'],
          additionalProperties: false,
        },
        drive: {
          type: 'object',
          properties: {
            mongodb: {
              $ref: '#/definitions/dockerAndVersion',
            },
            abci: {
              $ref: '#/definitions/dockerAndVersion',
            },
            tendermint: {
              $ref: '#/definitions/dockerAndVersion',
            },
          },
          required: ['mongodb', 'abci', 'tendermint'],
          additionalProperties: false,
        },
        dpns: {
          type: 'object',
          properties: {
            contractId: {
              type: ['string', 'null'],
            },
            ownerId: {
              type: ['string', 'null'],
            },
          },
          required: ['contractId', 'ownerId'],
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
  },
  required: ['description', 'core', 'platform', 'externalIp', 'network', 'compose'],
  additionalProperties: false,
};
