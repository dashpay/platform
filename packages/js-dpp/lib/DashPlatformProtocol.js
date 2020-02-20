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
   */
  constructor(options = {}) {
    this.dataProvider = options.dataProvider;

    const jsonSchemaValidator = new JsonSchemaValidator(new Ajv());

    this.initializeFacades(
      jsonSchemaValidator,
    );
  }

  /**
   * @private
   * @param {JsonSchemaValidator} jsonSchemaValidator
   */
  initializeFacades(jsonSchemaValidator) {
    this.dataContract = new DataContractFacade(
      jsonSchemaValidator,
    );

    this.document = new DocumentFacade(this.dataProvider, jsonSchemaValidator);

    this.stateTransition = new StateTransitionFacade(this.dataProvider, jsonSchemaValidator);

    this.identity = new IdentityFacade(jsonSchemaValidator);
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
