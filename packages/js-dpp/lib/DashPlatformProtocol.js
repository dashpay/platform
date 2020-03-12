const Ajv = require('ajv');

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
   * @param {DataProvider} [options.dataProvider]
   * @param {JsonSchemaValidator} [options.jsonSchemaValidator]
   */
  constructor(options = {}) {
    this.dataProvider = options.dataProvider;
    this.jsonSchemaValidator = options.jsonSchemaValidator;

    if (!this.jsonSchemaValidator) {
      this.jsonSchemaValidator = new JsonSchemaValidator(new Ajv());
    }

    this.initializeFacades();
  }

  /**
   * @private
   */
  initializeFacades() {
    this.dataContract = new DataContractFacade(
      this.jsonSchemaValidator,
    );

    this.document = new DocumentFacade(
      this.dataProvider,
      this.jsonSchemaValidator,
    );

    this.stateTransition = new StateTransitionFacade(
      this.dataProvider,
      this.jsonSchemaValidator,
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
   * Get Data Provider
   *
   * @return {DataProvider}
   */
  getDataProvider() {
    return this.dataProvider;
  }
}

module.exports = DashPlatformProtocol;
