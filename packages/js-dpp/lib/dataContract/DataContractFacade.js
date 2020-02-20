const $RefParser = require('json-schema-ref-parser');

const DataContractFactory = require('./DataContractFactory');
const validateDataContractFactory = require('./validateDataContractFactory');
const createDataContract = require('./createDataContract');
const enrichDataContractWithBaseDocument = require('./enrichDataContractWithBaseDocument');
const validateDataContractMaxDepthFactory = require('./stateTransition/validation/validateDataContractMaxDepthFactory');

class DataContractFacade {
  /**
   * @param {JsonSchemaValidator} jsonSchemaValidator
   */
  constructor(jsonSchemaValidator) {
    const validateDataContractMaxDepth = validateDataContractMaxDepthFactory($RefParser);

    this.validateDataContract = validateDataContractFactory(
      jsonSchemaValidator,
      validateDataContractMaxDepth,
      enrichDataContractWithBaseDocument,
      createDataContract,
    );

    this.factory = new DataContractFactory(
      createDataContract,
      this.validateDataContract,
    );
  }

  /**
   * Create Data Contract
   *
   * @param {string} contractId
   * @param {Object} documents
   * @return {DataContract}
   */
  create(contractId, documents) {
    return this.factory.create(contractId, documents);
  }

  /**
   * Create Data Contract from plain object
   *
   * @param {RawDataContract} rawDataContract
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<DataContract>}
   */
  async createFromObject(rawDataContract, options = { }) {
    return this.factory.createFromObject(rawDataContract, options);
  }

  /**
   * Create Data Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Promise<DataContract>}
   */
  async createFromSerialized(payload, options = { }) {
    return this.factory.createFromSerialized(payload, options);
  }

  /**
   * Create Data Contract State Transition
   *
   * @param {DataContract} dataContract
   * @return {DataContractStateTransition}
   */
  createStateTransition(dataContract) {
    return this.factory.createStateTransition(dataContract);
  }

  /**
   * Validate Data Contract
   *
   * @param {DataContract|RawDataContract} dataContract
   * @return {Promise<ValidationResult>}
   */
  async validate(dataContract) {
    return this.validateDataContract(dataContract);
  }
}

module.exports = DataContractFacade;
