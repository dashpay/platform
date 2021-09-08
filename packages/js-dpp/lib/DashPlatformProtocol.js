const { default: getRE2Class } = require('@dashevo/re2-wasm');
const createAjv = require('./ajv/createAjv');

const protocolVersion = require('./version/protocolVersion');

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
   * @param {number} [options.protocolVersion]
   */
  constructor(options = {}) {
    this.options = options;

    this.protocolVersion = this.options.protocolVersion !== undefined
      ? this.options.protocolVersion
      : protocolVersion.latestVersion;

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
        this,
        RE2,
      );

      this.document = new DocumentFacade(
        this,
      );

      this.stateTransition = new StateTransitionFacade(
        this,
        RE2,
      );

      this.identity = new IdentityFacade(
        this,
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

  /**
   * Get protocol version
   *
   * @return {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }

  /**
   * Set protocol version
   *
   * @param {number} version
   */
  setProtocolVersion(version) {
    this.protocolVersion = version;
  }
}

module.exports = DashPlatformProtocol;
