const DataContractFactory = require('./DataContractFactory');
const validateDataContractFactory = require('./validateDataContractFactory');
const createDataContract = require('./createDataContract');

class DataContractFacade {
  /**
   * @param {JsonSchemaValidator} validator
   */
  constructor(validator) {
    this.validateDataContract = validateDataContractFactory(validator);

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
  createFromObject(rawDataContract, options = { skipValidation: false }) {
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
  createFromSerialized(payload, options = { skipValidation: false }) {
    return this.factory.createFromSerialized(payload, options);
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
