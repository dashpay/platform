const { default: getRE2Class } = require('@dashevo/re2-wasm');
const createAjv = require('./ajv/createAjv');

const JsonSchemaValidator = require('./validation/JsonSchemaValidator');

const DataContractFacade = require('./dataContract/DataContractFacade');
const DocumentFacade = require('./document/DocumentFacade');
const StateTransitionFacade = require('./stateTransition/StateTransitionFacade');

const IdentityFacade = require('./identity/IdentityFacade');

/**
 * @class DashPlatformProtocol
 */
class DashPlatformProtocol {
  /**
   * @param {Object} options
   * @param {StateRepository} [options.stateRepository]
   * @param {JsonSchemaValidator} [options.jsonSchemaValidator]
   */
  constructor(options = {}) {
    this.options = options;

    this.stateRepository = undefined;
    this.jsonSchemaValidator = undefined;
    this.initialized = undefined;
  }

  /**
   * Initialize
   *
   * @return {Promise<boolean>}
   */
  async initialize() {
    if (this.initialized) {
      return this.initialized;
    }

    this.initialized = getRE2Class().then((RE2) => {
      this.stateRepository = this.options.stateRepository;

      this.jsonSchemaValidator = this.options.jsonSchemaValidator;
      if (this.jsonSchemaValidator === undefined) {
        const ajv = createAjv(RE2);

        this.jsonSchemaValidator = new JsonSchemaValidator(ajv);
      }

      this.dataContract = new DataContractFacade(
        this.jsonSchemaValidator,
        RE2,
      );

      this.document = new DocumentFacade(
        this.stateRepository,
        this.jsonSchemaValidator,
      );

      this.stateTransition = new StateTransitionFacade(
        this.stateRepository,
        this.jsonSchemaValidator,
        RE2,
      );

      this.identity = new IdentityFacade(
        this.jsonSchemaValidator,
      );

      return true;
    });

    return this.initialized;
  }

  /**
   * @return {JsonSchemaValidator}
   */
  getJsonSchemaValidator() {
    return this.jsonSchemaValidator;
  }

  /**
   * Get State Repository
   *
   * @return {StateRepository}
   */
  getStateRepository() {
    return this.stateRepository;
  }
}

module.exports = DashPlatformProtocol;
