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
   * @param {StateRepository} [options.stateRepository]
   * @param {JsonSchemaValidator} [options.jsonSchemaValidator]
   * @param {boolean} [options.identities.enableAssetLockTxOneBlockConfirmationFallback=false]
   */
  constructor(options = {}) {
    this.stateRepository = options.stateRepository;

    this.jsonSchemaValidator = options.jsonSchemaValidator;
    if (this.jsonSchemaValidator === undefined) {
      this.jsonSchemaValidator = new JsonSchemaValidator(new Ajv());
    }

    const enableAssetLockTxOneBlockConfirmationFallback = options.identities
      ? options.identities.enableAssetLockTxOneBlockConfirmationFallback : false;

    this.initializeFacades(enableAssetLockTxOneBlockConfirmationFallback);
  }

  /**
   * @private
   * @param {boolean} [enableAssetLockTxOneBlockConfirmationFallback=false]
   */
  initializeFacades(enableAssetLockTxOneBlockConfirmationFallback = false) {
    this.dataContract = new DataContractFacade(
      this.jsonSchemaValidator,
    );

    this.document = new DocumentFacade(
      this.stateRepository,
      this.jsonSchemaValidator,
    );

    this.stateTransition = new StateTransitionFacade(
      this.stateRepository,
      this.jsonSchemaValidator,
      enableAssetLockTxOneBlockConfirmationFallback,
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
