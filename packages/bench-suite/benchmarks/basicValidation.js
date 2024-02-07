const { PrivateKey } = require('@dashevo/dashcore-lib');

const dpnsDocumentTypes = require('@dashevo/dpns-contract/schema/v1/dpns-contract-documents.json');

// const { DashPlatformProtocol, Identity, Identifier, default: loadWasmDpp }  = require('@dashevo/wasm-dpp');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const crypto = require('crypto');

const TYPES = require('../lib/benchmarks/types');
const createProperties = require('../lib/util/createProperties');

class StateRepository {
  /**
   * @type {DataContract}
   */
  #dataContract;

  /**
   * @returns {Promise<DataContract>}
   */
  async fetchDataContract() {
    return this.#dataContract;
  }

  /**
   * @param {DataContract} dataContract
   */
  setDataContract(dataContract) {
    this.#dataContract = dataContract;
  }
}

module.exports = {
  title: 'DPP validate basic',
  type: TYPES.FUNCTION,

  /**
   * How many times repeat tests
   *
   * @type {number}
   */
  repeats: 100,

  /**
   * Run before all tests
   *
   * @param {Context} context
   * @returns {Promise<void>}
   */
  async beforeAll(context) {
    context.stateRepository = new StateRepository();

    // TODO: rework with wasm-dpp if needed and if basic validation is available
    // await loadWasmDpp();
    // context.dpp = new DashPlatformProtocol({
    //   stateRepository: context.stateRepository,
    // });

    await context.dpp.initialize();

    context.privateKey = new PrivateKey();

    context.identity = new Identity({
      protocolVersion: 1,
      id: Identifier.from('8vAaZuDCm2p1dGEnVQXHiiBTx7uvQqvjmzsGayqKYeDY'),
      publicKeys: [
        {
          id: 0,
          type: 0,
          purpose: 0,
          securityLevel: 0,
          data: context.privateKey.publicKey.toBuffer(),
          readOnly: false,
        },
      ],
      balance: 0,
      revision: 0,
    });
  },

  /**
   * Multiple tests can be defined
   *
   * Each test will be run `repeats` times
   */
  tests: {
    'Validate DPNS domain document': {
      /**
       * Run once before running tests
       *
       * @param {Context} context
       */
      beforeAll(context) {
        context.dataContract = context.dpp.dataContract.create(
          context.identity.getId(),
          dpnsDocumentTypes,
        );

        context.stateRepository.setDataContract(context.dataContract);
      },

      /**
       * Run before each test run
       *
       * @param context
       * @returns {Promise<void>}
       */
      async beforeEach(context) {
        const label = crypto.randomBytes(10).toString('hex');

        const document = context.dpp.document.create(
          context.dataContract,
          context.identity.getId(),
          'domain',
          {
            label,
            normalizedLabel: label.toLowerCase(),
            normalizedParentDomainName: 'dash',
            preorderSalt: crypto.randomBytes(32),
            records: {
              dashUniqueIdentityId: generateRandomIdentifier(),
            },
            subdomainRules: {
              allowSubdomains: false,
            },
          },
        );

        const stateTransition = context.dpp.document.createStateTransition({
          create: [document],
        });

        await stateTransition.sign(
          context.identity.getPublicKeys()[0],
          context.privateKey,
        );

        context.stateTransition = stateTransition.toBuffer();
      },

      /**
       * Run test `repeats` times
       *
       * @param {Context} context
       * @returns {Promise<void>}
       */
      async test(context) {
        await context.dpp.stateTransition.createFromBuffer(
          context.stateTransition,
        );
      },
    },
    'Validate 100 strings': {
      /**
       * Run once before running tests
       *
       * @param {Context} context
       */
      beforeAll(context) {
        context.dataContract = context.dpp.dataContract.create(
          context.identity.getId(),
          {
            plain: {
              type: 'object',
              properties: createProperties(100, {
                type: 'string',
              }),
              additionalProperties: false,
            },
          },
        );

        context.stateRepository.setDataContract(context.dataContract);
      },

      /**
       * Run before each test run
       *
       * @param context
       * @returns {Promise<void>}
       */
      async beforeEach(context) {
        const properties = {};

        for (let i = 0; i < 100; i++) {
          const name = `property${i}`;

          properties[name] = crypto.randomBytes(20)
            .toString('hex');
        }

        const document = context.dpp.document.create(
          context.dataContract,
          context.identity.getId(),
          'plain',
          properties,
        );

        const stateTransition = context.dpp.document.createStateTransition({
          create: [document],
        });

        await stateTransition.sign(
          context.identity.getPublicKeys()[0],
          context.privateKey,
        );

        context.stateTransition = stateTransition.toBuffer();
      },

      /**
       * Run test `repeats` times
       *
       * @param {Context} context
       * @returns {Promise<void>}
       */
      async test(context) {
        await context.dpp.stateTransition.createFromBuffer(
          context.stateTransition,
        );
      },
    },
    'Validate 100 regexps': {
      /**
       * Run once before running tests
       *
       * @param {Context} context
       */
      beforeAll(context) {
        context.dataContract = context.dpp.dataContract.create(
          context.identity.getId(),
          {
            regexp: {
              type: 'object',
              properties: createProperties(100, {
                type: 'string',
                pattern: '^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$',
                maxLength: 63,
              }),
              additionalProperties: false,
            },
          },
        );

        context.stateRepository.setDataContract(context.dataContract);
      },

      /**
       * Run before each test run
       *
       * @param context
       * @returns {Promise<void>}
       */
      async beforeEach(context) {
        const properties = {};

        for (let i = 0; i < 100; i++) {
          const name = `property${i}`;

          properties[name] = crypto.randomBytes(20).toString('hex');
        }

        const document = context.dpp.document.create(
          context.dataContract,
          context.identity.getId(),
          'regexp',
          properties,
        );

        const stateTransition = context.dpp.document.createStateTransition({
          create: [document],
        });

        await stateTransition.sign(
          context.identity.getPublicKeys()[0],
          context.privateKey,
        );

        context.stateTransition = stateTransition.toBuffer();
      },

      /**
       * Run test `repeats` times
       *
       * @param {Context} context
       * @returns {Promise<void>}
       */
      async test(context) {
        await context.dpp.stateTransition.createFromBuffer(
          context.stateTransition,
        );
      },
    },
  },

  /**
   * Test timeout
   *
   * @type {number}
   */
  timeout: 3000,

  /**
   * Statistical function
   *
   * Available functions: https://mathjs.org/docs/reference/functions.html#statistics-functions
   *
   * @type {string}
   */
  avgFunction: 'median',

  /**
   * Show all or only statistic result
   *
   * @type {boolean}
   */
  avgOnly: false,
};
