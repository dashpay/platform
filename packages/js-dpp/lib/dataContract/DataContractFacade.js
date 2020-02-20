const DataContractFactory = require('./DataContractFactory');
const validateDataContractFactory = require('./validateDataContractFactory');
const createDataContract = require('./createDataContract');
const enrichDataContractWithBaseDocument = require('./enrichDataContractWithBaseDocument');

class DataContractFacade {
  /**
   * @param {JsonSchemaValidator} validator
   */
  constructor(validator) {
    this.validateDataContract = validateDataContractFactory(
      validator,
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
   * @return {DataContract}
   */
  createFromObject(rawDataContract, options = { }) {
    return this.factory.createFromObject(rawDataContract, options);
  }

  /**
   * Create Data Contract from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContract}
   */
  createFromSerialized(payload, options = { }) {
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
   * @return {ValidationResult}
   */
  validate(dataContract) {
    return this.validateDataContract(dataContract);
  }
}

module.exports = DataContractFacade;
