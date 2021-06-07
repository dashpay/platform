const getRE2Class = require('@dashevo/re2-wasm').default;
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
    this.stateRepository = options.stateRepository;

    this.jsonSchemaValidator = options.jsonSchemaValidator;
  }

  async initialize() {
    if (this.jsonSchemaValidator === undefined) {
      const ajv = await createAjv();

      this.jsonSchemaValidator = new JsonSchemaValidator(ajv);
    }

    const RE2 = await getRE2Class();
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
