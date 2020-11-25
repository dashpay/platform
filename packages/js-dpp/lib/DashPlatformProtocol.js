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
   * @param {boolean} [options.identities.skipAssetLockProofSignatureVerification=false]
   */
  constructor(options = {}) {
    this.stateRepository = options.stateRepository;

    this.jsonSchemaValidator = options.jsonSchemaValidator;
    if (this.jsonSchemaValidator === undefined) {
      const ajv = createAjv();

      this.jsonSchemaValidator = new JsonSchemaValidator(ajv);
    }

    const skipAssetLockProofSignatureVerification = options.identities
      ? options.identities.skipAssetLockProofSignatureVerification : false;

    this.initializeFacades(skipAssetLockProofSignatureVerification);
  }

  /**
   * @private
   * @param {boolean} skipAssetLockProofSignatureVerification
   */
  initializeFacades(skipAssetLockProofSignatureVerification) {
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
      skipAssetLockProofSignatureVerification,
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
